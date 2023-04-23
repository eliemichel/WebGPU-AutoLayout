const SHADER: &str = r#"
struct Uniforms {
    kernel: mat3x3<f32>,
    test: f32,
    test2: array<f32, 5>,
    test3: array<vec3<f32>, 5>,
    test3: array<vec4<f32>, 5>,
    test3: array<mat3x3<f32>, 5>,
}

@group(0) @binding(0) var inputTexture: texture_2d<f32>;
@group(0) @binding(1) var outputTexture: texture_storage_2d<rgba8unorm,write>;
@group(0) @binding(2) var<uniform> uniforms: Uniforms;
"#;

fn main() {
    let cpp_source = wgsl_parser::generate_cpp_binding(SHADER);
    println!("// C++ Source:\n{}", cpp_source);
}
