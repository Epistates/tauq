use serde_json::json;
use tauq::tauq::Parser;

#[test]
fn test_tauq_inline_list() {
    let input = r#"
!def User id tags
!use User
1 [ "admin" "staff" ]
2 [ "user" ]
"#;
    let mut parser = Parser::new(input);
    let result = parser.parse().unwrap();

    let expected = json!([
        {"id": 1, "tags": ["admin", "staff"]},
        {"id": 2, "tags": ["user"]}
    ]);
    assert_eq!(result, expected);
}

#[test]
fn test_tauq_nested_list() {
    // This test was incomplete in original, fixing it to be meaningful
    let input = r#"
!def Matrix values
!use Matrix
[ [ 1 2 ] [ 3 4 ] ]
"#;
    let mut parser = Parser::new(input);
    let result = parser.parse().unwrap();

    let expected = json!({
        "values": [[1, 2], [3, 4]]
    });
    assert_eq!(result, expected);
}

#[test]
fn test_tauq_nested_list_corrected() {
    let input = r#"
!def Data values
!use Data
[ [ 1 2 ] [ 3 4 ] ]
"#;
    let mut parser = Parser::new(input);
    let result = parser.parse().unwrap();

    let expected = json!({
        "values": [[1, 2], [3, 4]]
    });
    assert_eq!(result, expected);
}
