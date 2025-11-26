use std::collections::HashMap;
use std::io::Write;
use tauq::tauq::tauqq::{self, ProcessConfig};

#[test]
fn test_tauqq_env() {
    unsafe { std::env::set_var("TEST_VAR", "hello") };

    let input = "!env TEST_VAR";
    let mut vars = HashMap::new();
    let result = tauqq::process(input, &mut vars, false).unwrap();
    assert!(result.contains("\"hello\""));
}

#[test]
fn test_tauqq_read() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    write!(file, "file content").unwrap();
    let path = file.path().to_str().unwrap();

    let input = format!("!read \"{}\"", path);
    let mut vars = HashMap::new();

    // Use explicit config with no base_dir restriction for testing
    let config = ProcessConfig {
        base_dir: None,
        safe_mode: false,
    };
    let result = tauqq::process_with_config(&input, &mut vars, &config).unwrap();
    assert!(result.contains("\"file content\""));
}

#[test]
fn test_path_traversal_blocked() {
    // Test that path traversal is blocked when base_dir is set
    let config = ProcessConfig {
        base_dir: Some(std::path::PathBuf::from("/tmp/tauq_test_sandbox")),
        safe_mode: false,
    };

    let input = "!read \"../../etc/passwd\"";
    let mut vars = HashMap::new();
    let result = tauqq::process_with_config(&input, &mut vars, &config);

    // Should fail with path traversal or resolution error
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("escapes base directory") || err.contains("Cannot resolve"),
        "Expected path traversal error, got: {}",
        err
    );
}
