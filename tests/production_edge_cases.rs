//! Production Edge Cases & Production Hardening Tests
//!
//! All tests in this file call real tauq library functions.
//! No test constructs a serde_json::Value and asserts on it alone —
//! every test exercises at least one path through compile_tauq,
//! format_to_tauq, tauq::tbf::encode_json, tauq::tbf::decode,
//! tauq::tbf::to_bytes, or tauq::tbf::from_bytes.

#[cfg(test)]
mod edge_cases {
    use tauq::compile_tauq;
    use tauq::tbf::{decode, encode_json, from_bytes, to_bytes};

    // -------------------------------------------------------------------------
    // Null handling
    // -------------------------------------------------------------------------

    /// Build a Tauq document with 10 null-valued fields, parse it, and
    /// confirm every field comes back as JSON null after a TBF roundtrip.
    #[test]
    fn test_all_null_column() {
        // Build: key0 null\nkey1 null\n...
        let mut src = String::new();
        for i in 0..10 {
            src.push_str(&format!("key{} null\n", i));
        }

        let json = compile_tauq(&src).expect("compile_tauq failed");
        let obj = json.as_object().expect("expected object");
        assert_eq!(obj.len(), 10, "expected 10 fields");
        for (_, v) in obj {
            assert!(v.is_null(), "every value must be null");
        }

        // TBF roundtrip preserves nulls.
        let bytes = encode_json(&json).expect("encode_json failed");
        let decoded = decode(&bytes).expect("decode failed");
        assert_eq!(json, decoded, "TBF roundtrip changed null values");
    }

    // -------------------------------------------------------------------------
    // Deep nesting
    // -------------------------------------------------------------------------

    /// Build a 50-level nested Tauq object using bracket syntax and verify
    /// that compile_tauq produces a Value with the correct depth.
    #[test]
    fn test_deeply_nested_structures() {
        // Construct: a { b { c { ... value 1 ... }}}
        // Tauq nested-object syntax: key { key { ... } }
        const DEPTH: usize = 50;

        let mut src = String::new();
        for i in 0..DEPTH {
            src.push_str(&format!("lvl{} {{\n", i));
        }
        src.push_str("leaf 1\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }

        let json = compile_tauq(&src).expect("compile_tauq must succeed at 50 levels");

        // Walk down to the leaf to confirm depth.
        let mut cursor = &json;
        for i in 0..DEPTH {
            let key = format!("lvl{}", i);
            cursor = cursor
                .get(&key)
                .unwrap_or_else(|| panic!("missing key {} at depth {}", key, i));
        }
        assert_eq!(cursor["leaf"], 1, "leaf value must be 1 at depth {}", DEPTH);
    }

    /// 101 levels of nesting must trigger the parser's MAX_NESTING_DEPTH guard.
    #[test]
    fn test_max_nesting_depth() {
        const DEPTH: usize = 101; // one over the 100-level limit

        let mut src = String::new();
        for i in 0..DEPTH {
            src.push_str(&format!("n{} {{\n", i));
        }
        src.push_str("leaf 1\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }

        let result = compile_tauq(&src);
        assert!(
            result.is_err(),
            "parser must reject nesting depth > MAX_NESTING_DEPTH (100), got: {:?}",
            result
        );
        let msg = result.unwrap_err().to_string().to_lowercase();
        assert!(
            msg.contains("nesting") || msg.contains("depth") || msg.contains("exceeded"),
            "error message should mention nesting/depth, got: {}",
            msg
        );
    }

    // -------------------------------------------------------------------------
    // Malformed input
    // -------------------------------------------------------------------------

    /// Strings that are not valid Tauq must make compile_tauq return Err.
    #[test]
    fn test_malformed_data_resilience() {
        let bad_inputs = [
            // Mismatched closing brace at top level
            "key }", // Mismatched closing bracket at top level
            "key ]", // Directive with no schema name
            "!def",
        ];

        for input in &bad_inputs {
            let result = compile_tauq(input);
            assert!(
                result.is_err(),
                "expected parse error for input {:?}, got: {:?}",
                input,
                result
            );
        }
    }

    // -------------------------------------------------------------------------
    // Medium dataset via !def schema
    // -------------------------------------------------------------------------

    /// Generate a 1 000-row !def table, parse with compile_tauq, and verify
    /// the result is an array whose length equals the row count.
    #[test]
    fn test_medium_dataset() {
        let mut src = String::from("!def Row id value flag\n!use Row\n");
        for i in 0u32..1000 {
            src.push_str(&format!("{} {} {}\n", i, i * 2, i % 2 == 0));
        }

        let json = compile_tauq(&src).expect("compile_tauq failed on 1000-row dataset");
        let arr = json.as_array().expect("expected array from !def table");
        assert_eq!(arr.len(), 1000, "must have exactly 1000 rows");

        // Spot-check first and last rows.
        assert_eq!(arr[0]["id"], 0);
        assert_eq!(arr[0]["value"], 0);
        assert_eq!(arr[999]["id"], 999);
        assert_eq!(arr[999]["value"], 1998);
    }

    // -------------------------------------------------------------------------
    // Whitespace variations
    // -------------------------------------------------------------------------

    /// Verify that Tauq correctly parses strings containing leading/trailing
    /// whitespace, tabs, and embedded newlines preserved inside double quotes.
    #[test]
    fn test_whitespace_variations() {
        // In Tauq, quoted strings preserve internal whitespace.
        let input = r#"
leading " leading"
trailing "trailing "
both " both "
tabs "	tab	here	"
"#;
        let json = compile_tauq(input).expect("compile_tauq failed");
        assert_eq!(json["leading"], " leading");
        assert_eq!(json["trailing"], "trailing ");
        assert_eq!(json["both"], " both ");
        assert_eq!(json["tabs"], "\ttab\there\t");
    }

    // -------------------------------------------------------------------------
    // Empty array syntax
    // -------------------------------------------------------------------------

    /// `key []` must produce a field containing an empty JSON array.
    #[test]
    fn test_empty_arrays() {
        let input = "queue []\npending []\n";
        let json = compile_tauq(input).expect("compile_tauq failed");
        let queue = json["queue"].as_array().expect("queue must be an array");
        let pending = json["pending"]
            .as_array()
            .expect("pending must be an array");
        assert!(queue.is_empty(), "queue array must be empty");
        assert!(pending.is_empty(), "pending array must be empty");

        // Roundtrip through TBF preserves empty arrays.
        let bytes = encode_json(&json).expect("encode_json failed");
        let decoded = decode(&bytes).expect("decode failed");
        assert_eq!(json, decoded);
    }

    // -------------------------------------------------------------------------
    // Float precision via TBF
    // -------------------------------------------------------------------------

    /// Encode floats through Tauq -> TBF and verify bit-exact f64 roundtrip.
    #[test]
    fn test_decimal_precision() {
        let cases: &[f64] = &[
            0.1,
            0.2,
            1.0 / 3.0,
            std::f64::consts::PI,
            f64::MAX,
            f64::MIN_POSITIVE,
        ];
        for &v in cases {
            let bytes = to_bytes(&v).expect("to_bytes failed");
            let decoded: f64 = from_bytes(&bytes).expect("from_bytes failed");
            assert_eq!(
                v.to_bits(),
                decoded.to_bits(),
                "f64 roundtrip must be bit-exact for {}",
                v
            );
        }
    }

    // -------------------------------------------------------------------------
    // Unicode
    // -------------------------------------------------------------------------

    /// Unicode strings must survive Tauq parsing and a TBF roundtrip intact.
    #[test]
    fn test_unicode_strings() {
        let input = r#"
chinese "你好世界"
arabic "مرحبا"
emoji "🚀🦀"
hebrew "שלום"
greek "Ελληνικά"
"#;
        let json = compile_tauq(input).expect("compile_tauq failed");
        assert_eq!(json["chinese"], "你好世界");
        assert_eq!(json["arabic"], "مرحبا");
        assert_eq!(json["emoji"], "🚀🦀");

        let bytes = encode_json(&json).expect("encode_json failed");
        let decoded = decode(&bytes).expect("decode failed");
        assert_eq!(json["chinese"], decoded["chinese"]);
        assert_eq!(json["emoji"], decoded["emoji"]);
    }

    // -------------------------------------------------------------------------
    // Special characters in strings
    // -------------------------------------------------------------------------

    /// Strings with backslashes, embedded quotes, and newline escapes must
    /// parse correctly and survive a TBF roundtrip.
    #[test]
    fn test_special_characters_in_strings() {
        // Tauq quoted strings support JSON-style escape sequences.
        let input = r#"
with_backslash "path\\to\\file"
with_newline "line1\nline2"
with_tab "col1\tcol2"
"#;
        let json = compile_tauq(input).expect("compile_tauq failed");
        assert!(
            json["with_backslash"].as_str().unwrap().contains('\\'),
            "backslash must be preserved"
        );
        assert!(
            json["with_newline"].as_str().unwrap().contains('\n'),
            "newline escape must be decoded"
        );
        assert!(
            json["with_tab"].as_str().unwrap().contains('\t'),
            "tab escape must be decoded"
        );

        // Roundtrip through TBF.
        let bytes = encode_json(&json).expect("encode_json failed");
        let decoded = decode(&bytes).expect("decode failed");
        assert_eq!(json["with_backslash"], decoded["with_backslash"]);
        assert_eq!(json["with_newline"], decoded["with_newline"]);
    }
}

// =============================================================================
// TBF binary resilience tests
// =============================================================================

#[cfg(test)]
mod tbf_resilience {
    use tauq::tbf::{TBF_MAGIC, TBF_VERSION};
    use tauq::tbf::{decode, to_bytes};

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    /// Produce a minimal but valid TBF buffer so we have a real header to
    /// mutate in subsequent tests.
    fn minimal_valid_tbf() -> Vec<u8> {
        // Encode the integer 42 — simple, produces a short valid buffer.
        to_bytes(&42i64).expect("to_bytes must succeed")
    }

    // -------------------------------------------------------------------------
    // Completely corrupted data
    // -------------------------------------------------------------------------

    /// Random bytes that share no structure with TBF must produce an error.
    #[test]
    fn test_tbf_corrupted_data() {
        let garbage: &[u8] = &[0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE, 0xFF, 0x00];
        let result = decode(garbage);
        assert!(
            result.is_err(),
            "decode must reject garbage bytes, got: {:?}",
            result
        );
    }

    // -------------------------------------------------------------------------
    // Empty input
    // -------------------------------------------------------------------------

    /// An empty slice has no header; decode must return an error, not panic.
    #[test]
    fn test_tbf_empty_input() {
        let result = decode(&[]);
        assert!(result.is_err(), "decode of empty slice must be an error");
        let msg = result.unwrap_err().to_string().to_lowercase();
        assert!(
            msg.contains("too short")
                || msg.contains("short")
                || msg.contains("empty")
                || msg.contains("unexpected"),
            "error must mention data is too short, got: {}",
            msg
        );
    }

    // -------------------------------------------------------------------------
    // Truncated data
    // -------------------------------------------------------------------------

    /// Truncating a valid TBF buffer at every possible offset before its end
    /// must never panic — each truncation must return an error.
    #[test]
    fn test_tbf_truncated_data() {
        let valid = minimal_valid_tbf();
        assert!(
            valid.len() > 8,
            "valid buffer must be longer than the 8-byte header"
        );

        // Skip offset 0 (empty slice covered by test_tbf_empty_input) and
        // skip the full length (that is the valid case).
        for len in 1..valid.len() {
            let truncated = &valid[..len];
            let result = decode(truncated);
            assert!(
                result.is_err(),
                "decode of truncated buffer (len={}) must be Err, got Ok({:?})",
                len,
                result
            );
        }
    }

    // -------------------------------------------------------------------------
    // Wrong magic bytes
    // -------------------------------------------------------------------------

    /// A buffer whose first four bytes are not the TBF magic must be rejected
    /// with a clear "invalid magic" error.
    #[test]
    fn test_tbf_wrong_magic() {
        let mut buf = minimal_valid_tbf();
        // Overwrite magic with 'FAKE'
        buf[0] = b'F';
        buf[1] = b'A';
        buf[2] = b'K';
        buf[3] = b'E';

        let result = decode(&buf);
        assert!(result.is_err(), "wrong magic must be rejected");
        let msg = result.unwrap_err().to_string().to_lowercase();
        assert!(
            msg.contains("magic") || msg.contains("invalid") || msg.contains("format"),
            "error must mention invalid magic/format, got: {}",
            msg
        );
    }

    // -------------------------------------------------------------------------
    // Wrong version
    // -------------------------------------------------------------------------

    /// A version byte higher than the current TBF_VERSION must be rejected.
    #[test]
    fn test_tbf_wrong_version() {
        let mut buf = minimal_valid_tbf();
        // byte 4 is the version field
        buf[4] = 0xFF;

        let result = decode(&buf);
        assert!(result.is_err(), "unsupported version 0xFF must be rejected");
        let msg = result.unwrap_err().to_string().to_lowercase();
        assert!(
            msg.contains("version") || msg.contains("unsupported"),
            "error must mention unsupported version, got: {}",
            msg
        );
    }

    // -------------------------------------------------------------------------
    // Invalid type tag in data section
    // -------------------------------------------------------------------------

    /// A buffer with a valid 8-byte header but a type tag byte of 0xFF (which
    /// does not map to any TypeTag variant) must cause decode to return Err.
    #[test]
    fn test_tbf_invalid_type_tag() {
        // Craft a minimal header manually:
        //   bytes 0-3: TBF_MAGIC
        //   byte 4:    TBF_VERSION
        //   byte 5:    flags = 0
        //   bytes 6-7: reserved = 0
        //   byte 8:    dictionary count = 0  (varint 0)
        //   byte 9:    type tag = 0xFF       (invalid)
        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(&TBF_MAGIC); // magic
        buf.push(TBF_VERSION); // version
        buf.push(0x00); // flags
        buf.push(0x00); // reserved
        buf.push(0x00); // reserved
        buf.push(0x00); // dict count = 0
        buf.push(0xFF); // invalid type tag

        let result = decode(&buf);
        assert!(
            result.is_err(),
            "invalid type tag 0xFF must produce Err, got: {:?}",
            result
        );
        let msg = result.unwrap_err().to_string().to_lowercase();
        assert!(
            msg.contains("tag") || msg.contains("invalid") || msg.contains("type"),
            "error must mention type/tag, got: {}",
            msg
        );
    }

    // -------------------------------------------------------------------------
    // format_to_tauq -> compile_tauq roundtrip for a JSON value with nulls
    // -------------------------------------------------------------------------

    /// Encoding a JSON object that contains null fields through format_to_tauq
    /// and then re-parsing with compile_tauq must recover the original value.
    #[test]
    fn test_null_format_roundtrip() {
        use serde_json::json;
        use tauq::{compile_tauq, format_to_tauq};

        let original = json!({
            "a": null,
            "b": 1,
            "c": null
        });

        let tauq_str = format_to_tauq(&original);
        let recovered =
            compile_tauq(&tauq_str).expect("compile_tauq must succeed after format_to_tauq");

        // The recovered object must contain null for keys a and c.
        assert!(
            recovered["a"].is_null(),
            "field 'a' must be null after roundtrip"
        );
        assert_eq!(recovered["b"], 1);
        assert!(
            recovered["c"].is_null(),
            "field 'c' must be null after roundtrip"
        );
    }
}
