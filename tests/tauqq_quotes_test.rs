use std::collections::HashMap;
use tauq::tauq::tauqq;

#[test]
fn test_tauqq_quotes() {
    let input = r#"
!emit echo "Hello World"
"#;
    let mut vars = HashMap::new();
    let result = tauqq::process(input, &mut vars, false).unwrap();
    assert!(result.contains("Hello World"));
}

#[test]
fn test_tauqq_quotes_single() {
    let input = r#"
!emit echo 'Hello World'
"#;
    let mut vars = HashMap::new();
    let result = tauqq::process(input, &mut vars, false).unwrap();
    assert!(result.contains("Hello World"));
}
