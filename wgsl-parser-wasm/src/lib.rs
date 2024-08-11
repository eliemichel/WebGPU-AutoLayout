use wasm_bindgen::prelude::*;

/*
use web_sys::console::log_1;

macro_rules! console_log {
    ($($t:tt)*) => (log_1(&format_args!($($t)*).to_string().into()))
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}
*/

#[wasm_bindgen]
pub fn generate_cpp_binding(shader_source: &str, use_alignas: bool) -> String {
    return wgsl_parser::generate_cpp_binding(shader_source, use_alignas);
}
