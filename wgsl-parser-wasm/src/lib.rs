use wasm_bindgen::prelude::*;
use web_sys::console::log_1;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Point {
    x: i32,
    y: i32,
}

macro_rules! console_log {
    ($($t:tt)*) => (log_1(&format_args!($($t)*).to_string().into()))
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet(name: &str) {
    alert(&format!("Hello, {}!", name));
}

#[wasm_bindgen]
pub fn run(shader_source: &str) -> String {
    let point = Point { x: 1, y: 42 };
    let serialized = serde_json::to_string(&point).unwrap();
    console_log!("Hello using web-sys: {}", serialized);
    wgsl_parser::inspect_from_source(shader_source);
    return serialized;
}
