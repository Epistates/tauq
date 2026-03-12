// Java Native Interface (JNI) Bindings for Tauq
//
// Enables Java/Kotlin applications to parse and generate Tauq via JNI.
//
// Native Methods:
// - Java_com_tauq_Tauq_parseToJson(EnvUnowned, JClass, jstring) -> jstring
// - Java_com_tauq_Tauq_formatJson(EnvUnowned, JClass, jstring) -> jstring
// - Java_com_tauq_Tauq_toTbf(EnvUnowned, JClass, jstring) -> jbyteArray
// - Java_com_tauq_Tauq_tbfToJson(EnvUnowned, JClass, jbyteArray) -> jstring

use crate::{compile_tauq, format_to_tauq};
use jni::EnvUnowned;
use jni::errors::ThrowRuntimeExAndDefault;
use jni::jni_str;
use jni::objects::{JByteArray, JClass, JString};
use jni::strings::JNIString;
use jni::sys::jstring;

/// Class:     com_tauq_Tauq
/// Method:    parseToJson
/// Signature: (Ljava/lang/String;)Ljava/lang/String;
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_tauq_Tauq_parseToJson<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _class: JClass<'local>,
    input: JString<'local>,
) -> jstring {
    unowned_env
        .with_env(|env| -> jni::errors::Result<jstring> {
            // 1. Convert Java String to Rust String
            let input: String = input.mutf8_chars(env)?.into();

            // 2. Compile Tauq
            let result = match compile_tauq(&input) {
                Ok(json_val) => json_val,
                Err(e) => {
                    let _ = env.throw_new(
                        jni_str!("java/lang/IllegalArgumentException"),
                        JNIString::new(format!("Tauq Parse Error: {}", e)),
                    );
                    return Ok(std::ptr::null_mut());
                }
            };

            // 3. Serialize to JSON String
            let json_str = match serde_json::to_string(&result) {
                Ok(s) => s,
                Err(e) => {
                    let _ = env.throw_new(
                        jni_str!("java/lang/RuntimeException"),
                        JNIString::new(format!("JSON Serialization Error: {}", e)),
                    );
                    return Ok(std::ptr::null_mut());
                }
            };

            // 4. Convert Rust String back to Java String
            Ok(env.new_string(json_str)?.into_raw())
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

/// Class:     com_tauq_Tauq
/// Method:    execQuery
/// Signature: (Ljava/lang/String;Z)Ljava/lang/String;
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_tauq_Tauq_execQuery<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _class: JClass<'local>,
    input: JString<'local>,
    safe_mode: jni::sys::jboolean,
) -> jstring {
    unowned_env
        .with_env(|env| -> jni::errors::Result<jstring> {
            // 1. Convert Java String
            let input: String = input.mutf8_chars(env)?.into();

            // 2. Execute TQQ — jboolean is bool in jni 0.22
            let is_safe = safe_mode;

            let result = match crate::compile_tauqq(&input, is_safe) {
                Ok(json_val) => json_val,
                Err(e) => {
                    let _ = env.throw_new(
                        jni_str!("java/lang/IllegalArgumentException"),
                        JNIString::new(format!("Tauq Query Error: {}", e)),
                    );
                    return Ok(std::ptr::null_mut());
                }
            };

            // 3. Serialize
            let json_str = match serde_json::to_string(&result) {
                Ok(s) => s,
                Err(e) => {
                    let _ = env.throw_new(
                        jni_str!("java/lang/RuntimeException"),
                        JNIString::new(format!("JSON Serialization Error: {}", e)),
                    );
                    return Ok(std::ptr::null_mut());
                }
            };

            // 4. Return
            Ok(env.new_string(json_str)?.into_raw())
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

/// Class:     com_tauq_Tauq
/// Method:    minify
/// Signature: (Ljava/lang/String;)Ljava/lang/String;
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_tauq_Tauq_minify<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _class: JClass<'local>,
    input: JString<'local>,
) -> jstring {
    unowned_env
        .with_env(|env| -> jni::errors::Result<jstring> {
            let input: String = input.mutf8_chars(env)?.into();

            let json_val = match crate::compile_tauq(&input) {
                Ok(v) => v,
                Err(e) => {
                    let _ = env.throw_new(
                        jni_str!("java/lang/IllegalArgumentException"),
                        JNIString::new(format!("Tauq Parse Error: {}", e)),
                    );
                    return Ok(std::ptr::null_mut());
                }
            };

            let minified = crate::minify_tauq_str(&json_val);

            Ok(env.new_string(minified)?.into_raw())
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

/// Class:     com_tauq_Tauq
/// Method:    formatJson
/// Signature: (Ljava/lang/String;)Ljava/lang/String;
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_tauq_Tauq_formatJson<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _class: JClass<'local>,
    json_input: JString<'local>,
) -> jstring {
    unowned_env
        .with_env(|env| -> jni::errors::Result<jstring> {
            // 1. Convert Java String to Rust String
            let input: String = json_input.mutf8_chars(env)?.into();

            // 2. Parse JSON
            let json_val: serde_json::Value = match serde_json::from_str(&input) {
                Ok(v) => v,
                Err(e) => {
                    let _ = env.throw_new(
                        jni_str!("java/lang/IllegalArgumentException"),
                        JNIString::new(format!("Invalid JSON: {}", e)),
                    );
                    return Ok(std::ptr::null_mut());
                }
            };

            // 3. Format to Tauq
            let tauq_str = format_to_tauq(&json_val);

            // 4. Return Java String
            Ok(env.new_string(tauq_str)?.into_raw())
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

/// Class:     com_tauq_Tauq
/// Method:    toTbf
/// Signature: (Ljava/lang/String;)[B
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_tauq_Tauq_toTbf<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _class: JClass<'local>,
    input: JString<'local>,
) -> jni::sys::jbyteArray {
    unowned_env
        .with_env(|env| -> jni::errors::Result<jni::sys::jbyteArray> {
            // 1. Convert Java String
            let input: String = input.mutf8_chars(env)?.into();

            // 2. Parse (Auto-detect JSON/Tauq) and Encode
            let json_val =
                if input.trim_start().starts_with('{') || input.trim_start().starts_with('[') {
                    match serde_json::from_str(&input) {
                        Ok(v) => v,
                        Err(e) => {
                            let _ = env.throw_new(
                                jni_str!("java/lang/IllegalArgumentException"),
                                JNIString::new(format!("JSON Parse Error: {}", e)),
                            );
                            return Ok(std::ptr::null_mut());
                        }
                    }
                } else {
                    match compile_tauq(&input) {
                        Ok(v) => v,
                        Err(e) => {
                            let _ = env.throw_new(
                                jni_str!("java/lang/IllegalArgumentException"),
                                JNIString::new(format!("Tauq Parse Error: {}", e)),
                            );
                            return Ok(std::ptr::null_mut());
                        }
                    }
                };

            match crate::tbf::encode_json(&json_val) {
                Ok(bytes) => match env.byte_array_from_slice(&bytes) {
                    Ok(arr) => Ok(arr.into_raw()),
                    Err(e) => {
                        let _ = env.throw_new(
                            jni_str!("java/lang/RuntimeException"),
                            JNIString::new(format!("JNI Array Creation Error: {}", e)),
                        );
                        Ok(std::ptr::null_mut())
                    }
                },
                Err(e) => {
                    let _ = env.throw_new(
                        jni_str!("java/lang/RuntimeException"),
                        JNIString::new(format!("TBF Encode Error: {}", e)),
                    );
                    Ok(std::ptr::null_mut())
                }
            }
        })
        .resolve::<ThrowRuntimeExAndDefault>()
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
pub extern "system" fn Java_com_tauq_Tauq_tbfToJson<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _class: JClass<'local>,
    data: jni::sys::jbyteArray,
) -> jstring {
    unowned_env
        .with_env(|env| -> jni::errors::Result<jstring> {
            let array = unsafe { JByteArray::from_raw(env, data) };
            let bytes = env.convert_byte_array(&array)?;

            match crate::tbf::decode(&bytes) {
                Ok(json_val) => match serde_json::to_string(&json_val) {
                    Ok(s) => Ok(env.new_string(s)?.into_raw()),
                    Err(e) => {
                        let _ = env.throw_new(
                            jni_str!("java/lang/RuntimeException"),
                            JNIString::new(format!("JSON Serialize Error: {}", e)),
                        );
                        Ok(std::ptr::null_mut())
                    }
                },
                Err(e) => {
                    let _ = env.throw_new(
                        jni_str!("java/lang/IllegalArgumentException"),
                        JNIString::new(format!("TBF Decode Error: {}", e)),
                    );
                    Ok(std::ptr::null_mut())
                }
            }
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

/// Class:     com_tauq_Tauq
/// Method:    tbfToTauq
/// Signature: ([B)Ljava/lang/String;
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "system" fn Java_com_tauq_Tauq_tbfToTauq<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _class: JClass<'local>,
    data: jni::sys::jbyteArray,
) -> jstring {
    unowned_env
        .with_env(|env| -> jni::errors::Result<jstring> {
            let array = unsafe { jni::objects::JByteArray::from_raw(env, data) };
            let bytes = env.convert_byte_array(&array)?;

            match crate::tbf::decode_to_tauq(&bytes) {
                Ok(tauq_str) => Ok(env.new_string(tauq_str)?.into_raw()),
                Err(e) => {
                    let _ = env.throw_new(
                        jni_str!("java/lang/IllegalArgumentException"),
                        JNIString::new(format!("TBF Decode Error: {}", e)),
                    );
                    Ok(std::ptr::null_mut())
                }
            }
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}
