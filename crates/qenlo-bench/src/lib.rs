//! Independent correctness and reporting support for Qenlo benchmarks.
//!
//! The oracle never calls Qenlo's scoring or filtering implementation. The CLI
//! adapts public API results into these backend-neutral types.

pub mod dataset;
#[cfg(feature = "otlp")]
pub mod telemetry;

use std::{collections::HashSet, error::Error, fmt, time::Duration};

#[derive(Clone, Debug)]
pub struct OracleRecord {
    pub id: u64,
    pub user_id: u64,
    pub timestamp_micros: i64,
    pub vector: Vec<f32>,
    pub deleted: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OracleFilter {
    pub user_id: Option<u64>,
    /// Inclusive lower bound.
    pub timestamp_from: Option<i64>,
    /// Exclusive upper bound.
    pub timestamp_to: Option<i64>,
}

impl OracleFilter {
    fn matches(self, record: &OracleRecord) -> bool {
        self.user_id.is_none_or(|id| id == record.user_id)
            && self
                .timestamp_from
                .is_none_or(|from| record.timestamp_micros >= from)
            && self
                .timestamp_to
                .is_none_or(|to| record.timestamp_micros < to)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OracleHit {
    pub id: u64,
    pub distance: f64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleError {
    EmptyDimension,
    InvalidK,
    InvalidFilterRange,
    DimensionMismatch {
        id: u64,
        expected: usize,
        actual: usize,
    },
    DuplicateId(u64),
    NonFinite {
        id: Option<u64>,
    },
    ZeroNorm {
        id: Option<u64>,
    },
}

impl fmt::Display for OracleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for OracleError {}

/// Exhaustive cosine search using f64 arithmetic and deterministic distance/ID ordering.
pub fn exact_cosine_search(
    records: &[OracleRecord],
    query: &[f32],
    filter: OracleFilter,
    k: usize,
) -> Result<Vec<OracleHit>, OracleError> {
    if k == 0 {
        return Err(OracleError::InvalidK);
    }
    if filter
        .timestamp_from
        .zip(filter.timestamp_to)
        .is_some_and(|(from, to)| from > to)
    {
        return Err(OracleError::InvalidFilterRange);
    }
    let query_norm = checked_norm(query, None)?;
    let mut ids = HashSet::with_capacity(records.len());
    let mut hits = Vec::new();

    for record in records {
        if !ids.insert(record.id) {
            return Err(OracleError::DuplicateId(record.id));
        }
        if record.vector.len() != query.len() {
            return Err(OracleError::DimensionMismatch {
                id: record.id,
                expected: query.len(),
                actual: record.vector.len(),
            });
        }
        let record_norm = checked_norm(&record.vector, Some(record.id))?;
        if record.deleted || !filter.matches(record) {
            continue;
        }
        let dot = query
            .iter()
            .zip(&record.vector)
            .map(|(&a, &b)| f64::from(a) * f64::from(b))
            .sum::<f64>();
        let similarity = (dot / (query_norm * record_norm)).clamp(-1.0, 1.0);
        hits.push(OracleHit {
            id: record.id,
            distance: 1.0 - similarity,
        });
    }

    hits.sort_unstable_by(|a, b| a.distance.total_cmp(&b.distance).then(a.id.cmp(&b.id)));
    hits.truncate(k);
    Ok(hits)
}

fn checked_norm(vector: &[f32], id: Option<u64>) -> Result<f64, OracleError> {
    if vector.is_empty() {
        return Err(OracleError::EmptyDimension);
    }
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(OracleError::NonFinite { id });
    }
    let squared = vector
        .iter()
        .map(|&value| {
            let value = f64::from(value);
            value * value
        })
        .sum::<f64>();
    if squared == 0.0 {
        return Err(OracleError::ZeroNorm { id });
    }
    Ok(squared.sqrt())
}

/// Recall over unique IDs, with the denominator capped by available ground truth.
pub fn recall_at_k(expected: &[u64], actual: &[u64], k: usize) -> Result<f64, OracleError> {
    if k == 0 {
        return Err(OracleError::InvalidK);
    }
    let expected: HashSet<_> = expected.iter().take(k).copied().collect();
    if expected.is_empty() {
        return Ok(1.0);
    }
    let actual: HashSet<_> = actual.iter().take(k).copied().collect();
    Ok(expected.intersection(&actual).count() as f64 / expected.len() as f64)
}

/// The nearest-rank percentile: rank = ceil(p * n), with p in (0, 1].
pub fn nearest_rank_percentile(samples: &[Duration], percentile: f64) -> Option<Duration> {
    if samples.is_empty() || !(0.0 < percentile && percentile <= 1.0) {
        return None;
    }
    let mut ordered = samples.to_vec();
    ordered.sort_unstable();
    let rank = (percentile * ordered.len() as f64).ceil() as usize;
    ordered.get(rank.saturating_sub(1)).copied()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SyntheticMetadata {
    pub user_id: u64,
    pub timestamp_micros: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataDistribution {
    Independent,
    PositivelyCorrelated,
    NegativelyCorrelated,
    Skewed,
}

impl MetadataDistribution {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Independent => "synthetic-independent",
            Self::PositivelyCorrelated => "synthetic-positive-correlation",
            Self::NegativelyCorrelated => "synthetic-negative-correlation",
            Self::Skewed => "synthetic-skewed",
        }
    }
}

/// Generates labelled synthetic metadata without an RNG dependency.
pub fn synthetic_metadata(
    rows: usize,
    user_count: u64,
    seed: u64,
    distribution: MetadataDistribution,
) -> Vec<SyntheticMetadata> {
    assert!(user_count > 0, "user_count must be non-zero");
    let mut rng = SplitMix64(seed);
    (0..rows)
        .map(|_| {
            let a = unit(rng.next());
            let b = unit(rng.next());
            let (user_position, time_position) = match distribution {
                MetadataDistribution::Independent => (a, b),
                MetadataDistribution::PositivelyCorrelated => (a, (a + b * 0.05).min(1.0)),
                MetadataDistribution::NegativelyCorrelated => (a, (1.0 - a + b * 0.05).min(1.0)),
                MetadataDistribution::Skewed => (a * a, b * b * b),
            };
            SyntheticMetadata {
                user_id: ((user_position * user_count as f64) as u64).min(user_count - 1),
                timestamp_micros: position_to_timestamp(time_position),
            }
        })
        .collect()
}

fn unit(value: u64) -> f64 {
    (value >> 11) as f64 * (1.0 / ((1_u64 << 53) as f64))
}

fn position_to_timestamp(position: f64) -> i64 {
    const SPAN: i64 = 2_000_000_000_000;
    -SPAN / 2 + (position * SPAN as f64) as i64
}

struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BenchmarkSample {
    pub query_index: usize,
    pub latency: Duration,
    pub result_count: usize,
    pub eligible_count: Option<usize>,
    pub upload_bytes: Option<u64>,
    pub readback_bytes: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunReport {
    /// Raw samples are retained so summaries can be independently recomputed.
    pub samples: Vec<BenchmarkSample>,
    pub p50: Option<Duration>,
    pub p95: Option<Duration>,
    pub p99: Option<Duration>,
    /// Wall time for the measured query window, excluding warmups and readiness.
    pub measured_wall_time: Option<Duration>,
}

impl RunReport {
    pub fn from_samples(samples: Vec<BenchmarkSample>) -> Self {
        let latencies: Vec<_> = samples.iter().map(|sample| sample.latency).collect();
        Self {
            p50: nearest_rank_percentile(&latencies, 0.50),
            p95: nearest_rank_percentile(&latencies, 0.95),
            p99: nearest_rank_percentile(&latencies, 0.99),
            measured_wall_time: None,
            samples,
        }
    }

    pub fn measured_qps(&self) -> Option<f64> {
        let seconds = self.measured_wall_time?.as_secs_f64();
        (seconds > 0.0).then(|| self.samples.len() as f64 / seconds)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkReport {
    pub workload: String,
    pub backend: String,
    pub platform: String,
    pub dataset_checksum: Option<String>,
    pub dimensions: usize,
    pub row_count: usize,
    pub eligible_fraction: f64,
    pub batch_size: usize,
    pub k: usize,
    pub build_time: Option<Duration>,
    pub readiness_time: Option<Duration>,
    pub host_memory_bytes: Option<u64>,
    pub gpu_allocation_bytes: Option<u64>,
    pub recall_at_10: Option<f64>,
    pub runs: Vec<RunReport>,
}

impl BenchmarkReport {
    /// Median of each run's nearest-rank P95, as required by the benchmark protocol.
    pub fn median_run_p95(&self) -> Option<Duration> {
        let mut values: Vec<_> = self.runs.iter().filter_map(|run| run.p95).collect();
        if values.is_empty() {
            return None;
        }
        values.sort_unstable();
        Some(values[(values.len() - 1) / 2])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: u64, user_id: u64, time: i64, vector: &[f32]) -> OracleRecord {
        OracleRecord {
            id,
            user_id,
            timestamp_micros: time,
            vector: vector.to_vec(),
            deleted: false,
        }
    }

    #[test]
    fn oracle_filters_deletions_and_breaks_ties_by_id() {
        let mut deleted = record(1, 7, 10, &[1.0, 0.0]);
        deleted.deleted = true;
        let records = [
            record(3, 7, 10, &[1.0, 0.0]),
            record(2, 7, 11, &[1.0, 0.0]),
            record(4, 8, 10, &[1.0, 0.0]),
            deleted,
        ];
        let filter = OracleFilter {
            user_id: Some(7),
            timestamp_from: Some(10),
            timestamp_to: Some(11),
        };
        let hits = exact_cosine_search(&records, &[1.0, 0.0], filter, 10).unwrap();
        assert_eq!(hits.iter().map(|hit| hit.id).collect::<Vec<_>>(), [3]);
    }

    #[test]
    fn recall_and_nearest_rank_handle_small_samples() {
        assert_eq!(recall_at_k(&[1, 2, 3], &[3, 8, 2], 3).unwrap(), 2.0 / 3.0);
        assert_eq!(recall_at_k(&[], &[], 10).unwrap(), 1.0);
        let samples = [1, 100, 2, 3].map(Duration::from_millis);
        assert_eq!(
            nearest_rank_percentile(&samples, 0.95),
            Some(Duration::from_millis(100))
        );
    }

    #[test]
    fn metadata_is_deterministic_and_distributions_differ() {
        let first = synthetic_metadata(64, 8, 42, MetadataDistribution::Independent);
        assert_eq!(
            first,
            synthetic_metadata(64, 8, 42, MetadataDistribution::Independent)
        );
        assert_ne!(
            first,
            synthetic_metadata(64, 8, 42, MetadataDistribution::Skewed)
        );
        assert!(first.iter().all(|row| row.user_id < 8));
    }

    #[test]
    fn report_retains_samples_and_uses_lower_median_for_five_runs() {
        let report = BenchmarkReport {
            workload: "test".into(),
            backend: "cpu".into(),
            platform: "test".into(),
            dataset_checksum: None,
            dimensions: 2,
            row_count: 1,
            eligible_fraction: 1.0,
            batch_size: 1,
            k: 10,
            build_time: None,
            readiness_time: None,
            host_memory_bytes: None,
            gpu_allocation_bytes: None,
            recall_at_10: Some(1.0),
            runs: [5, 1, 4, 2, 3]
                .map(|millis| {
                    RunReport::from_samples(vec![BenchmarkSample {
                        query_index: 0,
                        latency: Duration::from_millis(millis),
                        result_count: 10,
                        eligible_count: None,
                        upload_bytes: None,
                        readback_bytes: None,
                    }])
                })
                .into(),
        };
        assert_eq!(report.median_run_p95(), Some(Duration::from_millis(3)));
        assert_eq!(report.runs[0].samples.len(), 1);
    }
}
