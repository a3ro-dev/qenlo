//! Stable, privacy-safe wire contract shared by every Qenlo device tester.

extern crate self as qenlo_testkit;

use serde::{Deserialize, Serialize};

#[cfg(feature = "runner")]
#[path = "main.rs"]
#[allow(dead_code)]
mod runner;

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_RUN_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TestRun {
    pub schema_version: u32,
    pub run_id: String,
    pub install_id: String,
    pub started_at_unix_ms: u64,
    pub completed_at_unix_ms: u64,
    pub app_version: String,
    pub target: String,
    pub os: String,
    pub os_version: String,
    pub cpu_arch: String,
    pub cpu_name: String,
    pub gpu_name: Option<String>,
    pub gpu_api: Option<String>,
    pub power_source: Option<String>,
    pub thermal_state: Option<String>,
    pub suite: String,
    pub cells: Vec<TestCell>,
    pub failures: Vec<TestFailure>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TestCell {
    pub name: String,
    pub backend_requested: String,
    pub backend_actual: String,
    pub algorithm: String,
    pub rows: u64,
    pub dimensions: u32,
    pub eligible_fraction: f64,
    pub batch_size: u32,
    pub k: u32,
    pub samples: u32,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
    pub recall_at_k: f64,
    pub upload_bytes: Option<u64>,
    pub readback_bytes: Option<u64>,
    pub allocation_bytes: Option<u64>,
    pub dispatch_count: Option<u64>,
    pub routing_reason: Option<String>,
    pub fallback_reason: Option<String>,
    pub passed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TestFailure {
    pub stage: String,
    pub code: String,
    pub message: String,
}

impl TestRun {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != SCHEMA_VERSION {
            return Err("unsupported schema_version");
        }
        for value in [
            &self.run_id,
            &self.install_id,
            &self.app_version,
            &self.target,
            &self.os,
            &self.cpu_arch,
            &self.suite,
        ] {
            if value.is_empty() || value.len() > 256 {
                return Err("required string is empty or too long");
            }
        }
        if self.completed_at_unix_ms < self.started_at_unix_ms
            || self.completed_at_unix_ms > i64::MAX as u64
            || self.cells.len() > 512
        {
            return Err("invalid time range or too many cells");
        }
        if self.cells.iter().any(|cell| {
            cell.name.is_empty()
                || cell.name.len() > 256
                || cell.samples == 0
                || cell.dimensions == 0
                || cell.batch_size == 0
                || cell.k == 0
                || !(0.0..=1.0).contains(&cell.eligible_fraction)
                || !(0.0..=1.0).contains(&cell.recall_at_k)
        }) {
            return Err("invalid test cell");
        }
        Ok(())
    }
}

/// Run the same native suite used by the desktop CLI. Mobile shells call this on a worker thread.
#[cfg(feature = "runner")]
pub fn run_profile(profile: &str) -> Result<TestRun, String> {
    runner::block_on(runner::run_suite(profile)).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_wire_fields_and_invalid_recall() {
        let json = r#"{"schema_version":1,"run_id":"r","install_id":"i","started_at_unix_ms":1,"completed_at_unix_ms":2,"app_version":"v","target":"t","os":"o","os_version":"","cpu_arch":"a","cpu_name":"c","gpu_name":null,"gpu_api":null,"power_source":null,"thermal_state":null,"suite":"s","cells":[],"failures":[],"secret":"no"}"#;
        assert!(serde_json::from_str::<TestRun>(json).is_err());
    }
}
