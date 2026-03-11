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
                id: i,
                name: format!("Employee{}", i),
                age: 25 + (i % 40),
                city: ["NYC", "LA", "Chicago", "Houston", "Phoenix"][i as usize % 5].to_string(),
                department: ["Engineering", "Sales", "Marketing", "HR", "Finance"][i as usize % 5]
                    .to_string(),
                salary: 50000 + (i * 100),
            })
            .collect();

        let json = serde_json::to_string(&employees).unwrap();
        let generic_tbf = tauq::tbf::to_bytes(&employees).unwrap();

        let generic_pct = (generic_tbf.len() as f64 / json.len() as f64) * 100.0;
        let generic_reduction =
            ((json.len() - generic_tbf.len()) as f64 / json.len() as f64) * 100.0;

        println!("\n=== Generic Serde Encoding (1000 records) ===");
        println!("JSON size:           {} bytes", json.len());
        println!(
            "TBF (generic serde): {} bytes ({:.1}%)",
            generic_tbf.len(),
            generic_pct
        );
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
        // This test validates the documented compression tiers using representative data.
        // Generic serde encoding (no schema hints) should achieve meaningful reduction
        // over raw JSON on a dataset large enough for dictionary/columnar benefits to
        // overcome fixed header overhead.
        let employees: Vec<Employee> = (1..=500)
            .map(|i| Employee {
                id: i,
                name: format!("Employee{}", i),
                age: 25 + (i % 40),
                city: ["NYC", "LA", "Chicago", "Houston", "Phoenix"][i as usize % 5].to_string(),
                department: ["Engineering", "Sales", "Marketing", "HR", "Finance"][i as usize % 5]
                    .to_string(),
                salary: 50000 + (i * 100),
            })
            .collect();

        let json = serde_json::to_string(&employees).unwrap();
        let tbf = tauq::tbf::to_bytes(&employees).unwrap();

        // TBF must be strictly smaller than JSON for generic encoding on this dataset.
        assert!(
            tbf.len() < json.len(),
            "TBF ({} bytes) should be smaller than JSON ({} bytes) for generic encoding",
            tbf.len(),
            json.len()
        );

        // Generic encoding should achieve at least 25% reduction (conservative lower bound).
        let reduction = ((json.len() - tbf.len()) as f64 / json.len() as f64) * 100.0;
        assert!(
            reduction >= 25.0,
            "Generic TBF should achieve at least 25% size reduction over JSON, got {:.1}%",
            reduction
        );

        // Generic encoding should not exceed 75% of JSON size (i.e. at least 25% reduction).
        let pct = (tbf.len() as f64 / json.len() as f64) * 100.0;
        assert!(
            pct < 75.0,
            "Generic TBF should be under 75% of JSON size, got {:.1}%",
            pct
        );
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
                id: i,
                name: format!("Employee{}", i),
                age: 25 + (i % 40),
                city: ["NYC", "LA", "Chicago", "Houston", "Phoenix"][i as usize % 5].to_string(),
                department: ["Engineering", "Sales", "Marketing", "HR", "Finance"][i as usize % 5]
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
                id: i,
                name: format!("Employee{}", i),
                age: 25 + (i % 40),
                city: ["NYC", "LA", "Chicago", "Houston", "Phoenix"][i as usize % 5].to_string(),
                department: ["Engineering", "Sales", "Marketing", "HR", "Finance"][i as usize % 5]
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
