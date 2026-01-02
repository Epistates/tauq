// Java Native Interface (JNI) Bindings for Tauq
//
// Enables Java/Kotlin applications to parse and generate Tauq via JNI.
//
// Native Methods:
// - Java_com_tauq_Tauq_parseToJson(JNIEnv, JClass, jstring) -> jstring
// - Java_com_tauq_Tauq_formatJson(JNIEnv, JClass, jstring) -> jstring
// - Java_com_tauq_Tauq_toTbf(JNIEnv, JClass, jstring) -> jbyteArray
// - Java_com_tauq_Tauq_tbfToJson(JNIEnv, JClass, jbyteArray) -> jstring

use crate::{compile_tauq, format_to_tauq};
use jni::JNIEnv;
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::jstring;

/// Class:     com_tauq_Tauq
/// Method:    parseToJson
/// Signature: (Ljava/lang/String;)Ljava/lang/String;
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_tauq_Tauq_parseToJson(
    mut env: JNIEnv,
    _class: JClass,
    input: JString,
) -> jstring {
    // 1. Convert Java String to Rust String
    let input: String = match env.get_string(&input) {
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
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_tauq_Tauq_execQuery(
    mut env: JNIEnv,
    _class: JClass,
    input: JString,
    safe_mode: jni::sys::jboolean,
) -> jstring {
    // 1. Convert Java String
    let input: String = match env.get_string(&input) {
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
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_tauq_Tauq_minify(
    mut env: JNIEnv,
    _class: JClass,
    input: JString,
) -> jstring {
    let input: String = match env.get_string(&input) {
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
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_tauq_Tauq_formatJson(
    mut env: JNIEnv,
    _class: JClass,
    json_input: JString,
) -> jstring {
    // 1. Convert Java String to Rust String
    let input: String = match env.get_string(&json_input) {
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

/// Class:     com_tauq_Tauq
/// Method:    toTbf
/// Signature: (Ljava/lang/String;)V
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_tauq_Tauq_toTbf(
    mut env: JNIEnv,
    _class: JClass,
    input: JString,
) -> jni::sys::jbyteArray {
    // 1. Convert Java String
    let input: String = match env.get_string(&input) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    // 2. Parse (Auto-detect JSON/Tauq) and Encode
    let json_val = if input.trim_start().starts_with('{') || input.trim_start().starts_with('[') {
        match serde_json::from_str(&input) {
            Ok(v) => v,
            Err(e) => {
                 let _ = env.throw_new("java/lang/IllegalArgumentException", format!("JSON Parse Error: {}", e));
                 return std::ptr::null_mut();
            }
        }
    } else {
        match compile_tauq(&input) {
            Ok(v) => v,
            Err(e) => {
                 let _ = env.throw_new("java/lang/IllegalArgumentException", format!("Tauq Parse Error: {}", e));
                 return std::ptr::null_mut();
            }
        }
    };

    match crate::tbf::encode_json(&json_val) {
        Ok(bytes) => {
            let output = match env.byte_array_from_slice(&bytes) {
                Ok(arr) => arr,
                Err(e) => {
                    let _ = env.throw_new("java/lang/RuntimeException", format!("JNI Array Creation Error: {}", e));
                    return std::ptr::null_mut();
                }
            };
            output.into_raw()
        },
        Err(e) => {
            let _ = env.throw_new("java/lang/RuntimeException", format!("TBF Encode Error: {}", e));
            std::ptr::null_mut()
        }
    }
}

/// Class:     com_tauq_Tauq
/// Method:    tbfToJson
/// Signature: ([B)Ljava/lang/String;
///
/// # Safety
///
/// This function is a JNI callback and must be called from Java with a valid
/// `jbyteArray` pointer. The `data` parameter must be a valid JNI byte array
/// reference obtained from the JVM.
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "system" fn Java_com_tauq_Tauq_tbfToJson(
    mut env: JNIEnv,
    _class: JClass,
    data: jni::sys::jbyteArray,
) -> jstring {
    let array = unsafe { JByteArray::from_raw(data) };
    let bytes = match env.convert_byte_array(&array) {
        Ok(b) => b,
        Err(_) => return std::ptr::null_mut(),
    };

    match crate::tbf::decode(&bytes) {
        Ok(json_val) => {
            match serde_json::to_string(&json_val) {
                Ok(s) => {
                    match env.new_string(s) {
                        Ok(js) => js.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                },
                Err(e) => {
                     let _ = env.throw_new("java/lang/RuntimeException", format!("JSON Serialize Error: {}", e));
                     std::ptr::null_mut()
                }
            }
        },
        Err(e) => {
             let _ = env.throw_new("java/lang/IllegalArgumentException", format!("TBF Decode Error: {}", e));
             std::ptr::null_mut()
        }
    }
}
