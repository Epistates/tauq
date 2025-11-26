// tauq - Token-Efficient Data Notation
//
// Tauq (τq): Time constant meets charge density
// - 44-54% fewer tokens than JSON (verified with tiktoken)
// - Line-by-line lexing, batch parsing
// - Beautiful, minimal syntax
//
// Commands:
// - build: .tqn → .json (parse to JSON)
// - format: .json → .tqn (convert JSON to Tauq)
// - exec: .tqq → .json (execute transformations)
// - minify: .tqn → .tqn (compress to single line)
// - validate: check syntax

use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    let cmd = &args[1];

    match cmd.as_str() {
        "--help" | "-h" => {
            print_help();
            Ok(())
        }
        "--version" | "-v" => {
            print_version();
            Ok(())
        }
        "build" => cmd_build(&args[2..]),
        "format" | "fmt" => cmd_format(&args[2..]),
        "exec" => cmd_exec(&args[2..]),
        "minify" => cmd_minify(&args[2..]),
        "prettify" | "pretty" => cmd_prettify(&args[2..]),
        "validate" => cmd_validate(&args[2..]),
        "query" | "q" => cmd_query(&args[2..]),
        _ => {
            // Legacy: treat as build if file exists
            if std::path::Path::new(cmd).exists() {
                cmd_build_legacy(&args[1..])
            } else {
                eprintln!("Unknown command: {}", cmd);
                eprintln!("Run 'tauq --help' for usage information.");
                std::process::exit(1);
            }
        }
    }
}

// ========== BUILD: Smart compilation based on file type ==========
//
// .tqn files → JSON output (default)
// .tqq files → Tauq output (default), use --json for JSON

fn cmd_build(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Missing input file. Usage: tauq build <file.tqn|.tqq> [--json] [--pretty]".to_string());
    }

    let input_path = &args[0];
    let mut output_path: Option<PathBuf> = None;
    let mut pretty = false;
    let mut force_json = false;
    let mut safe_mode = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    output_path = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    return Err("Missing output file after -o".to_string());
                }
            }
            "-p" | "--pretty" => {
                pretty = true;
                i += 1;
            }
            "--json" => {
                force_json = true;
                i += 1;
            }
            "-s" | "--safe" => {
                safe_mode = true;
                i += 1;
            }
            _ => return Err(format!("Unknown option: {}", args[i])),
        }
    }

    // Detect file type
    let is_tqq = input_path.ends_with(".tqq");

    // Read source
    let source = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read {}: {}", input_path, e))?;

    // Parse/Execute based on file type
    let json = if is_tqq {
        // .tqq files: Two-step process for better error reporting
        // Step 1: Process TauqQ directives
        let processed = match tauq::process_tauqq(&source, safe_mode) {
            Ok(p) => p,
            Err(e) => {
                tauq::print_error_with_source(&source, &e);
                return Err("TauqQ processing failed".to_string());
            }
        };
        // Step 2: Parse the processed Tauq (show processed source on errors)
        match tauq::compile_tauq(&processed) {
            Ok(j) => j,
            Err(e) => {
                // Show the PROCESSED source since that's where the parse error is
                tauq::print_error_with_source(&processed, &e);
                return Err("Parse failed (in TauqQ output)".to_string());
            }
        }
    } else {
        // .tqn files: Parse Tauq
        match tauq::compile_tauq(&source) {
            Ok(j) => j,
            Err(e) => {
                tauq::print_error_with_source(&source, &e);
                return Err("Parse failed".to_string());
            }
        }
    };

    // Determine output format:
    // - .tqn → JSON (default)
    // - .tqq → Tauq (default), --json forces JSON
    let output = if is_tqq && !force_json {
        // .tqq defaults to Tauq output
        tauq::format_to_tauq(&json)
    } else {
        // .tqn defaults to JSON, or .tqq with --json
        if pretty {
            serde_json::to_string_pretty(&json)
        } else {
            serde_json::to_string(&json)
        }
        .map_err(|e| format!("JSON serialization error: {}", e))?
    };

    // Write output
    if let Some(path) = output_path {
        fs::write(&path, &output)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
        let format_name = if is_tqq && !force_json { "Tauq" } else { "JSON" };
        eprintln!("✓ Built {} → {} ({})", input_path, path.display(), format_name);
    } else {
        println!("{}", output);
    }

    Ok(())
}

fn cmd_build_legacy(args: &[String]) -> Result<(), String> {
    // Support: tauq input.tqn -o output.json
    cmd_build(args)
}

// ========== FORMAT: JSON → Tauq ==========

#[derive(Clone, Copy, PartialEq)]
enum FormatMode {
    Standard,   // Space-delimited, readable
    Optimized,  // Comma-delimited, token-efficient
    Ultra,      // Comma-delimited + minified, maximum efficiency
}

fn cmd_format(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Missing input file. Usage: tauq format <input.json> [--optimized|--ultra]".to_string());
    }

    let input_path = &args[0];
    let mut output_path: Option<PathBuf> = None;
    let mut mode = FormatMode::Standard;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    output_path = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    return Err("Missing output file after -o".to_string());
                }
            }
            "--optimized" | "-O" => {
                mode = FormatMode::Optimized;
                i += 1;
            }
            "--ultra" | "-U" => {
                mode = FormatMode::Ultra;
                i += 1;
            }
            _ => return Err(format!("Unknown option: {}", args[i])),
        }
    }

    // Read JSON
    let json_str = if input_path == "-" {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|e| format!("Failed to read stdin: {}", e))?;
        buffer
    } else {
        fs::read_to_string(input_path)
            .map_err(|e| format!("Failed to read {}: {}", input_path, e))?
    };

    // Parse JSON
    let json: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Format to Tauq based on mode
    let tauq_output = match mode {
        FormatMode::Standard => tauq::tauq::json_to_tauq(&json),
        FormatMode::Optimized => tauq::tauq::json_to_tauq_optimized(&json),
        FormatMode::Ultra => tauq::tauq::json_to_tauq_ultra(&json),
    };

    let mode_name = match mode {
        FormatMode::Standard => "standard",
        FormatMode::Optimized => "optimized",
        FormatMode::Ultra => "ultra",
    };

    // Write output
    if let Some(path) = output_path {
        fs::write(&path, &tauq_output)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
        eprintln!("✓ Formatted {} → {} ({})", input_path, path.display(), mode_name);
    } else {
        println!("{}", tauq_output);
    }

    Ok(())
}

// ========== EXEC: TauqQ → JSON ==========

fn cmd_exec(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Missing input file. Usage: tauq exec <input.tqq>".to_string());
    }

    let input_path = &args[0];
    let mut output_path: Option<PathBuf> = None;
    let mut pretty = false;
    let mut safe_mode = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    output_path = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    return Err("Missing output file after -o".to_string());
                }
            }
            "-p" | "--pretty" => {
                pretty = true;
                i += 1;
            }
            "-s" | "--safe" => {
                safe_mode = true;
                i += 1;
            }
            _ => return Err(format!("Unknown option: {}", args[i])),
        }
    }

    // Execute TauqQ
    let source = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read {}: {}", input_path, e))?;

    let json = match tauq::compile_tauqq(&source, safe_mode) {
        Ok(j) => j,
        Err(e) => {
            tauq::print_error_with_source(&source, &e);
            return Err("Execution failed".to_string());
        }
    };

    // Serialize to JSON
    let output = if pretty {
        serde_json::to_string_pretty(&json)
    } else {
        serde_json::to_string(&json)
    }
    .map_err(|e| format!("JSON serialization error: {}", e))?;

    // Write output
    if let Some(path) = output_path {
        fs::write(&path, output)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
        eprintln!("✓ Executed {} → {}", input_path, path.display());
    } else {
        println!("{}", output);
    }

    Ok(())
}

// ========== MINIFY: Tauq → Minified Tauq ==========

fn cmd_minify(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Missing input file. Usage: tauq minify <input.tqn>".to_string());
    }

    let input_path = &args[0];
    let mut output_path: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    output_path = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    return Err("Missing output file after -o".to_string());
                }
            }
            _ => return Err(format!("Unknown option: {}", args[i])),
        }
    }

    // Read, parse, and minify
    let source = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read {}: {}", input_path, e))?;

    let json = match tauq::compile_tauq(&source) {
        Ok(j) => j,
        Err(e) => {
            tauq::print_error_with_source(&source, &e);
            return Err("Parse failed".to_string());
        }
    };

    let minified = tauq::tauq::minify_tauq(&json);

    // Write output
    if let Some(path) = output_path {
        fs::write(&path, minified)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
        eprintln!("✓ Minified {} → {}", input_path, path.display());
    } else {
        println!("{}", minified);
    }

    Ok(())
}

// ========== PRETTIFY: Minified Tauq → Pretty Tauq ==========

fn cmd_prettify(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Missing input file. Usage: tauq prettify <input.tqn>".to_string());
    }

    let input_path = &args[0];
    let mut output_path: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    output_path = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    return Err("Missing output file after -o".to_string());
                }
            }
            _ => return Err(format!("Unknown option: {}", args[i])),
        }
    }

    // Read, parse, and prettify
    let source = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read {}: {}", input_path, e))?;

    let json = match tauq::compile_tauq(&source) {
        Ok(j) => j,
        Err(e) => {
            tauq::print_error_with_source(&source, &e);
            return Err("Parse failed".to_string());
        }
    };

    let pretty = tauq::tauq::json_to_tauq(&json);

    // Write output
    if let Some(path) = output_path {
        fs::write(&path, pretty)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
        eprintln!("✓ Prettified {} → {}", input_path, path.display());
    } else {
        println!("{}", pretty);
    }

    Ok(())
}

// ========== VALIDATE: Check Syntax ==========

fn cmd_validate(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Missing input file. Usage: tauq validate <input.tqn>".to_string());
    }

    let input_path = &args[0];

    // Read and parse
    let source = fs::read_to_string(input_path)
        .map_err(|e| format!("Failed to read {}: {}", input_path, e))?;

    // Try to parse
    let _ = match tauq::compile_tauq(&source) {
        Ok(j) => j,
        Err(e) => {
            tauq::print_error_with_source(&source, &e);
            return Err("Validation failed".to_string());
        }
    };

    println!("✓ Valid Tauq: {}", input_path);
    Ok(())
}

// ========== QUERY: Filter/Map with Rhai ==========

#[cfg(feature = "rhai")]
fn cmd_query(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: tauq query <file.tqn | -> <expression> [-o <output.tqn>]".to_string());
    }

    let input_source_arg = &args[0];
    let expression_arg_index = if input_source_arg == "-" {
        1 // If reading from stdin, expression is the first arg
    } else {
        if args.len() < 2 {
            return Err("Missing expression. Usage: tauq query <file.tqn | -> <expression> [-o <output.tqn>]".to_string());
        }
        1 // If reading from file, expression is the second arg
    };

    if args.len() <= expression_arg_index {
        return Err(
            "Missing expression. Usage: tauq query <file.tqn | -> <expression> [-o <output.tqn>]"
                .to_string(),
        );
    }

    let expression = &args[expression_arg_index];
    let mut output_path: Option<PathBuf> = None;

    let mut i = expression_arg_index + 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    output_path = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    return Err("Missing output file after -o".to_string());
                }
            }
            _ => return Err(format!("Unknown option: {}", args[i])),
        }
    }

    let source = if input_source_arg == "-" {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|e| format!("Failed to read stdin: {}", e))?;
        buffer
    } else {
        fs::read_to_string(input_source_arg)
            .map_err(|e| format!("Failed to read {}: {}", input_source_arg, e))?
    };

    let json = tauq::compile_tauq(&source).map_err(|e| e.to_string())?;

    let engine = rhai::Engine::new();
    let mut scope = rhai::Scope::new();

    let dynamic_json = rhai::serde::to_dynamic(&json).map_err(|e| e.to_string())?;
    scope.push("data", dynamic_json);

    // Ergonomics: Allow ".field" to imply "data.field"
    let script = expression.trim();
    let final_script = if script.starts_with('.') {
        format!("data{}", script)
    } else {
        script.to_string()
    };

    let result = engine
        .eval_with_scope::<rhai::Dynamic>(&mut scope, &final_script)
        .map_err(|e| format!("Query error: {}", e))?;

    let result_json: serde_json::Value = rhai::serde::from_dynamic(&result)
        .map_err(|e| format!("Result serialization error: {}", e))?;

    let output = tauq::tauq::json_to_tauq(&result_json);

    if let Some(path) = output_path {
        fs::write(&path, output)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
        eprintln!("✓ Query result saved to {}", path.display());
    } else {
        println!("{}", output);
    }

    Ok(())
}

#[cfg(not(feature = "rhai"))]
fn cmd_query(_args: &[String]) -> Result<(), String> {
    Err("Query support is disabled. Recompile with 'rhai' feature.".to_string())
}

// ========== HELP & VERSION ==========

fn print_help() {
    println!(
        r#"tauq - Token-Efficient Data Notation

Tauq (τq): Where time constant meets charge density
Fields, densities, rates - optimized for AI

USAGE:
    tauq <COMMAND> [OPTIONS]

COMMANDS:
    build <file>            Smart build based on extension:
                              .tqn → JSON (default output)
                              .tqq → Tauq (default), use --json for JSON
    format <file.json>      Convert JSON to Tauq
    query <file | -> <expr> Filter/Transform with Rhai expressions
    exec <file.tqq>         Execute Tauq Query (always outputs JSON)
    minify <file.tqn>       Compress to single line
    prettify <file.tqn>     Format to readable Tauq
    validate <file.tqn>     Check syntax

OPTIONS:
    -o, --output <FILE>     Write output to file
    -p, --pretty            Pretty-print JSON output
    --json                  Force JSON output (for .tqq files)
    -s, --safe              Enable safe mode (disable shell execution)
    -h, --help              Print this help
    -v, --version           Print version

FORMAT OPTIONS:
    -O, --optimized         Comma-delimited (TOON/CSV style, less efficient)
    -U, --ultra             Comma-delimited + minified (TOON/CSV style)

EXAMPLES:
    # Parse Tauq (.tqn) to JSON
    tauq build config.tqn -o config.json
    tauq build config.tqn --pretty

    # Execute Tauq Query (.tqq) to Tauq
    tauq build pipeline.tqq -o output.tqn

    # Execute Tauq Query (.tqq) to JSON
    tauq build pipeline.tqq --json -o output.json

    # Convert JSON to Tauq (standard mode)
    tauq format data.json -o data.tqn

    # Convert JSON to Tauq (comma-delimited, TOON/CSV style)
    tauq format data.json -O -o data.tqn

    # Convert JSON to Tauq (comma-delimited + minified)
    tauq format data.json -U -o data.tqn

    # Filter data using Rhai (our 'jq')
    tauq query users.tqn '.filter(|u| u.age > 30)'

    # Access properties
    tauq query config.tqn .server.port

    # Minify for production
    tauq minify config.tqn -o config.min.tqn

WHY TAUQ:
    • 44-54% fewer tokens than JSON (verified with tiktoken cl100k_base)
    • 11% more efficient than TOON/CSV overall
    • True streaming via StreamingParser iterator
    • Beautiful, minimal syntax
    • Schema-driven with !def / !use
    • Programmable with Tauq Query (TQQ)

Learn more: https://tauq.org
"#
    );
}

fn print_version() {
    println!("tauq {}", env!("CARGO_PKG_VERSION"));
    println!("Tauq (τq): Token-efficient data notation - 44-54% fewer tokens than JSON");
}
