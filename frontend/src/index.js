import { generate_cpp_binding } from '../../wgsl-parser-wasm/pkg';
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

function component() {
  const element = document.createElement('div');

  element.innerHTML = "<strong>Hello</strong> world!";

  return element;
}

document.body.appendChild(component());

import ace from 'ace-builds';
import "ace-builds/src-noconflict/mode-rust";
import "ace-builds/src-noconflict/mode-c_cpp";
import "ace-builds/src-noconflict/theme-xcode";

const wgslEditor = ace.edit(null, {
    maxLines: 50,
    minLines: 10,
    value: SHADER,
    mode: "ace/mode/rust",
    theme: "ace/theme/xcode",
})

document.body.appendChild(wgslEditor.container)

document.body.appendChild(component());

const cppEditor = ace.edit(null, {
    maxLines: 50,
    minLines: 10,
    value: generate_cpp_binding(SHADER),
    mode: "ace/mode/c_cpp",
    theme: "ace/theme/xcode",
    readOnly: true,
})

document.body.appendChild(cppEditor.container)

wgslEditor.session.on('change', function(delta) {
    const wgsl = wgslEditor.getValue();
    const cpp = generate_cpp_binding(wgsl);
    cppEditor.setValue(cpp);
    cppEditor.selection.clearSelection();
});
