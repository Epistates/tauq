use crate::{compile_tauq, compile_tauqq, format_to_tauq, minify_tauq_str};
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

fn set_error(err: String) {
    LAST_ERROR.with(|e| *e.borrow_mut() = err);
}

/// Get the last error message.
/// If `buffer` is null, returns the length of the error message.
/// If `buffer` is not null, copies up to `size` bytes into `buffer`, ensuring null-termination.
/// Returns the number of bytes copied (excluding null terminator).
///
/// # Safety
/// - If `buffer` is not null, it must point to a valid memory region of at least `size` bytes.
/// - The caller must ensure the buffer is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tauq_get_last_error(buffer: *mut c_char, size: usize) -> usize {
    LAST_ERROR.with(|e| {
        let err = e.borrow();
        let len = err.len();

        if buffer.is_null() {
            return len;
        }

        if size == 0 {
            return 0;
        }

        let bytes = err.as_bytes();
        let copy_len = std::cmp::min(len, size - 1);

        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer as *mut u8, copy_len);
            *buffer.add(copy_len) = 0; // Null terminate
        }

        copy_len
    })
}

/// Parse Tauq string to JSON string.
/// Caller must free the result with `tauq_free_string`.
///
/// # Safety
/// - `input` must be a valid pointer to a null-terminated UTF-8 string, or null.
/// - If null is passed, the function returns null and sets an error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tauq_to_json(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        set_error("Input pointer is null".to_string());
        return std::ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(input) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in input: {}", e));
            return std::ptr::null_mut();
        }
    };

    match compile_tauq(str_slice) {
        Ok(json_val) => {
            let json_str = json_val.to_string();
            match CString::new(json_str) {
                Ok(c) => c.into_raw(),
                Err(e) => {
                    set_error(format!("Nul byte in output JSON: {}", e));
                    std::ptr::null_mut()
                }
            }
        }
        Err(e) => {
            set_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Execute Tauq Query (TQQ) string to JSON string.
/// Caller must free the result with `tauq_free_string`.
///
/// # Safety
/// - `input` must be a valid pointer to a null-terminated UTF-8 string, or null.
/// - If null is passed, the function returns null and sets an error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tauq_exec_query(input: *const c_char, safe_mode: bool) -> *mut c_char {
    if input.is_null() {
        set_error("Input pointer is null".to_string());
        return std::ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(input) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in input: {}", e));
            return std::ptr::null_mut();
        }
    };

    match compile_tauqq(str_slice, safe_mode) {
        Ok(json_val) => {
            let json_str = json_val.to_string();
            match CString::new(json_str) {
                Ok(c) => c.into_raw(),
                Err(e) => {
                    set_error(format!("Nul byte in output JSON: {}", e));
                    std::ptr::null_mut()
                }
            }
        }
        Err(e) => {
            set_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Minify Tauq source to single-line Tauq string.
/// Caller must free the result with `tauq_free_string`.
///
/// # Safety
/// - `input` must be a valid pointer to a null-terminated UTF-8 string, or null.
/// - If null is passed, the function returns null and sets an error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tauq_minify(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        set_error("Input pointer is null".to_string());
        return std::ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(input) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in input: {}", e));
            return std::ptr::null_mut();
        }
    };

    // Parse first
    match compile_tauq(str_slice) {
        Ok(json_val) => {
            let minified = minify_tauq_str(&json_val);
            match CString::new(minified) {
                Ok(c) => c.into_raw(),
                Err(e) => {
                    set_error(format!("Nul byte in output: {}", e));
                    std::ptr::null_mut()
                }
            }
        }
        Err(e) => {
            set_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Convert JSON string to Tauq string.
/// Caller must free the result with `tauq_free_string`.
///
/// # Safety
/// - `input` must be a valid pointer to a null-terminated UTF-8 string, or null.
/// - If null is passed, the function returns null and sets an error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_to_tauq_c(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        set_error("Input pointer is null".to_string());
        return std::ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(input) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in input: {}", e));
            return std::ptr::null_mut();
        }
    };

    // Parse JSON first
    let json_val: serde_json::Value = match serde_json::from_str(str_slice) {
        Ok(v) => v,
        Err(e) => {
            set_error(format!("JSON parse error: {}", e));
            return std::ptr::null_mut();
        }
    };

    let tauq_str = format_to_tauq(&json_val);
    match CString::new(tauq_str) {
        Ok(c) => c.into_raw(),
        Err(e) => {
            set_error(format!("Nul byte in output: {}", e));
            std::ptr::null_mut()
        }
    }
}

/// Free a string returned by tauq functions
///
/// # Safety
/// This function reconstructs a CString from a raw pointer to free the memory.
/// The pointer MUST have been allocated by a Tauq function (e.g., tauq_to_json).
/// Passing a pointer not allocated by Tauq, or a pointer that has already been freed,
/// will result in UNDEFINED BEHAVIOR (e.g., double free, heap corruption).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tauq_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = unsafe { CString::from_raw(s) };
    }
}

/// Convert JSON/Tauq string to TBF bytes.
/// Returns pointer to bytes, sets out_len to length.
/// Result must be freed with `tauq_free_buffer`.
///
/// # Safety
///
/// - `input` must be a valid, null-terminated C string pointer.
/// - `out_len` must be a valid pointer to a writable `usize` location.
/// - The returned pointer must be freed with `tauq_free_buffer` using the
///   exact length written to `out_len`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tauq_to_tbf(input: *const c_char, out_len: *mut usize) -> *mut u8 {
    if input.is_null() || out_len.is_null() {
        set_error("Input pointer or out_len is null".to_string());
        return std::ptr::null_mut();
    }
    
    let c_str = unsafe { CStr::from_ptr(input) };
    let str_slice = match c_str.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in input: {}", e));
            return std::ptr::null_mut();
        }
    };

    // Auto-detect JSON vs Tauq
    let json_val = if str_slice.trim_start().starts_with('{') || str_slice.trim_start().starts_with('[') {
        match serde_json::from_str(str_slice) {
            Ok(v) => v,
            Err(e) => {
                set_error(format!("JSON parse error: {}", e));
                return std::ptr::null_mut();
            }
        }
    } else {
        match compile_tauq(str_slice) {
            Ok(v) => v,
            Err(e) => {
                set_error(format!("Tauq parse error: {}", e));
                return std::ptr::null_mut();
            }
        }
    };

    match crate::tbf::encode_json(&json_val) {
        Ok(vec) => {
            let mut buf = vec.into_boxed_slice();
            let ptr = buf.as_mut_ptr();
            let len = buf.len();
            std::mem::forget(buf);
            unsafe { *out_len = len };
            ptr
        },
        Err(e) => {
            set_error(format!("TBF encode error: {}", e));
            std::ptr::null_mut()
        }
    }
}

/// Convert TBF bytes to JSON string.
/// Caller must free result with `tauq_free_string`.
///
/// # Safety
///
/// - `data` must be a valid pointer to `len` bytes of TBF-encoded data.
/// - `len` must accurately reflect the number of bytes at `data`.
/// - The returned string must be freed with `tauq_free_string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tauq_tbf_to_json(data: *const u8, len: usize) -> *mut c_char {
    if data.is_null() || len == 0 {
        set_error("Invalid data pointer or length".to_string());
        return std::ptr::null_mut();
    }
    
    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    
    match crate::tbf::decode(slice) {
        Ok(json_val) => {
            let json_str = json_val.to_string();
            match CString::new(json_str) {
                Ok(c) => c.into_raw(),
                Err(e) => {
                    set_error(format!("Nul byte in output: {}", e));
                    std::ptr::null_mut()
                }
            }
        },
        Err(e) => {
            set_error(format!("TBF decode error: {}", e));
            std::ptr::null_mut()
        }
    }
}

/// Convert TBF bytes to Tauq string.
/// Caller must free result with `tauq_free_string`.
///
/// # Safety
///
/// - `data` must be a valid pointer to `len` bytes of TBF-encoded data.
/// - `len` must accurately reflect the number of bytes at `data`.
/// - The returned string must be freed with `tauq_free_string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tauq_tbf_to_tauq(data: *const u8, len: usize) -> *mut c_char {
    if data.is_null() || len == 0 {
        set_error("Invalid data pointer or length".to_string());
        return std::ptr::null_mut();
    }
    
    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    
    match crate::tbf::decode_to_tauq(slice) {
        Ok(tauq_str) => {
            match CString::new(tauq_str) {
                Ok(c) => c.into_raw(),
                Err(e) => {
                    set_error(format!("Nul byte in output: {}", e));
                    std::ptr::null_mut()
                }
            }
        },
        Err(e) => {
            set_error(format!("TBF decode error: {}", e));
            std::ptr::null_mut()
        }
    }
}

/// Free a buffer returned by tauq_to_tbf.
/// Must pass the exact length returned by the allocation.
///
/// # Safety
///
/// - `ptr` must be a pointer previously returned by `tauq_to_tbf`.
/// - `len` must be the exact length that was written to `out_len` by `tauq_to_tbf`.
/// - The pointer must not have been freed already.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tauq_free_buffer(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
        let _ = unsafe { Box::from_raw(slice) };
    }
}
