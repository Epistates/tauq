// Java Native Interface (JNI) Bindings for Tauq
//
// Enables Java/Kotlin applications to parse and generate Tauq via JNI.
//
// Native Methods:
// - Java_com_tauq_Tauq_parseToJson(JNIEnv, JClass, jstring) -> jstring
// - Java_com_tauq_Tauq_formatJson(JNIEnv, JClass, jstring) -> jstring

use crate::{compile_tauq, format_to_tauq};
use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::jstring;
use std::ffi::CString;

/// Class:     com_tauq_Tauq
/// Method:    parseToJson
/// Signature: (Ljava/lang/String;)Ljava/lang/String;
#[no_mangle]
pub extern "system" fn Java_com_tauq_Tauq_parseToJson(
    env: JNIEnv,
    _class: JClass,
    input: JString,
) -> jstring {
    // 1. Convert Java String to Rust String
    let input: String = match env.get_string(input) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    // 2. Compile Tauq
    let result = match compile_tauq(&input) {
        Ok(json_val) => json_val,
        Err(e) => {
            // Throw Java Exception
            let _ = env.throw_new(
                "java/lang/IllegalArgumentException",
                format!("Tauq Parse Error: {}", e),
            );
            return std::ptr::null_mut();
        }
    };

    // 3. Serialize to JSON String
    let json_str = match serde_json::to_string(&result) {
        Ok(s) => s,
        Err(e) => {
            let _ = env.throw_new(
                "java/lang/RuntimeException",
                format!("JSON Serialization Error: {}", e),
            );
            return std::ptr::null_mut();
        }
    };

    // 4. Convert Rust String back to Java String
    let output = match env.new_string(json_str) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    output.into_raw()
}

/// Class:     com_tauq_Tauq
/// Method:    execQuery
/// Signature: (Ljava/lang/String;Z)Ljava/lang/String;
#[no_mangle]
pub extern "system" fn Java_com_tauq_Tauq_execQuery(
    env: JNIEnv,
    _class: JClass,
    input: JString,
    safe_mode: jni::sys::jboolean,
) -> jstring {
    // 1. Convert Java String
    let input: String = match env.get_string(input) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    // 2. Execute TQQ
    let is_safe = safe_mode != 0;

    let result = match crate::compile_tauqq(&input, is_safe) {
        Ok(json_val) => json_val,
        Err(e) => {
            let _ = env.throw_new(
                "java/lang/IllegalArgumentException",
                format!("Tauq Query Error: {}", e),
            );
            return std::ptr::null_mut();
        }
    };

    // 3. Serialize
    let json_str = match serde_json::to_string(&result) {
        Ok(s) => s,
        Err(e) => {
            let _ = env.throw_new(
                "java/lang/RuntimeException",
                format!("JSON Serialization Error: {}", e),
            );
            return std::ptr::null_mut();
        }
    };

    // 4. Return
    let output = match env.new_string(json_str) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    output.into_raw()
}

/// Class:     com_tauq_Tauq
/// Method:    minify
/// Signature: (Ljava/lang/String;)Ljava/lang/String;
#[no_mangle]
pub extern "system" fn Java_com_tauq_Tauq_minify(
    env: JNIEnv,
    _class: JClass,
    input: JString,
) -> jstring {
    let input: String = match env.get_string(input) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let json_val = match crate::compile_tauq(&input) {
        Ok(v) => v,
        Err(e) => {
            let _ = env.throw_new(
                "java/lang/IllegalArgumentException",
                format!("Tauq Parse Error: {}", e),
            );
            return std::ptr::null_mut();
        }
    };

    let minified = crate::minify_tauq_str(&json_val);

    let output = match env.new_string(minified) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    output.into_raw()
}

/// Class:     com_tauq_Tauq
/// Method:    formatJson
/// Signature: (Ljava/lang/String;)Ljava/lang/String;
#[no_mangle]
pub extern "system" fn Java_com_tauq_Tauq_formatJson(
    env: JNIEnv,
    _class: JClass,
    json_input: JString,
) -> jstring {
    // 1. Convert Java String to Rust String
    let input: String = match env.get_string(json_input) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    // 2. Parse JSON
    let json_val: serde_json::Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(e) => {
            let _ = env.throw_new(
                "java/lang/IllegalArgumentException",
                format!("Invalid JSON: {}", e),
            );
            return std::ptr::null_mut();
        }
    };

    // 3. Format to Tauq
    let tauq_str = format_to_tauq(&json_val);

    // 4. Return Java String
    let output = match env.new_string(tauq_str) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    output.into_raw()
}
