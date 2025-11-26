use serde_json::Value;
use tauq::tauq::parser::Parser;

#[test]
fn test_schema_block() {
    let input = r#"
    !schemas
    User id name
    Product id price
    ---
    
    !use User
    1 Alice
    !use Product
    2 100
    "#;

    let mut parser = Parser::new(input);
    let result = parser.parse().unwrap();

    if let Value::Array(rows) = result {
        assert_eq!(rows.len(), 2);

        let u = &rows[0];
        assert_eq!(u["id"], 1.0);
        assert_eq!(u["name"], "Alice");

        let p = &rows[1];
        assert_eq!(p["id"], 2.0);
        assert_eq!(p["price"], 100.0);
    } else {
        panic!("Expected array of rows, got {:?}", result);
    }
}

#[test]
fn test_schema_block_with_delimiter() {
    // Test that TripleDash correctly ends the block
    let input = r#"
    !models
    A x
    B y
    ---
    !use A
    10
    "#;

    let mut parser = Parser::new(input);
    let result = parser.parse().unwrap();

    if let Value::Array(rows) = result {
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["x"], 10.0);
    } else if let Value::Object(obj) = result {
        assert_eq!(obj["x"], 10.0);
    } else {
        panic!("Expected row, got {:?}", result);
    }
}
