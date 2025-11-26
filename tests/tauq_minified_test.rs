use serde_json::json;
use tauq::tauq::Parser;

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
