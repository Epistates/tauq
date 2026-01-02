// Python Bindings for Tauq
#![allow(unsafe_op_in_unsafe_fn)]
//
// Enables Python applications to parse and generate Tauq:
//
// ```python
// import tauq
//
// # Parse Tauq (!def implies !use, so data rows immediately follow)
// data = tauq.loads("!def Config key value\nworkers 8\ntimeout 30")
//
// # Load from file
// config = tauq.load("config.tqn")
//
// # Serialize to Tauq
// tqn_str = tauq.dumps([{"id": 1, "name": "Alice"}])
// ```

#[cfg(feature = "python-bindings")]
use pyo3::exceptions::PyValueError;
#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;
#[cfg(feature = "python-bindings")]
use pyo3::types::{PyDict, PyList};

#[cfg(feature = "python-bindings")]
use crate::{compile_tauq, compile_tauqq, format_to_tauq, minify_tauq_str};
#[cfg(feature = "python-bindings")]
use serde_json::Value as JsonValue;
#[cfg(feature = "python-bindings")]
use std::path::Path;

/// Convert JSON Value to Python object
#[cfg(feature = "python-bindings")]
fn json_to_python(py: Python, value: &JsonValue) -> PyResult<PyObject> {
    match value {
        JsonValue::Null => Ok(py.None()),
        JsonValue::Bool(b) => Ok(b.to_object(py)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.to_object(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.to_object(py))
            } else {
                Ok(py.None())
            }
        }
        JsonValue::String(s) => Ok(s.to_object(py)),
        JsonValue::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_to_python(py, item)?)?;
            }
            Ok(list.to_object(py))
        }
        JsonValue::Object(obj) => {
            let dict = PyDict::new(py);
            for (key, val) in obj {
                dict.set_item(key, json_to_python(py, val)?)?;
            }
            Ok(dict.to_object(py))
        }
    }
}

/// Convert Python object to JSON Value
#[cfg(feature = "python-bindings")]
#[allow(clippy::only_used_in_recursion)]
fn python_to_json(py: Python, obj: &PyAny) -> PyResult<JsonValue> {
    if obj.is_none() {
        Ok(JsonValue::Null)
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(JsonValue::Bool(b))
    } else if let Ok(i) = obj.extract::<i64>() {
        Ok(JsonValue::Number(i.into()))
    } else if let Ok(f) = obj.extract::<f64>() {
        Ok(JsonValue::Number(
            serde_json::Number::from_f64(f).unwrap_or(serde_json::Number::from(0)),
        ))
    } else if let Ok(s) = obj.extract::<String>() {
        Ok(JsonValue::String(s))
    } else if let Ok(list) = obj.downcast::<PyList>() {
        let mut arr = Vec::new();
        for item in list.iter() {
            arr.push(python_to_json(py, item)?);
        }
        Ok(JsonValue::Array(arr))
    } else if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (key, val) in dict.iter() {
            let key_str = key.extract::<String>()?;
            map.insert(key_str, python_to_json(py, val)?);
        }
        Ok(JsonValue::Object(map))
    } else {
        Err(PyValueError::new_err(format!(
            "Cannot convert Python type {} to JSON",
            obj.get_type().name()?
        )))
    }
}

/// Parse Tauq from a string
///
/// # Arguments
/// * `source` - Tauq source string
///
/// # Returns
/// Python dict/list/value representing the parsed Tauq
///
/// # Example
/// ```python
/// import tauq
///
/// # !def implies !use, so data rows immediately follow
/// data = tauq.loads("""
/// !def User id name email
/// 1 "Alice" "alice@example.com"
/// 2 "Bob" "bob@example.com"
/// """)
///
/// print(data[0]["name"])  # "Alice"
/// ```
#[cfg(feature = "python-bindings")]
#[pyfunction]
fn loads(py: Python, source: &str) -> PyResult<PyObject> {
    let json = compile_tauq(source)
        .map_err(|e| PyValueError::new_err(format!("Tauq parse error: {}", e)))?;

    json_to_python(py, &json)
}

/// Load Tauq from a file
///
/// # Arguments
/// * `path` - Path to Tauq file
///
/// # Returns
/// Python dict/list/value representing the parsed Tauq
///
/// # Example
/// ```python
/// import tauq
///
/// config = tauq.load("config.tqn")
/// print(config[0]["workers"])
/// ```
#[cfg(feature = "python-bindings")]
#[pyfunction]
fn load(py: Python, path: &str) -> PyResult<PyObject> {
    let source = std::fs::read_to_string(Path::new(path))
        .map_err(|e| PyValueError::new_err(format!("File read error: {}", e)))?;

    let json = compile_tauq(&source)
        .map_err(|e| PyValueError::new_err(format!("Tauq parse error: {}", e)))?;

    json_to_python(py, &json)
}

/// Execute TauqQ (programmable Tauq) from a string
///
/// # Arguments
/// * `source` - TauqQ source string
///
/// # Returns
/// Python dict/list/value representing the parsed result
///
/// # Example
/// ```python
/// import tauq
///
/// data = tauq.compile_tauqq("""
/// !set COUNT "10"
///
/// !def Item id name
/// !use Item
///
/// !run python3 {
/// import os
/// count = int(os.environ.get('COUNT', '5'))
/// for i in range(1, count + 1):
///     print(f' {i} "Item_{i}"')
/// }
/// """)
///
/// print(len(data))  # 10
/// ```
#[cfg(feature = "python-bindings")]
#[pyfunction]
fn exec_tauqq(py: Python, source: &str) -> PyResult<PyObject> {
    let json =
        compile_tauqq(source, false) // Default to unsafe in Python bindings for now
            .map_err(|e| PyValueError::new_err(format!("TauqQ execution error: {}", e)))?;

    json_to_python(py, &json)
}

/// Serialize Python object to Tauq string
///
/// # Arguments
/// * `obj` - Python dict/list/value to serialize
///
/// # Returns
/// Tauq formatted string
///
/// # Example
/// ```python
/// import tauq
///
/// data = [
///     {"id": 1, "name": "Alice"},
///     {"id": 2, "name": "Bob"}
/// ]
///
/// flux_str = tauq.dumps(data)
/// print(flux_str)
/// # Output:
/// # !def Item id name
/// # !use Item
/// #  1 Alice
/// #  2 Bob
/// ```
#[cfg(feature = "python-bindings")]
#[pyfunction]
fn dumps(py: Python, obj: &PyAny) -> PyResult<String> {
    let json = python_to_json(py, obj)?;
    Ok(format_to_tauq(&json))
}

/// Minify Tauq source to single-line Tauq string
///
/// # Arguments
/// * `source` - Tauq source string
///
/// # Returns
/// Minified Tauq string
///
/// # Example
/// ```python
/// import tauq
///
/// minified = tauq.minify("!use User 1 Alice")
/// print(minified)
/// ```
#[cfg(feature = "python-bindings")]
#[pyfunction]
fn minify(_py: Python, source: &str) -> PyResult<String> {
    let json = compile_tauq(source)
        .map_err(|e| PyValueError::new_err(format!("Tauq parse error: {}", e)))?;

    Ok(minify_tauq_str(&json))
}

/// Write Python object to Tauq file
///
/// # Arguments
/// * `obj` - Python dict/list/value to serialize
/// * `path` - Path to output file
///
/// # Example
/// ```python
/// import tauq
///
/// data = [{"id": 1, "name": "Alice"}]
/// tauq.dump(data, "output.tqn")
/// ```
#[cfg(feature = "python-bindings")]
#[pyfunction]
fn dump(py: Python, obj: &PyAny, path: &str) -> PyResult<()> {
    let json = python_to_json(py, obj)?;
    let tauq_str = format_to_tauq(&json);

    std::fs::write(path, tauq_str)
        .map_err(|e| PyValueError::new_err(format!("Write error: {}", e)))?;

    Ok(())
}

// ============================================================================
// TBF Bindings
// ============================================================================

/// Serialize Python object to TBF bytes
///
/// # Arguments
/// * `obj` - Python dict/list/value to serialize
///
/// # Returns
/// Bytes object containing TBF data
#[cfg(feature = "python-bindings")]
#[pyfunction]
fn tbf_dumps(py: Python, obj: &PyAny) -> PyResult<PyObject> {
    use pyo3::types::PyBytes;
    
    let json = python_to_json(py, obj)?;
    let bytes = crate::tbf::encode_json(&json)
        .map_err(|e| PyValueError::new_err(format!("TBF encode error: {}", e)))?;
    
    Ok(PyBytes::new(py, &bytes).into())
}

/// Deserialize TBF bytes to Python object
///
/// # Arguments
/// * `data` - Bytes object containing TBF data
///
/// # Returns
/// Python dict/list/value
#[cfg(feature = "python-bindings")]
#[pyfunction]
fn tbf_loads(py: Python, data: &[u8]) -> PyResult<PyObject> {
    let json = crate::tbf::decode(data)
        .map_err(|e| PyValueError::new_err(format!("TBF decode error: {}", e)))?;
        
    json_to_python(py, &json)
}

/// Write Python object to TBF file
///
/// # Arguments
/// * `obj` - Python dict/list/value to serialize
/// * `path` - Path to output file
#[cfg(feature = "python-bindings")]
#[pyfunction]
fn tbf_dump(py: Python, obj: &PyAny, path: &str) -> PyResult<()> {
    let json = python_to_json(py, obj)?;
    let bytes = crate::tbf::encode_json(&json)
        .map_err(|e| PyValueError::new_err(format!("TBF encode error: {}", e)))?;
        
    std::fs::write(path, bytes)
        .map_err(|e| PyValueError::new_err(format!("Write error: {}", e)))?;
        
    Ok(())
}

/// Load TBF from file
///
/// # Arguments
/// * `path` - Path to TBF file
///
/// # Returns
/// Python dict/list/value
#[cfg(feature = "python-bindings")]
#[pyfunction]
fn tbf_load(py: Python, path: &str) -> PyResult<PyObject> {
    let bytes = std::fs::read(path)
        .map_err(|e| PyValueError::new_err(format!("Read error: {}", e)))?;
        
    let json = crate::tbf::decode(&bytes)
        .map_err(|e| PyValueError::new_err(format!("TBF decode error: {}", e)))?;
        
    json_to_python(py, &json)
}

/// Convert TBF bytes directly to Tauq string
///
/// # Arguments
/// * `data` - Bytes object containing TBF data
///
/// # Returns
/// Tauq formatted string
#[cfg(feature = "python-bindings")]
#[pyfunction]
fn tbf_to_tauq(_py: Python, data: &[u8]) -> PyResult<String> {
    crate::tbf::decode_to_tauq(data)
        .map_err(|e| PyValueError::new_err(format!("TBF decode error: {}", e)))
}

/// Python module definition
#[cfg(feature = "python-bindings")]
#[pymodule]
fn tauq(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(loads, m)?)?;
    m.add_function(wrap_pyfunction!(load, m)?)?;
    m.add_function(wrap_pyfunction!(exec_tauqq, m)?)?;
    m.add_function(wrap_pyfunction!(dumps, m)?)?;
    m.add_function(wrap_pyfunction!(minify, m)?)?;
    m.add_function(wrap_pyfunction!(dump, m)?)?;
    
    // TBF functions
    m.add_function(wrap_pyfunction!(tbf_dumps, m)?)?;
    m.add_function(wrap_pyfunction!(tbf_loads, m)?)?;
    m.add_function(wrap_pyfunction!(tbf_dump, m)?)?;
    m.add_function(wrap_pyfunction!(tbf_load, m)?)?;
    m.add_function(wrap_pyfunction!(tbf_to_tauq, m)?)?;

    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add(
        "__doc__",
        "Tauq parser for Python - JSON for the AI Era (44% fewer tokens than JSON)",
    )?;

    Ok(())
}
