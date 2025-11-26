use tauq;

#[test]
fn test_string_escapes() {
    let input = r#"
    message "Hello \"World\""
    path "C:\\Windows\\System32"
    newline "Line1\nLine2"
    "#;

    let result = tauq::compile_tauq(input);
    assert!(result.is_ok(), "Parsing failed: {:?}", result.err());

    let json = result.unwrap();
    // merged object
    assert_eq!(json["message"], "Hello \"World\"");
    assert_eq!(json["path"], "C:\\Windows\\System32");
    assert_eq!(json["newline"], "Line1\nLine2");
}
