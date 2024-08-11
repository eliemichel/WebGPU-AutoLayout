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
    position: vec3<f32>,
    view: mat4x4<f32>,

    // Even complex members work, e.g.:
    @align(128) stuff: array<mat2x3<f32>,5>,
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
  value: generate_cpp_binding(SHADER, false /* useAlignas */),
  mode: "ace/mode/c_cpp",
  theme: "ace/theme/xcode",
  readOnly: true,
})

const useAlignasRef = {
  current: null,
};

function recomputeCppEditorValue() {
  const useAlignas = useAlignasRef.current ? useAlignasRef.current.checked : false;
  console.log("useAlignas", useAlignas);
  const wgsl = wgslEditor.getValue();
  const cpp = generate_cpp_binding(wgsl, useAlignas);
  cppEditor.setValue(cpp);
  cppEditor.selection.clearSelection();
}

wgslEditor.session.on('change', recomputeCppEditorValue);

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

function optionsComponent() {

  const label = document.createElement('label');
  
  const checkbox = document.createElement('input');
  checkbox.type = "checkbox";
  checkbox.id = "use-alignas";
  useAlignasRef.current = checkbox;
  checkbox.addEventListener('change', recomputeCppEditorValue);

  const labelText = document.createTextNode("use alignas");

  label.replaceChildren(checkbox, labelText);

  const root = document.createElement('div');
  root.replaceChildren(label);

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
document.body.appendChild(optionsComponent());
document.body.appendChild(infoComponent());
