use naga::{
    front::wgsl::Frontend,
    TypeInner::{Struct},
    Type,
    StructMember,
    Module,
    Handle,
};

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
