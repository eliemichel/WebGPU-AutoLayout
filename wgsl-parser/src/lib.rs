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

use std::collections::LinkedList;

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

pub trait ToCpp {
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
impl ToCpp for VectorSize {
    fn to_cpp(&self) -> String {
        return match self {
            VectorSize::Bi => "2",
            VectorSize::Tri => "3",
            VectorSize::Quad => "4",
        }.to_string()
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

    inspect_module(&module);
    let mut ctx = Context::from_module(&module);

    for entry in module.types.iter() {
        generate_cpp_type_def(&mut ctx, &entry);
    }

    return ctx.out.into_iter().collect::<Vec<_>>().join("\n");
}

fn generate_cpp_type_def(ctx: &mut Context, entry: &(Handle<Type>, &Type)) {
    let (handle, type_def) = entry;
    let anonymous: String = "<anonymous>".to_string();
    let name = match &type_def.name {
        Some(name) => name,
        None => &anonymous
    };
    println!(" - {}: {}", handle.index(), name);
    if let Struct{members, span: _} = &type_def.inner {
        ctx.out.push_back(format!("struct {} {{", name));
        generate_cpp_struct_def(ctx, members);
        ctx.out.push_back("};\n".to_string());
    }
}

fn generate_cpp_struct_def(ctx: &mut Context, members: &Vec<StructMember>) {
    let anonymous: String = "<anonymous>".to_string();
    for m in members {
        let name = match &m.name {
            Some(name) => name,
            None => &anonymous
        };
        let ty = &ctx.module.types[m.ty];
        let cpp_type = match ty.inner {
            TypeInner::Scalar{kind, width} => format!("{}<width:{}>", kind.to_cpp(), width),
            TypeInner::Vector{size, kind, width} => format!("vec{}<{},width:{}>", size.to_cpp(), kind.to_cpp(), width),
            TypeInner::Matrix{columns, rows, width} => format!("mat{}x{}<{}>", columns.to_cpp(), rows.to_cpp(), width),
            TypeInner::Atomic{kind, width} => format!("atomic<{},{}>", kind.to_cpp(), width),
            TypeInner::Pointer{..} => format!("void*"),
            TypeInner::ValuePointer{..} => format!("<value pointers are not suported>"),
            _ => format!("<unsuported type>"),
            //TypeInner::Array{base, size, stride} => format!("array<{}, {}:{}>", base, size, stride),
        };
        ctx.out.push_back(format!("  {} {}; // at byte offset {}", cpp_type, name, m.offset));
    }
}

// DEBUG

pub fn inspect_from_source(wgsl_source: &str) {
    let mut frontend = Frontend::new();
    let module = match  frontend.parse(wgsl_source) {
        Ok(module) => module,
        Err(e) => {panic!("Parse Error: {}", e)},
    };
    inspect_module(&module);
}

fn inspect_module(module: &Module) {
    println!("== Module ==");
    println!("global_variables count: {}", module.global_variables.len());
    println!("types (count: {})", module.types.len());

    for entry in module.types.iter() {
        inspect_type_entry(&entry);
    }
}

fn inspect_type_entry(entry: &(Handle<Type>, &Type)) {
    let (handle, type_def) = entry;
    let anonymous: String = "<anonymous>".to_string();
    let name = match &type_def.name {
        Some(name) => name,
        None => &anonymous
    };
    println!(" - {}: {}", handle.index(), name);
    if let Struct{members, span} = &type_def.inner {
        inspect_struct(members, span);
    }
}

fn inspect_struct(members: &Vec<StructMember>, span: &u32) {
    println!("   Struct ({} members, span: {})", members.len(), span);
    let anonymous: String = "<anonymous>".to_string();
    for m in members {
        let name = match &m.name {
            Some(name) => name,
            None => &anonymous
        };
        println!("    - At offset {}: {}", m.offset, name);
    }
}
