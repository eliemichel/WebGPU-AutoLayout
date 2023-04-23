use naga::{
    front::wgsl::Frontend,
    TypeInner,
    TypeInner::{Struct},
    Type,
    StructMember,
    Module,
    Handle,
    ScalarKind,
    VectorSize,
};

use std::{
    collections::{LinkedList},
    cmp::max,
};

struct Context<'a> {
    out: LinkedList<String>,
    module: &'a Module,
}
impl<'a> Context<'a> {
    fn from_module(module: &'a Module) -> Self {
        return Self {
            module: module,
            out: LinkedList::new()
        };
    }
}

struct CppType {
    name: Option<String>, // anonymous is used for padding
    size: u32,
}

trait ToCpp {
    fn to_cpp(&self) -> String;
}
impl ToCpp for ScalarKind {
    fn to_cpp(&self) -> String {
        return match self {
            ScalarKind::Sint => "int32_t",
            ScalarKind::Uint => "uint32_t",
            ScalarKind::Float => "float",
            ScalarKind::Bool => "bool",
        }.to_string()
    }
}

trait ToU32 {
    fn to_u32(&self) -> u32;
}
impl ToU32 for VectorSize {
    fn to_u32(&self) -> u32 {
        return match self {
            VectorSize::Bi => 2,
            VectorSize::Tri => 3,
            VectorSize::Quad => 4,
        }
    }
}

// https://gpuweb.github.io/gpuweb/wgsl/#alignment-and-size
fn align_of(inner: &TypeInner, ctx: &Context) -> u32 {
    match inner {
        TypeInner::Scalar{width, ..} => (*width).into(),
        TypeInner::Vector{width, size, ..} => {
            let s = match size {
                VectorSize::Bi => 2,
                VectorSize::Tri => 4, // yep
                VectorSize::Quad => 4,
            };
            (s * width).into()
        },
        TypeInner::Matrix{width, rows, ..} => {
            let col_type = TypeInner::Vector{width: *width, size: *rows, kind: ScalarKind::Float};
            align_of(&col_type, ctx)
        },
        TypeInner::Atomic{..} => 4,
        TypeInner::Array{base, ..} => {
            let base_inner = &ctx.module.types[*base].inner;
            align_of(base_inner, ctx)
        },
        TypeInner::Struct{members, ..} => {
            let mut maximum = 0;
            for m in members {
                let ty_inner = &ctx.module.types[m.ty].inner;
                maximum = max(maximum, align_of(ty_inner, ctx));
            }
            maximum
        },
        _ => 0,//panic!("Type is not host-sharable!")
    }
}

trait ToCppType {
    fn to_cpp(&self, ctx: &Context) -> Vec<CppType>;
}
impl ToCppType for Type {
    fn to_cpp(&self, ctx: &Context) -> Vec<CppType> {
        return match self.inner {
            TypeInner::Scalar{kind, width} => vec![CppType {
                name: Some(format!("{}", kind.to_cpp())),
                size: width as u32,
            }],
            TypeInner::Vector{size, kind, width} => vec![CppType {
                name: Some(format!("vec{}<{}>", size.to_u32(), kind.to_cpp())),
                size: size.to_u32() * width as u32,
            }],
            TypeInner::Matrix{columns, rows, width} => {
                let c = columns.to_u32();
                let r = rows.to_u32();
                let col_type = TypeInner::Vector{width, size: rows, kind: ScalarKind::Float};
                let align = align_of(&col_type, ctx);
                let cpp_col_size = width as u32 * r;
                if align == cpp_col_size {
                    if columns == rows {
                        vec![CppType {
                            name: Some(format!("mat{}", c)),
                            size: c * r * width as u32,
                        }]
                    } else {
                        (0..c).map(|_| CppType {
                            name: Some(format!("vec{}", r)),
                            size: r * width as u32,
                        }).collect()
                    }
                } else {
                    (0..2*c).map(|i| match i % 2 {
                        0 => CppType {
                            name: Some(format!("vec{}", r)),
                            size: r * width as u32,
                        },
                        _ => CppType {
                            name: None, // padding
                            size: align - cpp_col_size,
                        },
                    }).collect()
                }
            },
            TypeInner::Atomic{kind, width} => vec![CppType {
                name: Some(format!("atomic<{}>", kind.to_cpp())),
                size: width as u32,
            }],
            //TypeInner::Array{base, size, stride} => format!("array<{}, {}:{}>", base, size, stride),
            // Array (of host-sharable element)
            // Struct (of host-sharable element)
            _ => vec![CppType {
                name: Some(format!("[Error: Type is not host-sharable!]")),
                size: 0,
            }],
        }
    }
}

pub fn generate_cpp_binding(wgsl_source: &str) -> String {
    let mut frontend = Frontend::new();
    let module = match  frontend.parse(wgsl_source) {
        Ok(module) => module,
        Err(e) => match e.location(wgsl_source) {
            Some(loc) => { return format!("Error at line {}, col {}: {}", loc.line_number, loc.line_position, e) },
            None => { return format!("Error: {}", e) },
        },
    };

    let mut ctx = Context::from_module(&module);

    ctx.out.push_back("// C++ binding".to_string());
    ctx.out.push_back("#include <glm/glm.hpp>".to_string());
    ctx.out.push_back("using namespace glm;".to_string());
    ctx.out.push_back("".to_string());

    ctx.out.push_back("// Host-sharable structures".to_string());
    ctx.out.push_back("".to_string());
    for entry in module.types.iter() {
        generate_cpp_type_def(&mut ctx, &entry);
    }

    ctx.out.push_back("// Bind Group Layouts".to_string());
    ctx.out.push_back("".to_string());

    ctx.out.push_back("// Bind Groups".to_string());
    ctx.out.push_back("".to_string());

    return ctx.out.into_iter().collect::<Vec<_>>().join("\n");
}

fn generate_cpp_type_def(ctx: &mut Context, entry: &(Handle<Type>, &Type)) {
    let (_handle, type_def) = entry;
    let anonymous: String = "<anonymous>".to_string();
    let name = match &type_def.name {
        Some(name) => name,
        None => &anonymous
    };
    
    if let Struct{members, span: _} = &type_def.inner {
        ctx.out.push_back(format!("struct {} {{", name));
        generate_cpp_struct_def(ctx, members);
        ctx.out.push_back("};".to_string());
        ctx.out.push_back(format!("static_assert(sizeof({}) % 16 == 0)\n", name));
    }
}

fn format_pad(byte_size: u32, pad_count: &mut u32) -> String {
    assert_eq!(byte_size % 4, 0);
    *pad_count += 1;
    match byte_size / 4 {
        1 => format!("  float _pad{};", *pad_count - 1),
        p => format!("  float _pad{}[{}];", *pad_count - 1, p),
    }
}

fn generate_cpp_struct_def(ctx: &mut Context, members: &Vec<StructMember>) {
    let anonymous: String = "<anonymous>".to_string();
    let mut cpp_offset = 0;
    let mut pad_count = 0;
    for m in members {
        let name = match &m.name {
            Some(name) => name,
            None => &anonymous
        };
        let ty = &ctx.module.types[m.ty];
        let type_size = ty.inner.size(&ctx.module.constants);

        assert!(cpp_offset <= m.offset);
        if cpp_offset < m.offset {
            ctx.out.push_back(format_pad(m.offset - cpp_offset, &mut pad_count));
        }

        let all_cpp_types = ty.to_cpp(ctx);
        let has_multiple_fields = all_cpp_types.len() > 1;

        if has_multiple_fields {
            ctx.out.push_back(
                format!("\n  // '{}' is split in {}, at byte offset {}, size {}",
                    name, all_cpp_types.len(), m.offset, type_size
                )
            );
        }

        let mut sub_field_index = 0;
        for cpp_type in all_cpp_types {
            ctx.out.push_back(match cpp_type.name {
                Some(cpp_type_name) => match has_multiple_fields {
                    true => format!("  {} {}_col{};",
                        cpp_type_name, name, sub_field_index
                    ),
                    false => format!("  {} {}; // at byte offset {}, size {}",
                        cpp_type_name, name, m.offset, type_size
                    )
                },
                None => {
                    sub_field_index -= 1;
                    format_pad(cpp_type.size, &mut pad_count)
                },
            });

            cpp_offset += cpp_type.size;
            sub_field_index += 1;
        }

        if has_multiple_fields {
            ctx.out.push_back("".to_string());
        }
    }

    let aligned_cpp_offset = (cpp_offset + 15) & !15;
    if cpp_offset < aligned_cpp_offset {
        ctx.out.push_back(format_pad(aligned_cpp_offset - cpp_offset, &mut pad_count));
    }
}
