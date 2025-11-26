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
        {"id": 1.0, "name": "Alice", "role": "admin"},
        {"id": 2.0, "name": "Bob", "role": "user"}
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
        "port": 8080.0
    });

    assert_eq!(result, expected);
}

#[test]
fn test_tauq_minified() {
    let input = "!def U i n; 1 A; 2 B";
    let mut parser = Parser::new(input);
    let result = parser.parse().unwrap();

    let expected = json!([
        {"i": 1.0, "n": "A"},
        {"i": 2.0, "n": "B"}
    ]);

    assert_eq!(result, expected);
}
