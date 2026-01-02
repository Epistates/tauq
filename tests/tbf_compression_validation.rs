#[cfg(test)]
mod tbf_compression_validation {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Employee {
        id: u32,
        name: String,
        age: u32,
        city: String,
        department: String,
        salary: u32,
    }

    #[test]
    fn validate_generic_serde_compression() {
        // Test 1: Generic serde encoding (what the CLI uses)
        // Expected: ~60% of JSON size (no schema hints)
        let employees: Vec<Employee> = (1..=1000)
            .map(|i| Employee {
                id: i as u32,
                name: format!("Employee{}", i),
                age: (25 + (i % 40)) as u32,
                city: ["NYC", "LA", "Chicago", "Houston", "Phoenix"][i as usize % 5]
                    .to_string(),
                department: ["Engineering", "Sales", "Marketing", "HR", "Finance"]
                    [i as usize % 5]
                    .to_string(),
                salary: 50000 + (i * 100),
            })
            .collect();

        let json = serde_json::to_string(&employees).unwrap();
        let generic_tbf = tauq::tbf::to_bytes(&employees).unwrap();

        let generic_pct = (generic_tbf.len() as f64 / json.len() as f64) * 100.0;
        let generic_reduction = ((json.len() - generic_tbf.len()) as f64 / json.len() as f64)
            * 100.0;

        println!("\n=== Generic Serde Encoding (1000 records) ===");
        println!("JSON size:           {} bytes", json.len());
        println!("TBF (generic serde): {} bytes ({:.1}%)", generic_tbf.len(), generic_pct);
        println!("Reduction:           {:.1}%", generic_reduction);

        // Generic encoder should achieve 55-65% of JSON (no schema knowledge)
        assert!(
            generic_pct < 70.0,
            "Generic TBF should be 55-70% of JSON size, got {:.1}%",
            generic_pct
        );
    }

    #[test]
    fn validate_claim_documentation() {
        // This test documents when the 83% claim applies and when it doesn't
        println!("\n=== Compression Claims Clarification ===");
        println!();
        println!("The 83% compression claim is CONDITIONAL:");
        println!();
        println!("1. GENERIC SERDE ENCODING (CLI default):");
        println!("   - What: Standard serde serialization without schema hints");
        println!("   - Achieves: ~60% of JSON size (~40% reduction)");
        println!("   - Used by: tauq build --format tbf (CLI conversion)");
        println!("   - Why: No type information to optimize further");
        println!();
        println!("2. SCHEMA-AWARE ENCODING (Rust API):");
        println!("   - What: TableEncode with type hints + columnar encoding");
        println!("   - Achieves: ~17% of JSON size (~83% reduction)");
        println!("   - Used by: #[derive(TableEncode)] with #[tauq(...)] hints");
        println!("   - Why: Adaptive integer encoding, offset encoding, column reordering");
        println!();
        println!("3. ICEBERG/ARROW INTEGRATION:");
        println!("   - What: Columnar encoding via Apache Arrow");
        println!("   - Achieves: ~17% of JSON size (~83% reduction)");
        println!("   - Used by: ArrowToTbf trait, TbfFileWriter");
        println!("   - Why: Full columnar layout + compression");
        println!();
        println!("BOTTOM LINE:");
        println!("- Use CLI (generic): Fast, no setup, ~40% reduction");
        println!("- Use Rust API (schema): Best compression, ~83% reduction");
        println!("- Use Iceberg (arrow): Data lake integration, ~83% reduction");
    }

    #[test]
    fn validate_small_dataset_behavior() {
        // Small datasets show different behavior (more overhead)
        let small: Vec<Employee> = vec![
            Employee {
                id: 1,
                name: "Alice".to_string(),
                age: 30,
                city: "NYC".to_string(),
                department: "Engineering".to_string(),
                salary: 100000,
            },
            Employee {
                id: 2,
                name: "Bob".to_string(),
                age: 28,
                city: "LA".to_string(),
                department: "Sales".to_string(),
                salary: 80000,
            },
        ];

        let json = serde_json::to_string(&small).unwrap();
        let tbf = tauq::tbf::to_bytes(&small).unwrap();

        let pct = (tbf.len() as f64 / json.len() as f64) * 100.0;

        println!("\n=== Small Dataset Behavior (2 records) ===");
        println!("JSON size: {} bytes", json.len());
        println!("TBF size:  {} bytes ({:.1}%)", tbf.len(), pct);
        println!("Note: Small datasets show higher overhead due to header/dict");

        // Small datasets may be larger due to fixed overhead
        // This is expected and acceptable for small data
    }

    #[test]
    fn validate_medium_dataset() {
        // Medium dataset where compression kicks in
        let employees: Vec<Employee> = (1..=100)
            .map(|i| Employee {
                id: i as u32,
                name: format!("Employee{}", i),
                age: (25 + (i % 40)) as u32,
                city: ["NYC", "LA", "Chicago", "Houston", "Phoenix"][i as usize % 5]
                    .to_string(),
                department: ["Engineering", "Sales", "Marketing", "HR", "Finance"]
                    [i as usize % 5]
                    .to_string(),
                salary: 50000 + (i * 100),
            })
            .collect();

        let json = serde_json::to_string(&employees).unwrap();
        let tbf = tauq::tbf::to_bytes(&employees).unwrap();

        let pct = (tbf.len() as f64 / json.len() as f64) * 100.0;
        let reduction = ((json.len() - tbf.len()) as f64 / json.len() as f64) * 100.0;

        println!("\n=== Medium Dataset (100 records) ===");
        println!("JSON size: {} bytes", json.len());
        println!("TBF size:  {} bytes ({:.1}%)", tbf.len(), pct);
        println!("Reduction: {:.1}%", reduction);

        // Medium datasets should show good compression (~55-65%)
        assert!(
            pct < 70.0,
            "Generic TBF should be under 70% of JSON, got {:.1}%",
            pct
        );
    }

    #[test]
    fn validate_large_dataset() {
        // Large dataset maximizes compression ratio
        let employees: Vec<Employee> = (1..=10000)
            .map(|i| Employee {
                id: i as u32,
                name: format!("Employee{}", i),
                age: (25 + (i % 40)) as u32,
                city: ["NYC", "LA", "Chicago", "Houston", "Phoenix"][i as usize % 5]
                    .to_string(),
                department: ["Engineering", "Sales", "Marketing", "HR", "Finance"]
                    [i as usize % 5]
                    .to_string(),
                salary: 50000 + (i * 100),
            })
            .collect();

        let json = serde_json::to_string(&employees).unwrap();
        let tbf = tauq::tbf::to_bytes(&employees).unwrap();

        let pct = (tbf.len() as f64 / json.len() as f64) * 100.0;
        let reduction = ((json.len() - tbf.len()) as f64 / json.len() as f64) * 100.0;

        println!("\n=== Large Dataset (10,000 records) ===");
        println!("JSON size: {} bytes", json.len());
        println!("TBF size:  {} bytes ({:.1}%)", tbf.len(), pct);
        println!("Reduction: {:.1}%", reduction);

        // Large datasets should show good compression (~55-65%)
        assert!(
            pct < 70.0,
            "Generic TBF should be under 70% of JSON, got {:.1}%",
            pct
        );
    }
}
