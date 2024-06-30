use js_sys::Function;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    pub type Terminal;

    #[wasm_bindgen(constructor)]
    pub fn new(options: &JsValue) -> Terminal;

    #[wasm_bindgen(method)]
    pub fn open(this: &Terminal, element: &JsValue);

    #[wasm_bindgen(method, getter)]
    pub fn cols(this: &Terminal) -> f64;

    #[wasm_bindgen(method, getter)]
    pub fn rows(this: &Terminal) -> f64;

    #[wasm_bindgen(method)]
    pub fn write(this: &Terminal, data: &JsValue);

    #[wasm_bindgen(method)]
    pub fn writeln(this: &Terminal, data: &JsValue);

    #[wasm_bindgen(method)]
    pub fn onData(this: &Terminal, callback: &Function);

    #[wasm_bindgen(method)]
    pub fn clear(this: &Terminal);
}
