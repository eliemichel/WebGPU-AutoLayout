const SHADER: &str = r#"
struct Uniforms {
    kernel: mat3x3<f32>,
    test: f32,
}

@group(0) @binding(0) var inputTexture: texture_2d<f32>;
@group(0) @binding(1) var outputTexture: texture_storage_2d<rgba8unorm,write>;
@group(0) @binding(2) var<uniform> uniforms: Uniforms;
"#;

use naga::{
    front::wgsl::Frontend,
    TypeInner::Struct,
};

fn main() {
    let mut frontend = Frontend::new();
    let module = match  frontend.parse(SHADER) {
        Ok(module) => module,
        Err(e) => {panic!("Parse Error: {}", e)},
    };

    //println!("Hello, world!\n{}", SHADER);
    println!("== Module ==");
    println!("global_variables count: {}", module.global_variables.len());
    println!("types (count: {})", module.types.len());

    for entry in module.types.iter() {
        let (handle, type_def) = entry;
        let anonymous: String = "<anonymous>".to_string();
        let name = match &type_def.name {
            Some(name) => name,
            None => &anonymous
        };
        println!(" - {}: {}", handle.index(), name);
        if let Struct{members, span: _} = &type_def.inner {
            println!("   Struct ({} members)", members.len());
        }
    }
}
