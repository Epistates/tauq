use serde_json::Value;
use tauq::tauq::parser::Parser;

#[test]
fn test_def_implies_use() {
    let input = r#"
    !def User id name
    1 "Alice"
    2 "Bob"
    "#;

    let mut parser = Parser::new(input);
    let result = parser.parse().unwrap();

    if let Value::Array(rows) = result {
        assert_eq!(rows.len(), 2);

        let u1 = &rows[0];
        assert_eq!(u1["id"], 1.0);
        assert_eq!(u1["name"], "Alice");

        let u2 = &rows[1];
        assert_eq!(u2["id"], 2.0);
        assert_eq!(u2["name"], "Bob");
    } else {
        panic!("Expected array of rows, got {:?}", result);
    }
}

#[test]
fn test_multiple_defs_auto_switch() {
    let input = r#"
    !def A val
    10
    !def B count
    20
    "#;

    let mut parser = Parser::new(input);
    let result = parser.parse().unwrap();

    if let Value::Array(rows) = result {
        assert_eq!(rows.len(), 2);

        // Row 1 should be type A (val)
        assert!(rows[0].as_object().unwrap().contains_key("val"));
        assert_eq!(rows[0]["val"], 10.0);

        // Row 2 should be type B (count)
        assert!(rows[1].as_object().unwrap().contains_key("count"));
        assert_eq!(rows[1]["count"], 20.0);
    } else {
        panic!("Expected array of rows, got {:?}", result);
    }
}

#[test]
fn test_explicit_switch_back() {
    let input = "!def A a; !def B b; !use A; 10";
    let json = tauq::compile_tauq(input).unwrap();
    // Single object result or array? If single row, compile_tauq returns Object.
    // If input has multiple rows, Array.
    // Here "!def A a; !def B b; !use A; 10" -> 1 row.
    assert_eq!(json["a"], 10.0);
    assert!(json.get("b").is_none());
}

#[test]
fn test_redundant_use() {
    let input = "!def A a; !use A; 10; !use A; 20";
    let json = tauq::compile_tauq(input).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["a"], 10.0);
    assert_eq!(arr[1]["a"], 20.0);
}

#[test]
fn test_mixed_implicit_explicit() {
    let input = "!def A a; 10; !def B b; 20; !use A; 30";
    let json = tauq::compile_tauq(input).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["a"], 10.0);
    assert_eq!(arr[1]["b"], 20.0);
    assert_eq!(arr[2]["a"], 30.0);
}

#[test]
fn test_undefined_schema_error() {
    let input = "!use Ghost; 1 2 3";
    let result = tauq::compile_tauq(input);
    // Should return an error for undefined schema
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("undefined schema"));
}
