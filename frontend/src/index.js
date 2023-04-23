import ace from 'ace-builds';
import "ace-builds/src-noconflict/mode-rust";
import "ace-builds/src-noconflict/mode-c_cpp";
import "ace-builds/src-noconflict/theme-xcode";

import { generate_cpp_binding } from '../../wgsl-parser-wasm/pkg';
import './style.css';

/*
const SHADER = `
struct Uniforms {
    kernel: mat3x3<f32>,
    test: f32,
}

@group(0) @binding(0) var inputTexture: texture_2d<f32>;
@group(0) @binding(1) var outputTexture: texture_storage_2d<rgba8unorm,write>;
@group(0) @binding(2) var<uniform> uniforms: Uniforms;
`;
*/

const SHADER = `
struct Uniforms {
    test: vec3<f32>,
    kernel: mat4x4<f32>,
}
`;

const wgslEditor = ace.edit(null, {
  maxLines: 50,
  minLines: 10,
  value: SHADER,
  mode: "ace/mode/rust",
  theme: "ace/theme/xcode",
});

const cppEditor = ace.edit(null, {
  maxLines: 50,
  minLines: 10,
  value: generate_cpp_binding(SHADER),
  mode: "ace/mode/c_cpp",
  theme: "ace/theme/xcode",
  readOnly: true,
})

wgslEditor.session.on('change', function(delta) {
    const wgsl = wgslEditor.getValue();
    const cpp = generate_cpp_binding(wgsl);
    cppEditor.setValue(cpp);
    cppEditor.selection.clearSelection();
});

function wgslEditorComponent() {

  const label = document.createElement('div');
  label.innerHTML = "<strong>Your WGSL:</strong>";

  const root = document.createElement('div');
  root.replaceChildren(label, wgslEditor.container);

  return root;
}

function cppEditorComponent() {

  const label = document.createElement('div');
  label.innerHTML = "<strong>Generated C++:</strong>";

  const root = document.createElement('div');
  root.replaceChildren(label, cppEditor.container);

  return root;
}

function infoComponent() {

  const info = document.createElement('p');
  info.innerHTML = `<em>
    This is a small utility tool to generate C++ structures that match the memory
    layout of WGSL host-sharable structures, as defined in
    <a href="https://gpuweb.github.io/gpuweb/wgsl/#structure-member-layout">the specification</a>.
    You may <a href="https://github.com/eliemichel/WebGPU-AutoLayout">Fork me on GitHub</a>.
  </em>`;

  const root = document.createElement('div');
  root.replaceChildren(info);

  return root;
}

document.body.appendChild(wgslEditorComponent());
document.body.appendChild(cppEditorComponent());
document.body.appendChild(infoComponent());
