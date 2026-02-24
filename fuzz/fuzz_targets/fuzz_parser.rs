#![no_main]

use libfuzzer_sys::fuzz_target;
use tauq::{compile_tauq, format_to_tauq};

fuzz_target!(|data: &[u8]| {
    // Tauq is a text format, so we primarily care about valid UTF-8,
    // but the parser should gracefully handle (or reject) invalid UTF-8 if exposed.
    // Ideally, the public API takes &str, so we convert first.
    if let Ok(s) = std::str::from_utf8(data) {
        // 1. Fuzz the Parser
        if let Ok(json) = compile_tauq(s) {
            // 2. If it parses, it must not panic the Formatter
            let _formatted = format_to_tauq(&json);
            
            // 3. Optional: Round-trip property (disabled for speed/noise, but good for deep fuzzing)
            // let round_trip_json = compile_tauq(&_formatted).unwrap();
            // assert_eq!(json, round_trip_json); 
        }
    }
});
