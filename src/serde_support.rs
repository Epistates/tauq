// Serde Support for Tauq
//
// Enables deserializing Tauq directly into Rust structs:
//
// ```rust
// use serde::Deserialize;
//
// #[derive(Deserialize)]
// struct Config {
//     workers: u32,
//     database: DatabaseConfig,
// }
//
// let config: Config = tauq::from_str(tauq_source)?;
// ```

use crate::{compile_tauq, RhoError};
use serde::de::DeserializeOwned;
use std::path::Path;

/// Deserialize Tauq from a string into a type T
///
/// # Example
///
/// ```
/// use serde::Deserialize;
/// use tauq::from_str;
///
/// #[derive(Deserialize, Debug, PartialEq)]
/// struct Config {
///     workers: f64,  // Numbers in Tauq are f64
///     timeout: f64,
/// }
///
/// let tauq = r#"
/// workers 8
/// timeout 30
/// "#;
///
/// let config: Config = from_str(tauq).unwrap();
/// assert_eq!(config.workers, 8.0);
/// assert_eq!(config.timeout, 30.0);
/// ```
pub fn from_str<T: DeserializeOwned>(s: &str) -> Result<T, RhoError> {
    let json = compile_tauq(s)?;
    serde_json::from_value(json).map_err(|e| {
        RhoError::Interpret(crate::error::InterpretError::new(format!(
            "Deserialization error: {}",
            e
        )))
    })
}

/// Deserialize Tauq from a file into a type T
///
/// Supports !import directives.
///
/// # Example
///
/// ```no_run
/// use serde::Deserialize;
/// use tauq::from_file;
/// use std::path::Path;
///
/// #[derive(Deserialize)]
/// struct Config {
///     workers: u32,
///     database: DatabaseConfig,
/// }
///
/// #[derive(Deserialize)]
/// struct DatabaseConfig {
///     host: String,
///     port: u16,
/// }
///
/// let config: Config = from_file(Path::new("config.tqn")).unwrap();
/// ```
pub fn from_file<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T, RhoError> {
    let source = std::fs::read_to_string(path.as_ref())
        .map_err(|e| RhoError::Io(e))?;
    let json = compile_tauq(&source)?;
    serde_json::from_value(json).map_err(|e| {
        RhoError::Interpret(crate::error::InterpretError::new(format!(
            "Deserialization error: {}",
            e
        )))
    })
}

/// Deserialize Tauq from bytes
///
/// # Example
///
/// ```
/// use serde::Deserialize;
/// use tauq::from_bytes;
///
/// #[derive(Deserialize)]
/// struct Data {
///     value: f64,  // Numbers in Tauq are f64
/// }
///
/// let bytes = b"value 42";
/// let data: Data = from_bytes(bytes).unwrap();
/// assert_eq!(data.value, 42.0);
/// ```
pub fn from_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, RhoError> {
    let s = std::str::from_utf8(bytes).map_err(|e| {
        RhoError::Interpret(crate::error::InterpretError::new(format!("Invalid UTF-8: {}", e)))
    })?;
    from_str(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize, Debug, PartialEq)]
    struct SimpleConfig {
        workers: f64,  // Numbers in Tauq are f64 by default
        timeout: f64,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct NestedConfig {
        app: AppConfig,
        database: DatabaseConfig,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct AppConfig {
        name: String,
        version: String,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct DatabaseConfig {
        host: String,
        port: f64,  // Numbers in Tauq are f64
        ssl: bool,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct ConfigWithArray {
        tags: Vec<String>,
        ports: Vec<f64>,  // Numbers in Tauq are f64
    }

    #[test]
    fn test_from_str_simple() {
        let tauq = r#"
workers 8
timeout 30
"#;

        let config: SimpleConfig = from_str(tauq).unwrap();
        assert_eq!(
            config,
            SimpleConfig {
                workers: 8.0,
                timeout: 30.0
            }
        );
    }

    #[test]
    fn test_from_str_nested() {
        let tauq = r#"
app {
    name "MyApp"
    version "1.0.0"
}

database {
    host localhost
    port 5432
    ssl true
}
"#;

        let config: NestedConfig = from_str(tauq).unwrap();
        assert_eq!(config.app.name, "MyApp");
        assert_eq!(config.app.version, "1.0.0");
        assert_eq!(config.database.host, "localhost");
        assert_eq!(config.database.port, 5432.0);
        assert_eq!(config.database.ssl, true);
    }

    #[test]
    fn test_from_str_with_arrays() {
        let tauq = r#"
tags [api backend web]
ports [8080 8081 8082]
"#;

        let config: ConfigWithArray = from_str(tauq).unwrap();
        assert_eq!(config.tags, vec!["api", "backend", "web"]);
        assert_eq!(config.ports, vec![8080.0, 8081.0, 8082.0]);
    }

    #[test]
    fn test_from_bytes() {
        let bytes = b"workers 8\ntimeout 30";
        let config: SimpleConfig = from_bytes(bytes).unwrap();
        assert_eq!(
            config,
            SimpleConfig {
                workers: 8.0,
                timeout: 30.0
            }
        );
    }

    #[test]
    fn test_deserialization_error() {
        use serde::Deserialize;

        let tauq = "workers 8\ntimeout 30";

        // Wrong type - expect string but got number
        #[derive(Deserialize)]
        struct WrongType {
            workers: String, // Should be f64
        }

        let result: Result<WrongType, _> = from_str(tauq);
        assert!(result.is_err());
    }
}
