use serde_json::json;
use tauq::tauq::Parser;

#[test]
fn test_hyphenated_identifiers() {
    let input = r#"
!def Service name-id
!use Service
my-service-1
"#;
    let json = tauq::compile_tauq(input).unwrap();
    // Single row returns an Object, not an Array
    assert_eq!(json["name-id"], "my-service-1");
}

#[test]
fn test_basic_types() {
    let input = r#"
!def User id name role
!use User
1 "Alice" admin
2 "Bob" user
"#;
    let mut parser = Parser::new(input);
    let result = parser.parse().unwrap();

    let expected = json!([
        {"id": 1, "name": "Alice", "role": "admin"},
        {"id": 2, "name": "Bob", "role": "user"}
    ]);

    assert_eq!(result, expected);
}

#[test]
fn test_tauq_map() {
    let input = r#"
host localhost
port 8080
"#;
    let mut parser = Parser::new(input);
    let result = parser.parse().unwrap();

    // Auto-merged into single object
    let expected = json!({
        "host": "localhost",
        "port": 8080
    });

    assert_eq!(result, expected);
}

#[test]
fn test_tauq_minified() {
    let input = "!def U i n; 1 A; 2 B";
    let mut parser = Parser::new(input);
    let result = parser.parse().unwrap();

    let expected = json!([
        {"i": 1, "n": "A"},
        {"i": 2, "n": "B"}
    ]);

    assert_eq!(result, expected);
}

#[test]
fn test_large_integers() {
    let input = r#"
!def Data id big_id ubig_id float_val
!use Data
1 9223372036854775807 18446744073709551615 1.5
"#;
    let mut parser = Parser::new(input);
    let result = parser.parse().unwrap();

    // Verify types using direct Value inspection
    if let serde_json::Value::Object(obj) = result {
        assert!(obj["id"].is_i64());
        assert_eq!(obj["id"], 1);

        assert!(obj["big_id"].is_i64());
        assert_eq!(obj["big_id"].as_i64(), Some(9223372036854775807));

        assert!(obj["ubig_id"].is_u64());
        assert_eq!(obj["ubig_id"].as_u64(), Some(18446744073709551615));

        assert!(obj["float_val"].is_f64());
        assert_eq!(obj["float_val"], 1.5);
    } else {
        panic!("Expected object result");
    }
}
