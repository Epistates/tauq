use serde_json::Value;
use tauq::tauq::parser::Parser;

#[test]
fn test_generic_object() {
    let input = r#"
    config {
        host "localhost"
        port 8080
        enabled true
    }
    "#;

    let mut parser = Parser::new(input);
    let result = parser.parse().unwrap();

    if let Value::Object(obj) = result {
        let config = obj["config"].as_object().expect("config should be object");
        assert_eq!(config["host"], "localhost");
        assert_eq!(config["port"], 8080.0);
        assert_eq!(config["enabled"], true);
    } else {
        panic!("Expected object, got {:?}", result);
    }
}

#[test]
fn test_nested_generic_object() {
    let input = r#"
    !def Log msg metadata
    !use Log
    "Error" { code 500 details "server error" }
    "#;

    let mut parser = Parser::new(input);
    let result = parser.parse().unwrap();

    // Parser unwraps single-item arrays
    let log = if let Value::Array(arr) = &result {
        &arr[0]
    } else {
        &result
    };

    assert_eq!(log["msg"], "Error");
    let meta = log["metadata"].as_object().expect("metadata is object");
    assert_eq!(meta["code"], 500.0);
    assert_eq!(meta["details"], "server error");
}
