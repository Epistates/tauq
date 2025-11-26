use std::collections::HashMap;
use std::io::Write;
use tauq::tauq::tauqq::{self, ProcessConfig};

#[test]
fn test_tauqq_json_directive() {
    // 1. Create a temp JSON file
    let mut temp_file = tempfile::NamedTempFile::new().unwrap();
    let json_content = r#"{
        "name": "Test",
        "values": [1, 2, 3]
    }"#;
    write!(temp_file, "{}", json_content).unwrap();
    temp_file.as_file().sync_all().unwrap(); // Ensure written to disk
    let temp_path = temp_file.path().to_str().unwrap().to_string(); // Keep path alive

    // Debug: read it back
    let content = std::fs::read_to_string(&temp_path).unwrap();
    println!("DEBUG: File content: '{}'", content);

    // 2. Create TauqQ source that uses !json
    let input = format!("!json \"{}\"", temp_path);

    // 3. Process with no base_dir restriction for testing with temp files
    let mut vars = HashMap::new();
    let config = ProcessConfig {
        base_dir: None,
        safe_mode: false,
    };
    let result = tauqq::process_with_config(&input, &mut vars, &config).unwrap();

    // 4. Verify result is valid Tauq and contains data
    println!("DEBUG: Result Tauq: '{}'", result);

    // 5. Parse the result to verify it's valid Tauq
    let mut parser = tauq::Parser::new(&result);
    let parsed_json = parser.parse().unwrap();

    assert_eq!(parsed_json["name"], "Test");
    assert_eq!(parsed_json["values"][0], 1.0);
}
