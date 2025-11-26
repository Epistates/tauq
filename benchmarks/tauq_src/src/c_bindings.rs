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
