import { greet, run } from '../../wgsl-parser-wasm/pkg';
import './style.css';

const SHADER = `
struct Uniforms {
    kernel: mat3x3<f32>,
    test: f32,
}

@group(0) @binding(0) var inputTexture: texture_2d<f32>;
@group(0) @binding(1) var outputTexture: texture_storage_2d<rgba8unorm,write>;
@group(0) @binding(2) var<uniform> uniforms: Uniforms;
`;

const ret = run(SHADER);
console.log(JSON.parse(ret));
