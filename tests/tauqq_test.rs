use std::collections::HashMap;
use tauq::tauq::tauqq;

// ==================== Output Validation Tests ====================

#[test]
fn test_tauqq_emit_valid_output() {
    // Valid Tauq output should pass
    let input = r#"!emit echo "name Alice""#;
    let mut vars = HashMap::new();
    let result = tauqq::process(input, &mut vars, false);
    assert!(result.is_ok(), "Valid Tauq output should pass: {:?}", result);
}

#[test]
fn test_tauqq_emit_json_is_valid() {
    // JSON is valid Tauq (Tauq is a superset of JSON)
    let input = r#"!emit echo '{"name": "Alice"}'"#;
    let mut vars = HashMap::new();
    let result = tauqq::process(input, &mut vars, false);
    assert!(result.is_ok(), "JSON output should be valid (Tauq is a JSON superset): {:?}", result);
}

#[test]
fn test_tauqq_run_valid_output() {
    // Valid Tauq output from !run should pass
    let input = r#"!run sh {
echo "count 42"
echo "name Alice"
}"#;
    let mut vars = HashMap::new();
    let result = tauqq::process(input, &mut vars, false);
    assert!(result.is_ok(), "Valid Tauq output should pass: {:?}", result);
    let output = result.unwrap();
    assert!(output.contains("count 42"));
    assert!(output.contains("name Alice"));
}

#[test]
fn test_tauqq_run_json_is_valid() {
    // JSON is valid Tauq (Tauq is a superset of JSON)
    let input = r#"!run python3 {
import json
print(json.dumps({"name": "Alice", "age": 30}))
}"#;
    let mut vars = HashMap::new();
    let result = tauqq::process(input, &mut vars, false);
    assert!(result.is_ok(), "JSON output should be valid (Tauq is a JSON superset): {:?}", result);
}

#[test]
fn test_tauqq_run_empty_output_valid() {
    // Empty output should be valid
    let input = r#"!run sh {
# This outputs nothing
}"#;
    let mut vars = HashMap::new();
    let result = tauqq::process(input, &mut vars, false);
    assert!(result.is_ok(), "Empty output should be valid: {:?}", result);
}

#[test]
fn test_tauqq_emit_invalid_output_fails() {
    // Unbalanced braces should fail
    let input = r#"!emit echo "{ name Alice""#;
    let mut vars = HashMap::new();
    let result = tauqq::process(input, &mut vars, false);
    assert!(result.is_err(), "Unbalanced braces should fail validation");
    let err = result.unwrap_err();
    assert!(err.contains("!emit"), "Error should mention the directive");
}

#[test]
fn test_tauqq_run_invalid_syntax_fails() {
    // Invalid syntax should fail
    let input = r#"!run sh {
echo "[ unclosed"
}"#;
    let mut vars = HashMap::new();
    let result = tauqq::process(input, &mut vars, false);
    assert!(result.is_err(), "Unclosed bracket should fail validation");
    let err = result.unwrap_err();
    assert!(err.contains("!run"), "Error should mention the directive");
}

#[test]
fn test_tauqq_pipe_valid_output() {
    // Valid output after pipe should pass
    let input = r#"name Alice
age 30
!pipe grep name"#;
    let mut vars = HashMap::new();
    let result = tauqq::process(input, &mut vars, false);
    assert!(result.is_ok(), "Valid piped output should pass: {:?}", result);
}

// ==================== Original Tests ====================

#[test]
fn test_tauqq_emit() {
    // Output valid key-value pairs
    let input = r#"
!emit echo "count 1"
"#;
    let mut vars = HashMap::new();
    let result = tauqq::process(input, &mut vars, false).unwrap();
    assert!(result.contains("count 1"));
}

#[test]
fn test_tauqq_pipe() {
    // Use valid Tauq key-value pairs
    let input = r#"
name Alice
name Bob
!pipe grep Alice
"#;
    let mut vars = HashMap::new();
    let result = tauqq::process(input, &mut vars, false).unwrap();
    assert!(result.contains("name Alice"));
    assert!(!result.contains("name Bob"));
}

#[test]
fn test_tauqq_mixed() {
    // Top-down pipe means we must preserve directives if they are already emitted
    let input = r#"
!def U i n
!use U
 1 A
 2 B
!pipe grep -E "A|!"
"#;
    let mut vars = HashMap::new();
    let result = tauqq::process(input, &mut vars, false).unwrap();
    assert!(result.contains("!def U i n"));
    assert!(result.contains("!use U"));
    assert!(result.contains(" 1 A"));
    assert!(!result.contains(" 2 B"));
}

#[test]
fn test_tauqq_parse() {
    use serde_json::json;
    use tauq::tauq::Parser;

    let input = r#"
!def U i n
!use U
 1 A
 2 B
!pipe grep -E "A|!"
"#;
    let mut vars = HashMap::new();
    let processed = tauqq::process(input, &mut vars, false).unwrap();
    let mut parser = Parser::new(&processed);
    let result = parser.parse().unwrap();

    let expected = json!({"i": 1.0, "n": "A"});
    assert_eq!(result, expected);
}
