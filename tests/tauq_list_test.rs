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
        {"id": 1.0, "tags": ["admin", "staff"]},
        {"id": 2.0, "tags": ["user"]}
    ]);
    assert_eq!(result, expected);
}

#[test]
fn test_tauq_nested_list() {
    let input = r#"
!def Matrix row
!use Matrix
1 [ [ 1 2 ] [ 3 4 ] ]
"#;
    let mut parser = Parser::new(input);
    let result = parser.parse().unwrap();

    let expected = json!({
        "row": 1.0,
        "row": [[1.0, 2.0], [3.0, 4.0]] // Wait, field name is 'row', value is the list?
        // Ah, !def Matrix row So first field is row.
        //  1 .. -> 1 is 'row'.
        // The list is extra? No, !def Matrix row Only 1 field.
        // So  1 [ .. ] is 2 values Error?
        // Let's redefine.
    });
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
        "values": [[1.0, 2.0], [3.0, 4.0]]
    });
    assert_eq!(result, expected);
}
