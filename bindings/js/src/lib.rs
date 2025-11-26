use wasm_bindgen::prelude::*;
use tauq::{compile_tauq, compile_tauqq, format_to_tauq, minify_tauq_str};

#[wasm_bindgen]
pub fn parse(input: &str) -> Result<JsValue, JsValue> {
    let json_val = compile_tauq(input)
        .map_err(|e| JsValue::from_str(&format!("Tauq Parse Error: {}", e)))?;
        
    serde_wasm_bindgen::to_value(&json_val)
        .map_err(|e| JsValue::from_str(&format!("Serialization Error: {}", e)))
}

#[wasm_bindgen]
pub fn exec(input: &str, safe_mode: bool) -> Result<JsValue, JsValue> {
    let json_val = compile_tauqq(input, safe_mode)
        .map_err(|e| JsValue::from_str(&format!("Tauq Query Error: {}", e)))?;
        
    serde_wasm_bindgen::to_value(&json_val)
        .map_err(|e| JsValue::from_str(&format!("Serialization Error: {}", e)))
}

#[wasm_bindgen]
pub fn minify(input: &str) -> Result<String, JsValue> {
    let json_val = compile_tauq(input)
        .map_err(|e| JsValue::from_str(&format!("Tauq Parse Error: {}", e)))?;
        
    Ok(minify_tauq_str(&json_val))
}

#[wasm_bindgen]
pub fn stringify(value: JsValue) -> Result<String, JsValue> {
    let json_val: serde_json::Value = serde_wasm_bindgen::from_value(value)
        .map_err(|e| JsValue::from_str(&format!("Invalid JS Value: {}", e)))?;
        
    Ok(format_to_tauq(&json_val))
}

#[wasm_bindgen]
pub fn to_json(input: &str) -> Result<String, JsValue> {
    let json_val = compile_tauq(input)
        .map_err(|e| JsValue::from_str(&format!("Tauq Parse Error: {}", e)))?;
    
    serde_json::to_string(&json_val)
        .map_err(|e| JsValue::from_str(&format!("JSON Serialize Error: {}", e)))
}