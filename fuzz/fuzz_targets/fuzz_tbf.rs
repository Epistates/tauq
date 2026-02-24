#![no_main]

use libfuzzer_sys::fuzz_target;
use tauq::tbf::{decode, decode_to_tauq};

fuzz_target!(|data: &[u8]| {
    // 1. Fuzz the Binary Decoder -> JSON
    if let Ok(json) = decode(data) {
        // If it decodes successfully, ensure we can re-encode it
        // (This catches "impossible" values that deserialize but can't serialize)
        let _ = tauq::tbf::encode_json(&json);
    }

    // 2. Fuzz the Binary Decoder -> Tauq Text
    // This exercises a different path (tbf::decode_to_tauq)
    let _ = decode_to_tauq(data);
});
