/**
 * WARNING: This code is messy, i.e., it works but is far from idiomatic.
 * It mostly comes from a 1-day hackathon by somebody who is not 100% fluent
 * in rust. Any suggestion for organizing it better is welcome!
 */

use naga::{
    front::wgsl::Frontend,
    TypeInner::{
        Struct, Array, Scalar, Vector, Matrix, Atomic, Image, Sampler
    },
    TypeInner,
    ConstantInner,
    ArraySize::{Constant, Dynamic},
    Type,
    GlobalVariable,
    StructMember,
    Module,
    Handle,
    ScalarValue,
    ScalarKind,
    ScalarKind::{Float, Uint, Sint, Bool},
    VectorSize,
    ImageClass,
};

use std::{
    collections::{LinkedList, HashMap},
    cmp::max,
};

// Short for outputing lines of C++
macro_rules! out {
    ($ctx:ident, $t:expr) => ($ctx.get_out().push_back(($t).into()))
}
macro_rules! out_format {
    ($ctx:ident, $($t:tt)*) => ($ctx.get_out().push_back(format!($($t)*)))
}

enum ContextStage {
    Structs,
    PaddedStructs,
    Bindings,
    Final,
}
struct Context<'a> {
    module: &'a Module,

    // Output sections
    out_structs: LinkedList<String>,
    out_padded_structs: LinkedList<String>,
    out_bindings: LinkedList<String>,
    out_final: LinkedList<String>,

    stage: ContextStage,

    // Set of structs for which we need to define a C++ equivalent with correct padding
    // For each such type, we save its name.
    // NB: This is populated while filling out_structs.
    padded_structs: HashMap<Handle<Type>, String>,

    bind_group_layout_reg: HashMap<u32, BindGroupLayout>,
}
impl<'a> Context<'a> {
    fn from_module(module: &'a Module) -> Self {
        return Self {
            module: module,
            out_structs: LinkedList::new(),
            out_padded_structs: LinkedList::new(),
            out_bindings: LinkedList::new(),
            out_final: LinkedList::new(),
            stage: ContextStage::Structs,
            padded_structs: HashMap::new(),
            bind_group_layout_reg: HashMap::new(),
        }
    }

    fn get_out(&mut self) -> &mut LinkedList<String> {
        return match self.stage {
            ContextStage::Structs => &mut self.out_structs,
            ContextStage::PaddedStructs => &mut self.out_padded_structs,
            ContextStage::Bindings => &mut self.out_bindings,
            ContextStage::Final => &mut self.out_final,
        };
    }
}

#[derive(Clone)]
struct BindGroupLayoutEntry {
    name: String,
    config: LinkedList<String>,
    config_layout: LinkedList<String>,
}
impl BindGroupLayoutEntry {
    fn new(name: &str) -> Self {
        return Self {
            name: name.to_string(),
            config: LinkedList::new(),
            config_layout: LinkedList::new(),
        }
    }
}
#[derive(Clone)]
struct BindGroupLayout {
    entries: HashMap<u32, BindGroupLayoutEntry>,
}
impl BindGroupLayout {
    fn new() -> Self {
        return Self {
            entries: HashMap::new(),
        }
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
            Sint => "int32_t",
            Uint => "uint32_t",
            Float => "float",
            Bool => "bool",
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
        Scalar{width, ..} => (*width).into(),
        Vector{width, size, ..} => {
            let s = match size {
                VectorSize::Bi => 2,
                VectorSize::Tri => 4, // yep
                VectorSize::Quad => 4,
            };
            (s * width).into()
        },
        Matrix{width, rows, ..} => {
            let col_type = Vector{width: *width, size: *rows, kind: Float};
            align_of(&col_type, ctx)
        },
        Atomic{..} => 4,
        Array{base, ..} => {
            let base_inner = &ctx.module.types[*base].inner;
            align_of(base_inner, ctx)
        },
        Struct{members, ..} => {
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

    // Host-sharable structs
    ctx.stage = ContextStage::Structs;
    for entry in module.types.iter() {
        generate_cpp_type_def(&mut ctx, &entry.1, 16);
    }

    // Extra structs needed to build host-sharable structs
    ctx.stage = ContextStage::PaddedStructs;
    for entry in ctx.padded_structs.clone().iter() {
        let value_ty = &ctx.module.types[*entry.0];
        let ty = Type {
            name: Some(entry.1.clone()),
            inner: TypeInner::Struct {
                members: vec![StructMember{
                    name: Some("value".to_string()),
                    ty: *entry.0,
                    binding: None,
                    offset: 0,
                }],
                span: value_ty.inner.size(&ctx.module.constants),
            },
        };
        let padding = align_of(&value_ty.inner, &ctx);
        generate_cpp_type_def(&mut ctx, &ty, padding);
    }

    // Bindings
    ctx.stage = ContextStage::Bindings;
    for entry in module.global_variables.iter() {
        process_bind_layout(&mut ctx, &entry.1);
    }
    if ctx.bind_group_layout_reg.len() > 0 {
        generate_cpp_bind_layouts(&mut ctx);
        generate_cpp_bind_groups(&mut ctx);
    }

    // Final code production
    ctx.stage = ContextStage::Final;

    out!(ctx, "#include <glm/glm.hpp> // from https://github.com/g-truc/glm");
    out!(ctx, "using namespace glm;");
    if ctx.out_bindings.len() > 0 {
        out!(ctx, "#include <webgpu/webgpu.hpp> // from https://github.com/eliemichel/WebGPU-Cpp");
        out!(ctx, "using namespace wgpu;");
        out!(ctx, "#include <vector>");
    }
    out!(ctx, "");

    if ctx.out_padded_structs.len() > 0 {
        out!(ctx, "// Padded structures");
        out!(ctx, "");
        ctx.out_final.append(&mut ctx.out_padded_structs);
    }

    if ctx.out_structs.len() > 0 {
        out!(ctx, "// Host-sharable structures");
        out!(ctx, "");
        ctx.out_final.append(&mut ctx.out_structs);
    }

    if ctx.out_bindings.len() > 0 {
        out!(ctx, "// Bind Group Layouts");
        out!(ctx, "");
        ctx.out_final.append(&mut ctx.out_bindings);
    }

    return ctx.out_final.into_iter().collect::<Vec<_>>().join("\n");
}

fn process_bind_layout(ctx: &mut Context, variable: &GlobalVariable) {
    let anonymous: String = "<anonymous>".to_string();
    let name = match &variable.name {
        Some(name) => name,
        None => &anonymous
    };

    match &variable.binding {
        Some(binding) => {
            let bind_group_layout = ctx.bind_group_layout_reg.entry(binding.group).or_insert(BindGroupLayout::new());
            let entry = bind_group_layout.entries.entry(binding.binding).or_insert(BindGroupLayoutEntry::new(name));
            match &ctx.module.types[variable.ty].inner {
                Image{class: ImageClass::Storage{format, access}, ..} => {
                    entry.config_layout.push_back("storageTexture.access = StorageTextureAccess::WriteOnly;".into());
                    entry.config_layout.push_back("storageTexture.format = TextureFormat::RGBA8Unorm;".into());
                    entry.config_layout.push_back("storageTexture.viewDimension = TextureViewDimension::_2D;".into());

                    entry.config.push_back("textureView = nullptr; // EDIT HERE".into());
                }
                Image{class: ImageClass::Depth{multi}, ..} => {
                    entry.config_layout.push_back("texture.sampleType = TextureSampleType::Float;".into());
                    entry.config_layout.push_back("texture.viewDimension = TextureViewDimension::_2D;".into());

                    entry.config.push_back("textureView = nullptr; // EDIT HERE".into());
                }
                Image{class: ImageClass::Sampled{kind, multi}, ..} => {
                    entry.config_layout.push_back("texture.sampleType = TextureSampleType::Float;".into());
                    entry.config_layout.push_back("texture.viewDimension = TextureViewDimension::_2D;".into());

                    entry.config.push_back("textureView = nullptr; // EDIT HERE".into());
                }
                Sampler{..} => {
                    entry.config_layout.push_back("sampler.type = SamplerBindingType::Filtering;".into());

                    entry.config.push_back("sampler = nullptr; // EDIT HERE".into());
                }
                _ => {
                    entry.config_layout.push_back("buffer.type = BufferBindingType::Uniform;".into());
                    entry.config_layout.push_back("buffer.minBindingSize = sizeof(Uniforms);".into());

                    entry.config.push_back("buffer = nullptr; // EDIT HERE".into());
                    entry.config.push_back("offset = 0;".into());
                    entry.config.push_back("size = sizeof(???);".into());
                }
            };
        },
        None => (),
    };
}

fn generate_cpp_bind_layouts(ctx: &mut Context) {
    let reg = ctx.bind_group_layout_reg.clone();
    out!(ctx, "std::vector<BindGroupLayout> initBindGroupLayouts(Device device) {");
    out_format!(ctx, "  std::vector<BindGroupLayout> bindGroupLayouts({}, nullptr);", reg.len());
    out!(ctx, "");
    let mut group_idx = 0;
    for pair in reg.iter() {
        let bind_group = pair.0;
        let bind_group_layout = pair.1;
        out_format!(ctx, "  {{ // bind group {bind_group}");
        out_format!(ctx, "    std::vector<BindGroupLayoutEntry> entries({}, Default);", bind_group_layout.entries.len());
        out!(ctx, "");
        let mut idx = 0;
        for subpair in bind_group_layout.entries.iter() {
            let binding = subpair.0;
            let entry = subpair.1;
            out_format!(ctx, "    // Binding '{}'", entry.name);
            out_format!(ctx, "    entries[{idx}].binding = {binding};");
            for line in entry.config_layout.iter() {
                out_format!(ctx, "    entries[{idx}].{line}");
            }
            out_format!(ctx, "    entries[{idx}].visibility = ShaderStage::Compute; // EDIT HERE");
            idx += 1;
            out!(ctx, "");
        }
        
        out!(ctx, "    BindGroupLayoutDescriptor bindGroupLayoutDesc;");
        out!(ctx, "    bindGroupLayoutDesc.entryCount = (uint32_t)entries.size();");
        out!(ctx, "    bindGroupLayoutDesc.entries = entries.data();");
        out_format!(ctx, "    bindGroupLayouts[{group_idx}] = device.createBindGroupLayout(bindGroupLayoutDesc);");
        out!(ctx, "  }\n");
        group_idx += 1;
    }
    out!(ctx, "  return bindGroupLayouts;");
    out!(ctx, "}");
    out!(ctx, "");
}

fn generate_cpp_bind_groups(ctx: &mut Context) {
    let reg = ctx.bind_group_layout_reg.clone();
    out!(ctx, "std::vector<BindGroup> initBindGroups(Device device, std::vector<BindGroupLayout> bindGroupLayouts) {");
    out_format!(ctx, "  std::vector<BindGroup> bindGroups({}, nullptr);", reg.len());
    out!(ctx, "");
    let mut group_idx = 0;
    for pair in reg.iter() {
        let bind_group = pair.0;
        let bind_group_layout = pair.1;
        out_format!(ctx, "  {{ // bind group {bind_group}");
        out_format!(ctx, "    std::vector<BindGroupEntry> entries({}, Default);", bind_group_layout.entries.len());
        out!(ctx, "");
        let mut entry_idx = 0;
        for subpair in bind_group_layout.entries.iter() {
            let binding = subpair.0;
            let entry = subpair.1;
            out_format!(ctx, "    // Binding '{}'", entry.name);
            out_format!(ctx, "    entries[{entry_idx}].binding = {binding};");
            for line in entry.config.iter() {
                out_format!(ctx, "    entries[{entry_idx}].{line}");
            }
            entry_idx += 1;
            out!(ctx, "");
        }

        out!(ctx, "    BindGroupDescriptor bindGroupDesc;");
        out_format!(ctx, "    bindGroupDesc.layout = bindGroupLayouts[{group_idx}];");
        out!(ctx, "    bindGroupDesc.entryCount = (uint32_t)entries.size();");
        out!(ctx, "    bindGroupDesc.entries = (WGPUBindGroupEntry*)entries.data();");
        out_format!(ctx, "    bindGroups[{group_idx}] = device.createBindGroup(bindGroupDesc);");
        out!(ctx, "  }\n");
        group_idx += 1;
    }
    out!(ctx, "  return bindGroups;");
    out!(ctx, "}");
}

fn generate_cpp_type_def(ctx: &mut Context, ty: &Type, padding: u32) {
    let anonymous: String = "<anonymous>".to_string();
    let name = match &ty.name {
        Some(name) => name,
        None => &anonymous
    };

    if let Struct{members, span: _} = &ty.inner {
        out_format!(ctx, "struct {} {{", name);
        generate_cpp_struct_def(ctx, members, padding);
        out!(ctx, "};");
        //out_format!(ctx, "static_assert(sizeof({}) % {} == 0);\n", name, padding);
    }
}

/**
 * When using an array<T, N> whose base type T is not representable without
 * extra padding, we create a new struct to represent T.
 */
fn add_extra_struct(ty: Handle<Type>, ctx: &mut Context) -> String {
    match &ctx.module.types[ty].inner {
        Matrix{columns, rows, ..} => {
            let c = columns.to_u32();
            let r = rows.to_u32();
            let name = format!("padded_mat{c}x{r}");
            ctx.padded_structs.insert(ty, name.clone());
            name
        },
        Vector{size: VectorSize::Tri, ..} => {
            let name = format!("padded_vec3");
            ctx.padded_structs.insert(ty, name.clone());
            name
        },
        _ => { panic!("add_extra_struct() must only be called on a type for which generate_cpp_fields() returns more than one field.") },
    }
}

fn generate_cpp_fields(ty: &Type, ctx: &mut Context) -> Vec<CppType> {
    match ty.inner {
        Scalar{kind, width} => vec![CppType {
            name: Some(format!("{}", kind.to_cpp())),
            size: width as u32,
        }],
        Vector{size, kind, width} => {
            let prefix = match kind {
                Sint => "i",
                Uint => "u",
                Float => "",
                Bool => "d",
            };
            vec![CppType {
                name: Some(format!("{}vec{}", prefix, size.to_u32())),
                size: size.to_u32() * width as u32,
            }]
        },
        Matrix{columns, rows, width} => {
            let c = columns.to_u32();
            let r = rows.to_u32();
            let col_type = Vector{width, size: rows, kind: Float};
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
        Atomic{kind, width} => vec![CppType {
            name: Some(kind.to_cpp()),
            size: width as u32,
        }],
        Array{base, size, stride} => {
            let base_ty = &ctx.module.types[base];
            let cpp_fields = generate_cpp_fields(base_ty, ctx);
            let base_name = match cpp_fields.len() {
                1 => {
                    if cpp_fields[0].size % align_of(&base_ty.inner, &ctx) == 0 {
                        match &cpp_fields[0].name {
                            Some(name) => name.clone(),
                            None => { panic!("There should not be an anonymous type here.") },
                        }
                    } else {
                        add_extra_struct(base, ctx)
                    }
                },
                _ => add_extra_struct(base, ctx)
            };
            match size {
                Constant(cst) => match ctx.module.constants[cst].inner {
                    ConstantInner::Scalar{value: ScalarValue::Uint(s), ..} => vec![CppType {
                        name: Some(format!("std::array<{}, {}>", base_name, s)),
                        size: s as u32 * stride as u32,
                    }],
                    ConstantInner::Scalar{value: ScalarValue::Sint(s), ..} => vec![CppType {
                        name: Some(format!("std::array<{}, {}>", base_name, s)),
                        size: s as u32 * stride as u32,
                    }],
                    _ => { panic!("An array size must be an int") },
                },
                Dynamic => vec![CppType {
                    name: Some(format!("std::vector<{}>", base_name)),
                    size: 0, // supposed to be the last field anyways
                }],
            }
        },
        Struct{span, ..} => vec![CppType {
            name: ty.name.clone(),
            size: span,
        }],
        _ => vec![CppType {
            name: Some(format!("[Error: Type is not host-sharable!]")),
            size: 0,
        }],
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

fn generate_cpp_struct_def(ctx: &mut Context, members: &Vec<StructMember>, padding: u32) {
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
            out!(ctx, format_pad(m.offset - cpp_offset, &mut pad_count));
            cpp_offset = m.offset;
        }

        // Transform a WGSL type into one or multiple C++ types
        let cpp_fields = generate_cpp_fields(ty, ctx);
        let has_multiple_fields = cpp_fields.len() > 1;

        if has_multiple_fields {
            out_format!(ctx,
                "\n  // '{}' is split in {}, at byte offset {}, size {}",
                name, cpp_fields.len(), m.offset, type_size
            );
        }

        let mut sub_field_index = 0;
        for cpp_type in cpp_fields {
            out!(ctx, match cpp_type.name {
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
            out!(ctx, "");
        }
    }

    let p = padding - 1;
    let aligned_cpp_offset = (cpp_offset + p) & !p;
    if cpp_offset < aligned_cpp_offset {
        out!(ctx, format_pad(aligned_cpp_offset - cpp_offset, &mut pad_count));
    }
}
