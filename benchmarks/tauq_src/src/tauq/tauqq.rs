use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use std::collections::{HashMap, HashSet};

use super::Parser;

/// Configuration for TauqQ processing
#[derive(Default)]
pub struct ProcessConfig {
    /// Base directory for resolving relative paths (security boundary)
    pub base_dir: Option<std::path::PathBuf>,
    /// Safe mode disables all shell execution and file I/O
    pub safe_mode: bool,
}

/// Process TauqQ directives (!pipe, !emit) and return canonical Tauq source.
pub fn process(
    input: &str,
    vars: &mut HashMap<String, String>,
    safe_mode: bool,
) -> Result<String, String> {
    let config = ProcessConfig {
        base_dir: std::env::current_dir().ok(),
        safe_mode,
    };
    let mut visited = HashSet::new();
    process_internal(input, vars, &config, 0, &mut visited)
}

/// Process with explicit configuration
pub fn process_with_config(
    input: &str,
    vars: &mut HashMap<String, String>,
    config: &ProcessConfig,
) -> Result<String, String> {
    let mut visited = HashSet::new();
    process_internal(input, vars, config, 0, &mut visited)
}

/// Validate that a path is safe (doesn't escape base_dir)
fn validate_path(
    path_str: &str,
    base_dir: &Option<std::path::PathBuf>,
) -> Result<std::path::PathBuf, String> {
    let path = Path::new(path_str);

    // Resolve relative to base_dir if set
    let resolved = if let Some(base) = base_dir {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            base.join(path)
        }
    } else {
        path.to_path_buf()
    };

    // Canonicalize to resolve symlinks and ..
    let canonical = resolved
        .canonicalize()
        .map_err(|e| format!("Cannot resolve path '{}': {}", path_str, e))?;

    // Check path traversal if base_dir is set
    if let Some(base) = base_dir {
        let base_canonical = base
            .canonicalize()
            .map_err(|e| format!("Cannot resolve base directory: {}", e))?;

        if !canonical.starts_with(&base_canonical) {
            return Err(format!(
                "Path '{}' escapes base directory (path traversal blocked)",
                path_str
            ));
        }
    }

    Ok(canonical)
}

fn process_internal(
    input: &str,
    vars: &mut HashMap<String, String>,
    config: &ProcessConfig,
    depth: usize,
    visited: &mut HashSet<String>,
) -> Result<String, String> {
    if depth > 50 {
        return Err("Maximum import depth (50) exceeded".to_string());
    }

    let mut output = String::new();
    let mut lines = input.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        if trimmed.starts_with("!set ") {
            let parts: Vec<&str> = trimmed
                .strip_prefix("!set ")
                .unwrap()
                .splitn(2, ' ')
                .collect();
            if parts.len() == 2 {
                let key = parts[0].trim();
                let val = parts[1].trim().trim_matches('"'); // Strip quotes if present
                vars.insert(key.to_string(), val.to_string());
            }
        } else if trimmed.starts_with("!import ") {
            if config.safe_mode {
                return Err("!import directive is disabled in safe mode".to_string());
            }
            let path_str = trimmed.strip_prefix("!import ").unwrap().trim();
            let clean_path = path_str.trim_matches('"');

            // Validate path for security
            let validated_path = validate_path(clean_path, &config.base_dir)?;
            let abs_path = validated_path.to_string_lossy().into_owned();

            if visited.contains(&abs_path) {
                return Err(format!("Circular import detected: {}", abs_path));
            }

            visited.insert(abs_path.clone());

            let content = std::fs::read_to_string(&validated_path)
                .map_err(|e| format!("Failed to read imported file '{}': {}", clean_path, e))?;

            // Recursive process with same vars, update base_dir to imported file's directory
            let import_config = ProcessConfig {
                base_dir: validated_path.parent().map(|p| p.to_path_buf()),
                safe_mode: config.safe_mode,
            };
            let processed_import =
                process_internal(&content, vars, &import_config, depth + 1, visited)?;
            output.push_str(&processed_import);
            output.push('\n');

            visited.remove(&abs_path);
        } else if trimmed.starts_with("!emit ") {
            if config.safe_mode {
                return Err("!emit directive is disabled in safe mode".to_string());
            }
            let cmd_str = trimmed.strip_prefix("!emit ").unwrap();
            let result = run_command(cmd_str, None, vars)?;
            validate_tauq_output(&result, "!emit", cmd_str)?;
            output.push_str(&result);
            output.push('\n');
        } else if trimmed.starts_with("!env ") {
            if config.safe_mode {
                return Err("!env directive is disabled in safe mode".to_string());
            }
            let var_name = trimmed.strip_prefix("!env ").unwrap().trim();
            if let Ok(val) = std::env::var(var_name) {
                // Emit as string
                output.push_str(&format!("\"{}\"\n", val));
            } else {
                return Err(format!("Environment variable '{}' not found", var_name));
            }
        } else if trimmed.starts_with("!read ") {
            if config.safe_mode {
                return Err("!read directive is disabled in safe mode".to_string());
            }
            let path_str = trimmed.strip_prefix("!read ").unwrap().trim();
            let clean_path = path_str.trim_matches('"');

            // Validate path for security
            let validated_path = validate_path(clean_path, &config.base_dir)?;

            let content = std::fs::read_to_string(&validated_path)
                .map_err(|e| format!("Failed to read file '{}': {}", clean_path, e))?;
            let json_str = serde_json::to_string(&content).map_err(|e| e.to_string())?;
            output.push_str(&json_str);
            output.push('\n');
        } else if trimmed.starts_with("!json ") {
            if config.safe_mode {
                return Err("!json directive is disabled in safe mode".to_string());
            }
            let path_str = trimmed.strip_prefix("!json ").unwrap().trim();
            let clean_path = path_str.trim_matches('"');

            // Validate path for security
            let validated_path = validate_path(clean_path, &config.base_dir)?;

            let content = std::fs::read_to_string(&validated_path)
                .map_err(|e| format!("Failed to read file '{}': {}", clean_path, e))?;

            let json_val: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse JSON file '{}': {}", clean_path, e))?;

            let tauq_str = super::json_to_tauq(&json_val);
            output.push_str(&tauq_str);
            output.push('\n');
        } else if trimmed.starts_with("!run ") {
            if config.safe_mode {
                return Err("!run directive is disabled in safe mode".to_string());
            }
            // Parse "!run cmd args... {"
            let line_content = trimmed.strip_prefix("!run ").unwrap().trim();
            let cmd_part = if let Some(stripped) = line_content.strip_suffix(" {") {
                stripped
            } else {
                // Handle case where { is on next line? Or just lenient parsing
                line_content
            };

            let cmd_parts = split_args(cmd_part)?;
            if cmd_parts.is_empty() {
                return Err("!run missing command".to_string());
            }
            let program = &cmd_parts[0];
            let args = &cmd_parts[1..];

            let mut raw_lines = Vec::new();
            let mut found_end = false;

            for l in lines.by_ref() {
                if l.trim() == "}" {
                    found_end = true;
                    break;
                }
                raw_lines.push(l);
            }

            if !found_end {
                return Err("Unterminated code block for !run".to_string());
            }

            // Dedent logic
            let mut min_indent = usize::MAX;
            for line in &raw_lines {
                let trimmed = line.trim_start();
                if !trimmed.is_empty() {
                    let indent = line.len() - trimmed.len();
                    if indent < min_indent {
                        min_indent = indent;
                    }
                }
            }

            if min_indent == usize::MAX {
                min_indent = 0;
            }

            let mut code_block = String::new();
            for line in raw_lines {
                if line.len() >= min_indent {
                    code_block.push_str(&line[min_indent..]);
                } else {
                    code_block.push_str(line);
                }
                code_block.push('\n');
            }

            let result = run_code_block(program, args, &code_block, vars, None)?;
            validate_tauq_output(&result, "!run", program)?;
            output.push_str(&result);
            output.push('\n');
        } else if trimmed.starts_with("!pipe ") {
            if config.safe_mode {
                return Err("!pipe directive is disabled in safe mode".to_string());
            }
            let cmd_str = trimmed.strip_prefix("!pipe ").unwrap().trim();

            // Check for block syntax: "!pipe cmd args... {"
            if let Some(stripped_cmd) = cmd_str.strip_suffix(" {") {
                let cmd_parts = split_args(stripped_cmd)?;
                if cmd_parts.is_empty() {
                    return Err("!pipe missing command".to_string());
                }
                let program = &cmd_parts[0];
                let args = &cmd_parts[1..];

                let mut raw_lines = Vec::new();
                let mut found_end = false;

                for l in lines.by_ref() {
                    if l.trim() == "}" {
                        found_end = true;
                        break;
                    }
                    raw_lines.push(l);
                }

                if !found_end {
                    return Err("Unterminated code block for !pipe".to_string());
                }

                // Dedent logic: Find the minimum common indentation level among non-empty lines.
                // This allows the user to write code flush-left or indented relative to the parent file structure
                // without manual adjustments.
                let mut min_indent = usize::MAX;
                for line in &raw_lines {
                    let trimmed = line.trim_start();
                    if !trimmed.is_empty() {
                        let indent = line.len() - trimmed.len();
                        if indent < min_indent {
                            min_indent = indent;
                        }
                    }
                }

                if min_indent == usize::MAX {
                    min_indent = 0;
                }

                let mut code_block = String::new();
                for line in raw_lines {
                    if line.len() >= min_indent {
                        code_block.push_str(&line[min_indent..]);
                    } else {
                        // Preserve empty lines or lines with only whitespace
                        code_block.push_str(line);
                    }
                    code_block.push('\n');
                }

                // Execute block with input
                let result = run_code_block(program, args, &code_block, vars, Some(&output))?;
                validate_tauq_output(&result, "!pipe", program)?;
                output = result;
            } else {
                // Standard single-line pipe
                // Top-down pipe: transform current output
                let result = run_command(cmd_str, Some(&output), vars)?;
                validate_tauq_output(&result, "!pipe", cmd_str)?;
                output = result;
            }
        } else if trimmed.starts_with('#') || trimmed.is_empty() {
            // Ignore comments and empty lines
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }

    Ok(output)
}

/// Validate that command output is valid Tauq notation.
/// Returns Ok(output) if valid, Err with helpful message if not.
fn validate_tauq_output(output: &str, directive: &str, source_hint: &str) -> Result<(), String> {
    let trimmed = output.trim();

    // Empty output is valid
    if trimmed.is_empty() {
        return Ok(());
    }

    // Try to parse as Tauq
    let mut parser = Parser::new(trimmed);
    match parser.parse() {
        Ok(_) => Ok(()),
        Err(e) => {
            // Create a helpful error message with context
            let preview = if trimmed.len() > 200 {
                format!("{}...", &trimmed[..200])
            } else {
                trimmed.to_string()
            };

            // Check for common JSON mistakes
            let hint = if trimmed.contains("\":") || trimmed.contains("\": ") {
                "\n  Hint: Output looks like JSON. Use Tauq syntax (spaces, no colons/commas) or use !json for JSON files."
            } else if trimmed.contains(',') && (trimmed.starts_with('{') || trimmed.starts_with('[')) {
                "\n  Hint: Output contains commas. Tauq uses spaces as delimiters, not commas."
            } else {
                ""
            };

            Err(format!(
                "Invalid Tauq output from {} ({}):\n  Output: {}\n  Error: {}{}",
                directive, source_hint, preview, e, hint
            ))
        }
    }
}

fn run_command(
    cmd_str: &str,
    input: Option<&str>,
    vars: &HashMap<String, String>,
) -> Result<String, String> {
    let parts = split_args(cmd_str)?;
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }

    let program = &parts[0];
    let args = &parts[1..];

    let mut child = Command::new(program)
        .args(args)
        .envs(vars)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn command '{}': {}", program, e))?;

    if let Some(input_str) = input
        && let Some(mut stdin) = child.stdin.take()
    {
        stdin
            .write_all(input_str.as_bytes())
            .map_err(|e| format!("Failed to write to stdin: {}", e))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait on command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Command '{}' failed: {}", cmd_str, stderr));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| format!("Command output is not valid UTF-8: {}", e))
}

fn run_code_block(
    program: &str,
    args: &[String],
    code: &str,
    vars: &HashMap<String, String>,
    input: Option<&str>,
) -> Result<String, String> {
    use std::io::Write;

    // Create a temporary file with the code
    let mut temp_file =
        tempfile::NamedTempFile::new().map_err(|e| format!("Failed to create temp file: {}", e))?;

    write!(temp_file, "{}", code).map_err(|e| format!("Failed to write to temp file: {}", e))?;

    let path = temp_file.path().to_str().ok_or("Invalid temp file path")?;

    // Execute the interpreter with the file
    let mut child = Command::new(program)
        .args(args)
        .arg(path)
        .envs(vars)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn interpreter '{}': {}", program, e))?;

    if let Some(input_str) = input
        && let Some(mut stdin) = child.stdin.take()
    {
        stdin
            .write_all(input_str.as_bytes())
            .map_err(|e| format!("Failed to write to stdin: {}", e))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait on interpreter: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Code execution failed: {}", stderr));
    }

    String::from_utf8(output.stdout).map_err(|e| format!("Code output is not valid UTF-8: {}", e))
}

/// Split command string into arguments, respecting quotes.
fn split_args(input: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut quote_char = '\0';
    let mut escaped = false;

    for c in input.chars() {
        if in_quote && quote_char == '\'' {
            // Single quotes: literal content, no escapes allowed (except closing quote which isn't escaped but matched)
            if c == '\'' {
                in_quote = false;
            } else {
                current.push(c);
            }
        } else if escaped {
            current.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if in_quote {
            // Double quotes (or other): allow escapes
            if c == quote_char {
                in_quote = false;
            } else {
                current.push(c);
            }
        } else if c == '"' || c == '\'' {
            in_quote = true;
            quote_char = c;
        } else if c.is_whitespace() {
            if !current.is_empty() {
                args.push(current);
                current = String::new();
            }
        } else {
            current.push(c);
        }
    }

    if in_quote {
        return Err("Unterminated quote".to_string());
    }

    if !current.is_empty() {
        args.push(current);
    }

    Ok(args)
}
