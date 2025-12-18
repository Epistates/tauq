//! Real-world system event logs dataset for codec testing
//!
//! Simulates production system logs with:
//! - Sequential timestamps (delta optimal)
//! - Repeated hostnames (dictionary optimal)
//! - Repeated service names (dictionary optimal)
//! - Repeated event types (dictionary optimal)
//! - Severity levels (RLE/dictionary optimal)
//! - Variable message lengths (raw or RLE)
//! - Duration metrics (delta optimal for patterns)

use serde_json::{json, Value};
use rand::Rng;

/// Common hostnames in a data center
const HOSTNAMES: &[&str] = &[
    "web-server-01", "web-server-02", "web-server-03", "web-server-04", "web-server-05",
    "api-server-01", "api-server-02", "api-server-03", "database-01", "database-02",
    "cache-01", "cache-02", "load-balancer-01", "load-balancer-02", "search-01",
    "message-queue-01", "message-queue-02", "scheduler-01", "worker-01", "worker-02",
    "worker-03", "worker-04", "worker-05", "backup-01", "monitoring-01",
    "auth-server-01", "auth-server-02", "api-gateway-01", "cdn-01", "cdn-02",
    "storage-01", "storage-02", "config-server-01", "logging-01", "metrics-01",
    "web-cache-01", "web-cache-02", "session-store-01", "session-store-02", "redis-01",
    "postgres-01", "postgres-02", "mongo-01", "elastic-01", "rabbit-01",
    "router-01", "switch-01", "firewall-01", "vpn-01", "dns-01",
];

/// Service names
const SERVICES: &[&str] = &[
    "api", "web", "auth", "database", "cache",
    "scheduler", "worker", "notification", "analytics", "search",
    "storage", "monitoring", "logging", "config", "messaging",
    "billing", "payment", "inventory", "order", "user",
    "product", "recommendation", "recommendation-engine", "ml-service", "recommendation-api",
    "report-service", "export-service", "import-service", "sync-service", "cleanup-service",
];

/// Event types
const EVENT_TYPES: &[&str] = &[
    "started", "stopped", "error", "warning", "info",
    "debug", "request", "response", "timeout", "retry",
    "failed", "success", "connected", "disconnected", "heartbeat",
    "metrics", "alert", "escalation", "resolved", "pending",
    "queued", "processing", "completed", "abandoned", "rollback",
    "deployed", "rolled_back", "scaled_up", "scaled_down", "restarted",
];

/// Severity levels
#[derive(Clone, Copy)]
enum Severity {
    Debug = 0,
    Info = 1,
    Warning = 2,
    Error = 3,
    Critical = 4,
}

impl Severity {
    fn as_str(&self) -> &'static str {
        match self {
            Severity::Debug => "DEBUG",
            Severity::Info => "INFO",
            Severity::Warning => "WARNING",
            Severity::Error => "ERROR",
            Severity::Critical => "CRITICAL",
        }
    }
}

/// Generate realistic system event logs
///
/// # Arguments
/// * `count` - Number of log entries to generate
///
/// # Returns
/// Vec of log entry JSON values with realistic patterns
pub fn generate_event_logs(count: usize) -> Vec<Value> {
    let mut rng = rand::thread_rng();
    let base_timestamp = 1766534400i64;  // Dec 17, 2025 00:00:00 UTC

    (0..count)
        .map(|i| {
            let hostname_idx = rng.gen_range(0..HOSTNAMES.len());
            let service_idx = rng.gen_range(0..SERVICES.len());
            let event_type_idx = rng.gen_range(0..EVENT_TYPES.len());

            // Severity distribution: mostly info/debug, rare critical
            let severity = if rng.gen_bool(0.001) {
                Severity::Critical
            } else if rng.gen_bool(0.01) {
                Severity::Error
            } else if rng.gen_bool(0.05) {
                Severity::Warning
            } else if rng.gen_bool(0.6) {
                Severity::Info
            } else {
                Severity::Debug
            };

            // Duration varies, but often clustered (good for delta)
            let duration_ms = match severity {
                Severity::Error | Severity::Critical => rng.gen_range(50..5000),
                Severity::Warning => rng.gen_range(20..1000),
                _ => rng.gen_range(1..500),
            };

            // Message varies by severity
            let message = match severity {
                Severity::Critical => {
                    format!("CRITICAL: {} service on {} failed with error code {}",
                            SERVICES[service_idx], HOSTNAMES[hostname_idx],
                            rng.gen_range(500..599))
                }
                Severity::Error => {
                    format!("Error processing request: {} ({}ms timeout)",
                            ["Connection timeout", "Database unavailable", "Parse error"][rng.gen_range(0..3)],
                            duration_ms)
                }
                Severity::Warning => {
                    format!("High latency detected: {}ms for {}", duration_ms, EVENT_TYPES[event_type_idx])
                }
                Severity::Info => {
                    format!("{} {} on {} ({}ms)",
                            EVENT_TYPES[event_type_idx], SERVICES[service_idx],
                            HOSTNAMES[hostname_idx], duration_ms)
                }
                Severity::Debug => {
                    format!("Debug: {} execution trace for {}",
                            SERVICES[service_idx], HOSTNAMES[hostname_idx])
                }
            };

            // Log entry structure matches real-world logging frameworks
            json!({
                "timestamp": base_timestamp + (i as i64 * 5),  // 5 seconds apart on average
                "hostname": HOSTNAMES[hostname_idx],
                "service": SERVICES[service_idx],
                "event_type": EVENT_TYPES[event_type_idx],
                "severity": severity.as_str(),
                "message": message,
                "duration_ms": duration_ms,
                "request_id": format!("req_{:012x}", rng.gen_range(0u64..1000000000000)),
                "user_id": rng.gen_range(1..10000),
                "status_code": match severity {
                    Severity::Error | Severity::Critical => rng.gen_range(400..599),
                    _ => 200,
                },
            })
        })
        .collect()
}

/// Generate logs with specific seed
pub fn generate_event_logs_with_seed(count: usize, seed: u64) -> Vec<Value> {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let base_timestamp = 1766534400i64;

    (0..count)
        .map(|i| {
            let hostname = HOSTNAMES[rng.gen_range(0..HOSTNAMES.len())];
            let service = SERVICES[rng.gen_range(0..SERVICES.len())];
            let event_type = EVENT_TYPES[rng.gen_range(0..EVENT_TYPES.len())];

            let severity = if rng.gen_bool(0.001) {
                Severity::Critical
            } else if rng.gen_bool(0.01) {
                Severity::Error
            } else if rng.gen_bool(0.05) {
                Severity::Warning
            } else if rng.gen_bool(0.6) {
                Severity::Info
            } else {
                Severity::Debug
            };

            let duration_ms = match severity {
                Severity::Error | Severity::Critical => rng.gen_range(50..5000),
                Severity::Warning => rng.gen_range(20..1000),
                _ => rng.gen_range(1..500),
            };

            json!({
                "timestamp": base_timestamp + (i as i64 * 5),
                "hostname": hostname,
                "service": service,
                "event_type": event_type,
                "severity": severity.as_str(),
                "message": format!("{} on {}", event_type, hostname),
                "duration_ms": duration_ms,
                "request_id": format!("req_{:012x}", i),
                "user_id": rng.gen_range(1..10000),
                "status_code": match severity {
                    Severity::Error | Severity::Critical => rng.gen_range(400..599),
                    _ => 200,
                },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_event_logs_small() {
        let data = generate_event_logs(100);
        assert_eq!(data.len(), 100);

        // Verify structure
        for log in &data {
            assert!(log["timestamp"].is_i64());
            assert!(log["hostname"].is_string());
            assert!(log["service"].is_string());
            assert!(log["event_type"].is_string());
            assert!(log["severity"].is_string());
            assert!(log["message"].is_string());
            assert!(log["duration_ms"].is_i64());
            assert!(log["status_code"].is_i64());
        }
    }

    #[test]
    fn test_logs_temporal_ordering() {
        let data = generate_event_logs(1000);

        // Verify chronological ordering
        let mut prev_ts = i64::MIN;
        for log in &data {
            let ts = log["timestamp"].as_i64().unwrap();
            assert!(ts >= prev_ts);
            prev_ts = ts;
        }
    }

    #[test]
    fn test_logs_cardinality() {
        let data = generate_event_logs(10000);

        // Count unique values
        let hostnames: std::collections::HashSet<_> = data.iter()
            .map(|l| l["hostname"].as_str().unwrap())
            .collect();

        let services: std::collections::HashSet<_> = data.iter()
            .map(|l| l["service"].as_str().unwrap())
            .collect();

        let severities: std::collections::HashSet<_> = data.iter()
            .map(|l| l["severity"].as_str().unwrap())
            .collect();

        // Cardinality good for dictionary encoding
        assert_eq!(hostnames.len(), HOSTNAMES.len());
        assert_eq!(services.len(), SERVICES.len());
        assert_eq!(severities.len(), 5);  // 5 severity levels
    }

    #[test]
    fn test_logs_severity_distribution() {
        let data = generate_event_logs(10000);

        let mut severity_counts = std::collections::HashMap::new();
        for log in &data {
            *severity_counts.entry(log["severity"].as_str().unwrap().to_string())
                .or_insert(0) += 1;
        }

        // Debug/Info should dominate (> 50%)
        let info_debug_count = severity_counts.get("INFO").copied().unwrap_or(0)
                             + severity_counts.get("DEBUG").copied().unwrap_or(0);
        assert!(info_debug_count as f64 / data.len() as f64 > 0.5);

        // Critical should be rare (< 0.2%)
        let critical_count = severity_counts.get("CRITICAL").copied().unwrap_or(0);
        assert!(critical_count as f64 / data.len() as f64 < 0.01);
    }

    #[test]
    fn test_logs_delta_optimal_timestamps() {
        let data = generate_event_logs(100);

        // Timestamps should be mostly ordered with small deltas
        let mut deltas = Vec::new();
        let mut prev_ts = data[0]["timestamp"].as_i64().unwrap();

        for log in &data[1..] {
            let ts = log["timestamp"].as_i64().unwrap();
            if ts >= prev_ts {
                deltas.push(ts - prev_ts);
            }
            prev_ts = ts;
        }

        // Most deltas should be 5 (the typical 5-second increment)
        let five_second_deltas = deltas.iter().filter(|d| **d == 5).count();
        assert!(five_second_deltas as f64 / deltas.len() as f64 > 0.5);
    }

    #[test]
    fn test_logs_reproducible() {
        let data1 = generate_event_logs_with_seed(1000, 42);
        let data2 = generate_event_logs_with_seed(1000, 42);

        for (d1, d2) in data1.iter().zip(data2.iter()) {
            assert_eq!(d1["timestamp"], d2["timestamp"]);
            assert_eq!(d1["hostname"], d2["hostname"]);
            assert_eq!(d1["service"], d2["service"]);
        }
    }
}
