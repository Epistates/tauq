// Comprehensive integration tests validating tauq format correctness at scale
// These tests mirror the LLM benchmark datasets to ensure format fidelity

use serde_json::json;
use tauq::{compile_tauq, compile_tauqq_safe};

// ============================================================================
// Employee Dataset Tests (matching LLM benchmark structure)
// ============================================================================

/// Generate employee record as JSON
fn make_employee(id: i32, name: &str, age: i32, city: &str, dept: &str, salary: i32, exp: i32, projects: i32) -> serde_json::Value {
    json!({
        "id": id,
        "name": name,
        "age": age,
        "city": city,
        "department": dept,
        "salary": salary,
        "experience": exp,
        "project_count": projects
    })
}

/// Generate TAUQ input for employees using !def schema
fn employees_to_tauq(employees: &[serde_json::Value]) -> String {
    let mut lines = vec!["!def Employee id name age city department salary experience project_count".to_string()];
    lines.push("!use Employee".to_string());

    for emp in employees {
        let name = emp["name"].as_str().unwrap();
        let city = emp["city"].as_str().unwrap();
        let dept = emp["department"].as_str().unwrap();

        lines.push(format!(
            "{} \"{}\" {} {} {} {} {} {}",
            emp["id"].as_i64().unwrap(),
            name,
            emp["age"].as_i64().unwrap(),
            city,
            dept,
            emp["salary"].as_i64().unwrap(),
            emp["experience"].as_i64().unwrap(),
            emp["project_count"].as_i64().unwrap()
        ));
    }

    lines.join("\n")
}

#[test]
fn test_single_employee_roundtrip() {
    let employees = vec![
        make_employee(1, "Alice A001", 30, "NYC", "Engineering", 85000, 5, 10),
    ];

    let tauq = employees_to_tauq(&employees);
    let result = compile_tauq(&tauq).expect("Failed to parse tauq");

    // Single row should be an object, not array
    assert_eq!(result["id"], 1);
    assert_eq!(result["name"], "Alice A001");
    assert_eq!(result["age"], 30);
    assert_eq!(result["city"], "NYC");
    assert_eq!(result["department"], "Engineering");
    assert_eq!(result["salary"], 85000);
    assert_eq!(result["experience"], 5);
    assert_eq!(result["project_count"], 10);
}

#[test]
fn test_multiple_employees_roundtrip() {
    let employees = vec![
        make_employee(1, "Alice A001", 30, "NYC", "Engineering", 85000, 5, 10),
        make_employee(2, "Bob B002", 28, "LA", "Sales", 72000, 3, 8),
        make_employee(3, "Carol C003", 35, "Chicago", "Marketing", 92000, 10, 15),
    ];

    let tauq = employees_to_tauq(&employees);
    let result = compile_tauq(&tauq).expect("Failed to parse tauq");

    let arr = result.as_array().expect("Expected array");
    assert_eq!(arr.len(), 3);

    // Check first employee
    assert_eq!(arr[0]["id"], 1);
    assert_eq!(arr[0]["name"], "Alice A001");
    assert_eq!(arr[0]["department"], "Engineering");

    // Check second employee
    assert_eq!(arr[1]["id"], 2);
    assert_eq!(arr[1]["name"], "Bob B002");
    assert_eq!(arr[1]["city"], "LA");

    // Check third employee
    assert_eq!(arr[2]["id"], 3);
    assert_eq!(arr[2]["salary"], 92000);
    assert_eq!(arr[2]["experience"], 10);
}

#[test]
fn test_large_employee_dataset() {
    // Generate 100 employees to stress test parsing
    let mut employees = Vec::new();
    let names = ["Alice", "Bob", "Carol", "Dave", "Eve", "Frank", "Grace", "Hank"];
    let cities = ["NYC", "LA", "Chicago", "Houston", "Phoenix"];
    let depts = ["Engineering", "Sales", "Marketing", "HR", "Finance"];

    for i in 0..100 {
        let name = format!("{} {}{:03}", names[i % names.len()], (65 + (i / 26)) as u8 as char, i);
        employees.push(make_employee(
            (i + 1) as i32,
            &name,
            22 + (i % 44) as i32,
            cities[i % cities.len()],
            depts[i % depts.len()],
            40000 + (i * 1000) as i32,
            (i % 30) as i32,
            1 + (i % 50) as i32,
        ));
    }

    let tauq = employees_to_tauq(&employees);
    let result = compile_tauq(&tauq).expect("Failed to parse large dataset");

    let arr = result.as_array().expect("Expected array");
    assert_eq!(arr.len(), 100);

    // Verify first and last entries
    assert_eq!(arr[0]["id"], 1);
    assert_eq!(arr[99]["id"], 100);
}

// ============================================================================
// Edge Cases and Special Characters
// ============================================================================

#[test]
fn test_names_with_special_characters() {
    let tauq = r#" 
!def Employee id name city
!use Employee
1 "Alice O'Brien" NYC
2 "Bob \"Bobby\" Smith" LA
3 "Carol Van-Der-Berg" Chicago
"#;

    let result = compile_tauq(tauq).expect("Failed to parse");
    let arr = result.as_array().unwrap();

    assert_eq!(arr[0]["name"], "Alice O'Brien");
    assert_eq!(arr[1]["name"], "Bob \"Bobby\" Smith");
    assert_eq!(arr[2]["name"], "Carol Van-Der-Berg");
}

#[test]
fn test_unicode_names() {
    let tauq = r#" 
!def Employee id name city
!use Employee
1 "José García" "México"
2 "李明" "北京"
3 "Müller" "München"
"#;

    let result = compile_tauq(tauq).expect("Failed to parse unicode");
    let arr = result.as_array().unwrap();

    assert_eq!(arr[0]["name"], "José García");
    assert_eq!(arr[0]["city"], "México");
    assert_eq!(arr[1]["name"], "李明");
    assert_eq!(arr[1]["city"], "北京");
    assert_eq!(arr[2]["name"], "Müller");
    assert_eq!(arr[2]["city"], "München");
}

#[test]
#[allow(clippy::approx_constant)]
fn test_numeric_edge_cases() {
    let tauq = r#"
!def Data id value
!use Data
1 0
2 -1
3 999999999
4 3.14159
5 1e10
6 -1.5e-3
"#;

    let result = compile_tauq(tauq).expect("Failed to parse numbers");
    let arr = result.as_array().unwrap();

    assert_eq!(arr[0]["value"], 0);
    assert_eq!(arr[1]["value"], -1);
    assert_eq!(arr[2]["value"], 999999999);
    assert!((arr[3]["value"].as_f64().unwrap() - 3.14159).abs() < 0.0001);
    // 1e10 is parsed as f64 because it uses scientific notation
    assert_eq!(arr[4]["value"], 1.0e10);
    assert!((arr[5]["value"].as_f64().unwrap() - (-0.0015)).abs() < 0.0001);
}
// ============================================================================
// Schema Variations
// ============================================================================

#[test]
fn test_multiple_schemas() {
    let tauq = r#"
!def Employee id name department
!def Product sku name price
---
employees [
    !use Employee
    1 Alice Engineering
    2 Bob Sales
]
products [
    !use Product
    "SKU001" "Widget" 29.99
    "SKU002" "Gadget" 49.99
]
"#;

    let result = compile_tauq(tauq).expect("Failed to parse multiple schemas");

    let employees = result["employees"].as_array().unwrap();
    assert_eq!(employees.len(), 2);
    assert_eq!(employees[0]["name"], "Alice");
    assert_eq!(employees[1]["department"], "Sales");

    let products = result["products"].as_array().unwrap();
    assert_eq!(products.len(), 2);
    assert_eq!(products[0]["sku"], "SKU001");
    assert_eq!(products[1]["price"], 49.99);
}
#[test]
fn test_nested_schema() {
    let tauq = r#" 
!def Address street city
!def Employee id name addr:Address
!use Employee
1 Alice { "123 Main St" "New York" }
2 Bob { "456 Oak Ave" "Los Angeles" }
"#;

    let result = compile_tauq(tauq).expect("Failed to parse nested schema");
    let arr = result.as_array().unwrap();

    assert_eq!(arr[0]["addr"]["street"], "123 Main St");
    assert_eq!(arr[0]["addr"]["city"], "New York");
    assert_eq!(arr[1]["addr"]["street"], "456 Oak Ave");
    assert_eq!(arr[1]["addr"]["city"], "Los Angeles");
}

// ============================================================================
// Minified Format Tests
// ============================================================================

#[test]
fn test_minified_semicolon_separator() {
    let tauq = "!def E id name age; 1 Alice 30; 2 Bob 25; 3 Carol 35";
    let result = compile_tauq(tauq).expect("Failed to parse minified");

    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["id"], 1);
    assert_eq!(arr[0]["name"], "Alice");
    assert_eq!(arr[2]["age"], 35);
}

#[test]
fn test_minified_with_strings() {
    let tauq = r#"!def E name email; "Alice" "alice@test.com"; "Bob" "bob@test.com""#;
    let result = compile_tauq(tauq).expect("Failed to parse minified with strings");

    let arr = result.as_array().expect("Result should be an array");
    assert_eq!(arr[0]["name"], "Alice");
    assert_eq!(arr[0]["email"], "alice@test.com");
    assert_eq!(arr[1]["name"], "Bob");
}

// ============================================================================
// Key-Value (Map) Format Tests
// ============================================================================

#[test]
fn test_config_style_map() {
    let tauq = r#" 
host localhost
port 8080
debug true
version "1.0.0"
timeout 30
"#;

    let result = compile_tauq(tauq).expect("Failed to parse config");

    assert_eq!(result["host"], "localhost");
    assert_eq!(result["port"], 8080);
    assert_eq!(result["debug"], true);
    assert_eq!(result["version"], "1.0.0");
    assert_eq!(result["timeout"], 30);
}

#[test]
fn test_map_with_arrays() {
    let tauq = r#" 
name "MyService"
ports [8080 8443 9090]
features [api websocket metrics]
"#;

    let result = compile_tauq(tauq).expect("Failed to parse map with arrays");

    assert_eq!(result["name"], "MyService");
    assert_eq!(result["ports"], json!([8080, 8443, 9090]));
    assert_eq!(result["features"], json!(["api", "websocket", "metrics"]));
}

// ============================================================================
// TauqQ Preprocessor Tests (Safe Mode)
// ============================================================================

#[test]
fn test_tauqq_basic_preprocessor() {
    // TauqQ processes !def and !use as a preprocessor
    let tauq = r#" 
!def Employee id name
!use Employee
1 Alice
2 Bob
"#;

    let result = compile_tauqq_safe(tauq).expect("Failed to process tauqq");
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["name"], "Alice");
    assert_eq!(arr[1]["name"], "Bob");
}

#[test]
fn test_tauqq_with_triple_dash() {
    // TauqQ handles --- separator for document sections
    let tauq = r#" 
!def Config key value
---
settings [
    !use Config
    host localhost
    port 8080
]
"#;

    let result = compile_tauqq_safe(tauq).expect("Failed to process tauqq");
    let settings = result["settings"].as_array().unwrap();
    assert_eq!(settings.len(), 2);
    assert_eq!(settings[0]["key"], "host");
    assert_eq!(settings[0]["value"], "localhost");
    assert_eq!(settings[1]["key"], "port");
    assert_eq!(settings[1]["value"], 8080);
}

// ============================================================================
// Stress Tests
// ============================================================================

#[test]
fn test_many_fields() {
    // 20 fields per record
    let mut fields = Vec::new();
    for i in 1..=20 {
        fields.push(format!("f{}", i));
    }

    let def_line = format!("!def R {}", fields.join(" "));
    let use_line = "!use R";
    let data_line = (1..=20).map(|i| i.to_string()).collect::<Vec<_>>().join(" ");

    let tauq = format!("{}\n{}\n{}", def_line, use_line, data_line);
    let result = compile_tauq(&tauq).expect("Failed to parse many fields");

    assert_eq!(result["f1"], 1);
    assert_eq!(result["f10"], 10);
    assert_eq!(result["f20"], 20);
}

#[test]
fn test_deeply_nested_arrays() {
    let tauq = "matrix [[[[1 2] [3 4]] [[5 6] [7 8]]] [[[9 10] [11 12]] [[13 14] [15 16]]]]";
    let result = compile_tauq(tauq).expect("Failed to parse deep nesting");

    // Navigate to deeply nested value
    let val = &result["matrix"][0][0][0][0];
    assert_eq!(*val, 1);

    let val2 = &result["matrix"][1][1][1][1];
    assert_eq!(*val2, 16);
}

#[test]
fn test_long_string_values() {
    let long_str = "x".repeat(1000);
    let tauq = format!(r#"message "{}""#, long_str);
    let result = compile_tauq(&tauq).expect("Failed to parse long string");

    let msg = result["message"].as_str().expect("message should be a string");
    assert_eq!(msg.len(), 1000);
}

// ============================================================================
// Comment Handling
// ============================================================================

#[test]
fn test_comments_preserved_behavior() {
    let tauq = r#" 
# This is a header comment
host localhost # inline comment
# Another comment
port 8080
# Final comment
"#;

    let result = compile_tauq(tauq).expect("Failed to parse with comments");

    assert_eq!(result["host"], "localhost");
    assert_eq!(result["port"], 8080);
}

// ============================================================================
// Error Cases (should fail gracefully)
// ============================================================================

#[test]
fn test_invalid_use_without_def() {
    let tauq = "!use NonExistent\n1 2 3";
    let result = compile_tauq(tauq);
    assert!(result.is_err(), "Should fail with undefined schema");
}

#[test]
fn test_unclosed_string() {
    let tauq = r#"name \"Alice"#;  // Missing closing quote
    // Parser should either handle gracefully or return error
    // This is more of a parse robustness test
    let _result = compile_tauq(tauq);
    // Just checking it doesn't panic
}

#[test]
fn test_empty_input() {
    let result = compile_tauq("").expect("Empty input should parse");
    assert_eq!(result, json!([]));
}

#[test]
fn test_whitespace_only() {
    let result = compile_tauq("   \n\t\n   ").expect("Whitespace should parse");
    assert_eq!(result, json!([]));
}

// ============================================================================
// Benchmark Data Format Validation
// These tests ensure the LLM benchmark generates valid, parseable data
// ============================================================================

#[test]
fn test_benchmark_employee_format() {
    // This is the exact format the LLM benchmark generates
    let tauq = r#" 
!def Employee id name age city department salary experience project_count
!use Employee
1 "Alice A001" 30 NYC Engineering 85000 5 10
2 "Bob B002" 28 LA Sales 72000 3 8
3 "Carol C003" 35 Chicago Marketing 92000 10 15
4 "Dave D004" 45 Houston Finance 110000 20 25
5 "Eve E005" 22 Phoenix HR 45000 0 2
"#;

    let result = compile_tauq(tauq).expect("Benchmark format should parse");
    let arr = result.as_array().expect("Should be array");

    assert_eq!(arr.len(), 5);

    // Verify all 8 fields are present and correct
    let emp1 = &arr[0];
    assert_eq!(emp1["id"], 1);
    assert_eq!(emp1["name"], "Alice A001");
    assert_eq!(emp1["age"], 30);
    assert_eq!(emp1["city"], "NYC");
    assert_eq!(emp1["department"], "Engineering");
    assert_eq!(emp1["salary"], 85000);
    assert_eq!(emp1["experience"], 5);
    assert_eq!(emp1["project_count"], 10);

    // Verify edge case employee (0 experience)
    let emp5 = &arr[4];
    assert_eq!(emp5["experience"], 0);
}
