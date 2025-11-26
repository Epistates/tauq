use tauq;

#[test]
fn test_named_args() {
    let input = r#"                                                                                                                                                                   
        !def User id name role                                                                                                                                                            
        !use User                                                                                                                                                                         
        id:1 name:"Alice" role:admin                                                                                                                                                    
        name:"Bob" id:2 role:user                                                                                                                                                       
        3 "Carol" role:admin                                                                                                                                                            
        "#;
    let result = tauq::compile_tauq(input).expect("Failed to parse");
    let arr = result.as_array().expect("Expected array");

    // Row 1: All named
    assert_eq!(arr[0]["id"].as_f64(), Some(1.0));
    assert_eq!(arr[0]["name"], "Alice");
    assert_eq!(arr[0]["role"], "admin");

    // Row 2: Mixed order
    assert_eq!(arr[1]["id"].as_f64(), Some(2.0));
    assert_eq!(arr[1]["name"], "Bob");
    assert_eq!(arr[1]["role"], "user");

    // Row 3: Mixed positional and named
    assert_eq!(arr[2]["id"].as_f64(), Some(3.0));
    assert_eq!(arr[2]["name"], "Carol");
    assert_eq!(arr[2]["role"], "admin");
}
