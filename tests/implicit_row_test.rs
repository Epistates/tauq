use serde_json::Value;
use tauq::tauq::parser::Parser;

#[test]
fn test_implicit_rows() {
    let input = r#"
    !def User name age
    !use User
    
    Alice 30
    Bob 40
    "#;

    let mut parser = Parser::new(input);
    let result = parser.parse().unwrap();

    if let Value::Array(rows) = result {
        assert_eq!(rows.len(), 2);

        let row1 = &rows[0];
        assert_eq!(row1["name"], "Alice");
        assert_eq!(row1["age"], 30.0);

        let row2 = &rows[1];
        assert_eq!(row2["name"], "Bob");
        assert_eq!(row2["age"], 40.0);
    } else {
        panic!("Expected array of rows, got {:?}", result);
    }
}

#[test]
fn test_implicit_nested_rows() {
    let input = r#"
    !def Point x y
    !def Shape points:[Point]
    !use Shape
    
    [
        10 20
        30 40
    ]
    "#;

    let mut parser = Parser::new(input);
    let result = parser.parse().unwrap();

    if let Value::Object(obj) = result {
        let points = obj["points"].as_array().expect("points should be array");
        assert_eq!(points.len(), 2);
        assert_eq!(points[0]["x"], 10.0);
        assert_eq!(points[0]["y"], 20.0);
    } else if let Value::Array(arr) = result {
        let obj = &arr[0];
        let points = obj["points"].as_array().expect("points should be array");
        assert_eq!(points.len(), 2);
        assert_eq!(points[0]["x"], 10.0);
        assert_eq!(points[0]["y"], 20.0);
    } else {
        panic!("Expected object or array, got {:?}", result);
    }
}
