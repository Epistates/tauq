// Format roundtrip tests: JSON -> Tauq -> JSON
// Ensures the formatter produces output that correctly parses back to the original data

use serde_json::json;
use tauq::{compile_tauq, format_to_tauq};

/// Normalize JSON for comparison (convert integer floats back to integers when appropriate)
#[allow(dead_code)]
fn normalize_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Number(n) => {
            // Keep as-is for comparison
            serde_json::Value::Number(n.clone())
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(normalize_json).collect())
        }
        serde_json::Value::Object(obj) => serde_json::Value::Object(
            obj.iter()
                .map(|(k, v)| (k.clone(), normalize_json(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Compare two JSON values for semantic equality (allows float/int differences)
fn json_equivalent(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    match (a, b) {
        (serde_json::Value::Number(n1), serde_json::Value::Number(n2)) => {
            // Compare as f64 to handle int/float differences
            let f1 = n1.as_f64().unwrap_or(f64::NAN);
            let f2 = n2.as_f64().unwrap_or(f64::NAN);
            (f1 - f2).abs() < 0.0001 || (f1.is_nan() && f2.is_nan())
        }
        (serde_json::Value::Array(arr1), serde_json::Value::Array(arr2)) => {
            arr1.len() == arr2.len()
                && arr1
                    .iter()
                    .zip(arr2.iter())
                    .all(|(v1, v2)| json_equivalent(v1, v2))
        }
        (serde_json::Value::Object(obj1), serde_json::Value::Object(obj2)) => {
            obj1.len() == obj2.len()
                && obj1.keys().all(|k| {
                    obj2.get(k)
                        .map(|v2| json_equivalent(&obj1[k], v2))
                        .unwrap_or(false)
                })
        }
        (serde_json::Value::String(s1), serde_json::Value::String(s2)) => s1 == s2,
        (serde_json::Value::Bool(b1), serde_json::Value::Bool(b2)) => b1 == b2,
        (serde_json::Value::Null, serde_json::Value::Null) => true,
        _ => false,
    }
}

/// Test roundtrip: JSON -> Tauq -> JSON
fn roundtrip(original: &serde_json::Value) -> Result<serde_json::Value, String> {
    let tauq_str = format_to_tauq(original);
    compile_tauq(&tauq_str).map_err(|e| format!("Parse error: {}", e))
}

// ============================================================================
// Simple Object Roundtrip Tests
// ============================================================================

#[test]
fn test_roundtrip_simple_object() {
    let original = json!({
        "name": "Alice",
        "age": 30,
        "active": true
    });

    let result = roundtrip(&original).expect("Roundtrip failed");
    assert!(
        json_equivalent(&original, &result),
        "Roundtrip mismatch:\nOriginal: {:?}\nResult: {:?}",
        original,
        result
    );
}

#[test]
fn test_roundtrip_nested_object() {
    let original = json!({
        "user": {
            "name": "Bob",
            "address": {
                "city": "NYC",
                "zip": "10001"
            }
        }
    });

    let result = roundtrip(&original).expect("Roundtrip failed");
    assert!(json_equivalent(&original, &result));
}

// ============================================================================
// Array Roundtrip Tests
// ============================================================================

#[test]
fn test_roundtrip_array_of_objects() {
    let original = json!([
        {"id": 1, "name": "Alice"},
        {"id": 2, "name": "Bob"},
        {"id": 3, "name": "Carol"}
    ]);

    let result = roundtrip(&original).expect("Roundtrip failed");
    assert!(json_equivalent(&original, &result));
}

#[test]
fn test_roundtrip_array_of_primitives() {
    let original = json!({
        "numbers": [1, 2, 3, 4, 5],
        "strings": ["a", "b", "c"],
        "mixed": [1, "two", true, null]
    });

    let result = roundtrip(&original).expect("Roundtrip failed");
    assert!(json_equivalent(&original, &result));
}

// ============================================================================
// Employee Dataset Roundtrip (matching LLM benchmark)
// ============================================================================

#[test]
fn test_roundtrip_employee_dataset() {
    let original = json!([
        {
            "id": 1,
            "name": "Alice A001",
            "age": 30,
            "city": "NYC",
            "department": "Engineering",
            "salary": 85000,
            "experience": 5,
            "project_count": 10
        },
        {
            "id": 2,
            "name": "Bob B002",
            "age": 28,
            "city": "LA",
            "department": "Sales",
            "salary": 72000,
            "experience": 3,
            "project_count": 8
        }
    ]);

    let result = roundtrip(&original).expect("Roundtrip failed");
    assert!(json_equivalent(&original, &result));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_roundtrip_empty_array() {
    let original = json!({ "items": [] });
    let result = roundtrip(&original).expect("Roundtrip failed");
    assert!(json_equivalent(&original, &result));
}

#[test]
fn test_roundtrip_null_values() {
    let original = json!({
        "value": null,
        "data": {"inner": null}
    });

    let result = roundtrip(&original).expect("Roundtrip failed");
    assert!(json_equivalent(&original, &result));
}

#[test]
fn test_roundtrip_unicode_strings() {
    let original = json!({
        "greeting": "Hello 世界",
        "emoji": "🚀 Tauq",
        "accent": "José García"
    });

    let result = roundtrip(&original).expect("Roundtrip failed");
    assert!(json_equivalent(&original, &result));
}

#[test]
fn test_roundtrip_special_characters() {
    let original = json!({
        "quote": "He said \"hello\"",
        "backslash": "path\\to\\file",
        "newline": "line1\nline2"
    });

    let result = roundtrip(&original).expect("Roundtrip failed");
    assert!(json_equivalent(&original, &result));
}

#[test]
fn test_roundtrip_numeric_precision() {
    let original = json!({
        "integer": 42,
        "float": 1.23456,
        "scientific": 1e10,
        "negative": -999,
        "zero": 0
    });

    let result = roundtrip(&original).expect("Roundtrip failed");
    assert!(json_equivalent(&original, &result));
}

// ============================================================================
// Large Dataset Roundtrip
// ============================================================================

#[test]
fn test_roundtrip_large_dataset() {
    let cities = ["NYC", "LA", "Chicago", "Houston", "Phoenix"];
    let departments = ["Engineering", "Sales", "Marketing"];

    // Generate 50 employee records
    let employees: Vec<serde_json::Value> = (0..50)
        .map(|i| {
            json!({
                "id": i + 1,
                "name": format!("Employee{}", i + 1),
                "age": 22 + (i % 44),
                "city": cities[i % 5],
                "department": departments[i % 3],
                "salary": 40000 + (i * 1000),
                "experience": i % 30,
                "project_count": 1 + (i % 50)
            })
        })
        .collect();

    let original = serde_json::Value::Array(employees);
    let result = roundtrip(&original).expect("Roundtrip failed");

    assert!(json_equivalent(&original, &result));
}

// ============================================================================
// Nested Array Roundtrip
// ============================================================================

#[test]
fn test_roundtrip_nested_arrays() {
    let original = json!({
        "matrix": [[1, 2, 3], [4, 5, 6], [7, 8, 9]],
        "deep": [[[1, 2], [3, 4]], [[5, 6], [7, 8]]]
    });

    let result = roundtrip(&original).expect("Roundtrip failed");
    assert!(json_equivalent(&original, &result));
}

// ============================================================================
// Complex Real-World Structures
// ============================================================================

#[test]
fn test_roundtrip_config_structure() {
    let original = json!({
        "app_name": "MyService",
        "version": "1.0.0",
        "debug": true,
        "port": 8080,
        "features": ["api", "websocket", "metrics"],
        "database": {
            "host": "localhost",
            "port": 5432,
            "name": "mydb"
        }
    });

    let result = roundtrip(&original).expect("Roundtrip failed");
    assert!(json_equivalent(&original, &result));
}

#[test]
fn test_roundtrip_order_with_nested_items() {
    let original = json!({
        "order": {
            "id": "ORD-001",
            "customer": {
                "id": 123,
                "name": "Alice"
            },
            "items": [
                {"sku": "SKU-001", "name": "Widget", "qty": 2, "price": 29.99},
                {"sku": "SKU-002", "name": "Gadget", "qty": 1, "price": 49.99}
            ],
            "total": 109.97
        }
    });

    let result = roundtrip(&original).expect("Roundtrip failed");
    assert!(json_equivalent(&original, &result));
}

// ============================================================================
// Format Preservation Tests
// ============================================================================

#[test]
fn test_tauq_format_readable() {
    let original = json!([
        {"id": 1, "name": "Alice", "role": "admin"},
        {"id": 2, "name": "Bob", "role": "user"}
    ]);

    let tauq_str = format_to_tauq(&original);

    // Verify the output is readable and uses schema
    assert!(
        tauq_str.contains("!def"),
        "Should use schema definition for uniform arrays"
    );
    assert!(tauq_str.contains("Alice"), "Should contain data values");
    assert!(tauq_str.contains("Bob"), "Should contain data values");
}

#[test]
fn test_tauq_format_key_value() {
    let original = json!({
        "host": "localhost",
        "port": 8080
    });

    let tauq_str = format_to_tauq(&original);

    // Simple key-value should not use schema
    assert!(tauq_str.contains("host"), "Should contain key names");
    assert!(tauq_str.contains("localhost"), "Should contain values");
}
