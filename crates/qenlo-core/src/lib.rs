//! Portable canonical storage and exact filtered vector search for Qenlo.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap};
use std::fmt;

/// Inclusive-lower, exclusive-upper timestamp range.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TimestampRange {
    pub lower: Option<i64>,
    pub upper: Option<i64>,
}

impl TimestampRange {
    pub const ALL: Self = Self {
        lower: None,
        upper: None,
    };

    pub const fn new(lower: Option<i64>, upper: Option<i64>) -> Self {
        Self { lower, upper }
    }

    fn contains(self, timestamp: i64) -> bool {
        self.lower.is_none_or(|lower| timestamp >= lower)
            && self.upper.is_none_or(|upper| timestamp < upper)
    }

    fn is_empty(self) -> bool {
        matches!((self.lower, self.upper), (Some(lower), Some(upper)) if lower >= upper)
    }
}

/// The prototype's fixed metadata predicate: all supplied clauses are joined by AND.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Predicate {
    pub user_id: Option<u64>,
    pub timestamp: TimestampRange,
}

impl Predicate {
    pub const ALL: Self = Self {
        user_id: None,
        timestamp: TimestampRange::ALL,
    };

    pub const fn new(user_id: Option<u64>, timestamp: TimestampRange) -> Self {
        Self { user_id, timestamp }
    }
}

/// A canonical row. Vectors are finite, non-zero, and unit-normalized.
#[derive(Clone, Debug)]
pub struct Record {
    id: u64,
    user_id: u64,
    timestamp: i64,
    vector: Vec<f32>,
    live: bool,
}

impl Record {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }

    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn vector(&self) -> &[f32] {
        &self.vector
    }

    pub fn is_live(&self) -> bool {
        self.live
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SearchHit {
    pub id: u64,
    pub distance: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SearchOutput {
    pub hits: Vec<SearchHit>,
    pub evaluated_rows: usize,
}

/// Disposable symmetric scalar quantization for ANN candidate generation.
/// Canonical vectors remain FP32 and should re-rank final candidates.
#[derive(Clone, Debug, PartialEq)]
pub struct Sq8Vector {
    codes: Vec<i8>,
    scale: f32,
}

impl Sq8Vector {
    pub fn quantize(vector: &[f32]) -> Result<Self, Error> {
        if vector.is_empty() {
            return Err(Error::ZeroDimension);
        }
        if vector.iter().any(|value| !value.is_finite()) {
            return Err(Error::NonFiniteVector);
        }
        let maximum = vector
            .iter()
            .map(|value| value.abs())
            .fold(0.0f32, f32::max);
        if maximum == 0.0 {
            return Err(Error::ZeroNormVector);
        }
        let scale = maximum / 127.0;
        Ok(Self {
            codes: vector
                .iter()
                .map(|value| (value / scale).round().clamp(-127.0, 127.0) as i8)
                .collect(),
            scale,
        })
    }

    pub fn dimension(&self) -> usize {
        self.codes.len()
    }

    pub fn approximate_dot(&self, query: &[f32]) -> Result<f32, Error> {
        if query.len() != self.codes.len() {
            return Err(Error::DimensionMismatch {
                expected: self.codes.len(),
                actual: query.len(),
            });
        }
        if query.iter().any(|value| !value.is_finite()) {
            return Err(Error::NonFiniteVector);
        }
        Ok(self
            .codes
            .iter()
            .zip(query)
            .map(|(&code, &value)| f32::from(code) * self.scale * value)
            .sum())
    }

    pub fn bytes(&self) -> usize {
        self.codes.len() + std::mem::size_of::<f32>()
    }
}

/// One borrowed canonical mutation for atomic storage/WAL integration.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub enum Mutation<'a> {
    Add {
        id: u64,
        user_id: u64,
        timestamp: i64,
        vector: &'a [f32],
    },
    Delete(u64),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    ZeroDimension,
    DimensionMismatch { expected: usize, actual: usize },
    NonFiniteVector,
    ZeroNormVector,
    NonUnitStoredVector,
    DuplicateId(u64),
    UnknownId(u64),
    AlreadyDeleted(u64),
    InvalidK(usize),
    CapacityExceeded,
    GenerationExhausted,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDimension => write!(f, "vector dimension must be non-zero"),
            Self::DimensionMismatch { expected, actual } => {
                write!(
                    f,
                    "vector dimension mismatch: expected {expected}, got {actual}"
                )
            }
            Self::NonFiniteVector => write!(f, "vectors must contain only finite values"),
            Self::ZeroNormVector => write!(f, "vectors must have a non-zero finite norm"),
            Self::NonUnitStoredVector => write!(f, "stored vectors must be unit-normalized"),
            Self::DuplicateId(id) => write!(f, "record ID {id} already exists"),
            Self::UnknownId(id) => write!(f, "record ID {id} does not exist"),
            Self::AlreadyDeleted(id) => write!(f, "record ID {id} is already deleted"),
            Self::InvalidK(k) => write!(f, "k must be in 1..=64, got {k}"),
            Self::CapacityExceeded => write!(f, "record count exceeds u32 row-slot capacity"),
            Self::GenerationExhausted => write!(f, "store generation is exhausted"),
        }
    }
}

impl std::error::Error for Error {}

/// Canonical record storage with metadata indexes and exact cosine search.
#[derive(Clone, Debug)]
pub struct CoreStore {
    dimension: usize,
    records: Vec<Record>,
    ids: HashMap<u64, u32>,
    users: BTreeMap<u64, BTreeSet<u32>>,
    timestamps: BTreeMap<i64, BTreeSet<u32>>,
    live_len: usize,
    generation: u64,
}

impl CoreStore {
    pub fn new(dimension: usize) -> Result<Self, Error> {
        if dimension == 0 {
            return Err(Error::ZeroDimension);
        }
        Ok(Self {
            dimension,
            records: Vec::new(),
            ids: HashMap::new(),
            users: BTreeMap::new(),
            timestamps: BTreeMap::new(),
            live_len: 0,
            generation: 0,
        })
    }

    /// Restore canonical rows from a checked on-disk snapshot.
    ///
    /// This constructor preserves normalized vector bytes and tombstone slots. It
    /// is intended for storage implementations; ordinary callers should use
    /// [`CoreStore::new`] and [`CoreStore::add`].
    pub fn restore(
        dimension: usize,
        generation: u64,
        records: Vec<RestoredRecord>,
    ) -> Result<Self, Error> {
        Self::restore_iter(dimension, generation, records.into_iter().map(Ok))
    }

    /// Restore one decoded row at a time without staging a second vector store.
    /// Decoder errors propagate unchanged; no partially restored store is returned.
    pub fn restore_iter<E: From<Error>>(
        dimension: usize,
        generation: u64,
        records: impl IntoIterator<Item = Result<RestoredRecord, E>>,
    ) -> Result<Self, E> {
        let mut store = Self::new(dimension)?;
        for restored in records {
            let restored = restored?;
            if store.ids.contains_key(&restored.id) {
                return Err(Error::DuplicateId(restored.id).into());
            }
            validate_stored_vector(&restored.vector, dimension)?;
            let slot = u32::try_from(store.records.len()).map_err(|_| Error::CapacityExceeded)?;
            store.ids.insert(restored.id, slot);
            if restored.live {
                store
                    .users
                    .entry(restored.user_id)
                    .or_default()
                    .insert(slot);
                store
                    .timestamps
                    .entry(restored.timestamp)
                    .or_default()
                    .insert(slot);
                store.live_len += 1;
            }
            store.records.push(Record {
                id: restored.id,
                user_id: restored.user_id,
                timestamp: restored.timestamp,
                vector: restored.vector,
                live: restored.live,
            });
        }
        store.generation = generation;
        Ok(store)
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Total allocated row slots, including tombstones.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn live_len(&self) -> usize {
        self.live_len
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn record(&self, slot: u32) -> Option<&Record> {
        self.records.get(slot as usize)
    }

    pub fn records(&self) -> impl ExactSizeIterator<Item = (u32, &Record)> {
        self.records
            .iter()
            .enumerate()
            .map(|(slot, record)| (slot as u32, record))
    }

    pub fn add(
        &mut self,
        id: u64,
        user_id: u64,
        timestamp: i64,
        vector: impl AsRef<[f32]>,
    ) -> Result<u32, Error> {
        if self.ids.contains_key(&id) {
            return Err(Error::DuplicateId(id));
        }
        let vector = normalize_vector(vector.as_ref(), self.dimension)?;
        let slot = u32::try_from(self.records.len()).map_err(|_| Error::CapacityExceeded)?;
        let generation = self.next_generation()?;

        self.records.push(Record {
            id,
            user_id,
            timestamp,
            vector,
            live: true,
        });
        self.ids.insert(id, slot);
        self.users.entry(user_id).or_default().insert(slot);
        self.timestamps.entry(timestamp).or_default().insert(slot);
        self.live_len += 1;
        self.generation = generation;
        Ok(slot)
    }

    pub fn delete(&mut self, id: u64) -> Result<(), Error> {
        let slot = *self.ids.get(&id).ok_or(Error::UnknownId(id))?;
        let record = &self.records[slot as usize];
        if !record.live {
            return Err(Error::AlreadyDeleted(id));
        }
        let generation = self.next_generation()?;
        let (user_id, timestamp) = (record.user_id, record.timestamp);

        self.records[slot as usize].live = false;
        remove_slot(&mut self.users, user_id, slot);
        remove_slot(&mut self.timestamps, timestamp, slot);
        self.live_len -= 1;
        self.generation = generation;
        Ok(())
    }

    /// Validate an ordered batch without changing canonical state.
    pub fn validate_batch(&self, mutations: &[Mutation<'_>]) -> Result<(), Error> {
        self.prepare_batch(mutations).map(drop)
    }

    /// Validate an ordered batch completely before applying any mutation.
    pub fn apply_batch(&mut self, mutations: &[Mutation<'_>]) -> Result<(), Error> {
        let normalized = self.prepare_batch(mutations)?;
        for mutation in normalized {
            match mutation {
                OwnedMutation::Add {
                    id,
                    user_id,
                    timestamp,
                    vector,
                } => {
                    self.add(id, user_id, timestamp, vector)?;
                }
                OwnedMutation::Delete(id) => self.delete(id)?,
            }
        }
        Ok(())
    }

    fn prepare_batch(&self, mutations: &[Mutation<'_>]) -> Result<Vec<OwnedMutation>, Error> {
        if mutations.is_empty() {
            return Ok(Vec::new());
        }
        let mutation_count =
            u64::try_from(mutations.len()).map_err(|_| Error::GenerationExhausted)?;
        self.generation
            .checked_add(mutation_count)
            .ok_or(Error::GenerationExhausted)?;

        let mut states = HashMap::<u64, bool>::with_capacity(mutations.len());
        let mut normalized = Vec::with_capacity(mutations.len());
        let mut next_len = self.records.len();
        for mutation in mutations {
            match mutation {
                Mutation::Add {
                    id,
                    user_id,
                    timestamp,
                    vector,
                } => {
                    if self.ids.contains_key(id) || states.contains_key(id) {
                        return Err(Error::DuplicateId(*id));
                    }
                    u32::try_from(next_len).map_err(|_| Error::CapacityExceeded)?;
                    next_len = next_len.checked_add(1).ok_or(Error::CapacityExceeded)?;
                    normalized.push(OwnedMutation::Add {
                        id: *id,
                        user_id: *user_id,
                        timestamp: *timestamp,
                        vector: normalize_vector(vector, self.dimension)?,
                    });
                    states.insert(*id, true);
                }
                Mutation::Delete(id) => {
                    let live = states.get(id).copied().or_else(|| {
                        self.ids
                            .get(id)
                            .map(|slot| self.records[*slot as usize].live)
                    });
                    match live {
                        None => return Err(Error::UnknownId(*id)),
                        Some(false) => return Err(Error::AlreadyDeleted(*id)),
                        Some(true) => {}
                    }
                    normalized.push(OwnedMutation::Delete(*id));
                    states.insert(*id, false);
                }
            }
        }

        Ok(normalized)
    }

    /// Return matching live row slots in ascending slot order.
    pub fn filter(&self, predicate: &Predicate) -> Vec<u32> {
        if predicate.timestamp.is_empty() {
            return Vec::new();
        }

        match predicate.user_id {
            Some(user_id) => self
                .users
                .get(&user_id)
                .into_iter()
                .flatten()
                .copied()
                .filter(|&slot| {
                    predicate
                        .timestamp
                        .contains(self.records[slot as usize].timestamp)
                })
                .collect(),
            None if predicate.timestamp == TimestampRange::ALL => self
                .records
                .iter()
                .enumerate()
                .filter(|(_, record)| record.live)
                .map(|(slot, _)| slot as u32)
                .collect(),
            None => {
                let lower = predicate.timestamp.lower.unwrap_or(i64::MIN);
                let mut slots: Vec<_> = match predicate.timestamp.upper {
                    Some(upper) => self
                        .timestamps
                        .range(lower..upper)
                        .flat_map(|(_, slots)| slots.iter().copied())
                        .collect(),
                    None => self
                        .timestamps
                        .range(lower..)
                        .flat_map(|(_, slots)| slots.iter().copied())
                        .collect(),
                };
                slots.sort_unstable();
                slots
            }
        }
    }

    pub fn search(
        &self,
        query: impl AsRef<[f32]>,
        predicate: &Predicate,
        k: usize,
    ) -> Result<SearchOutput, Error> {
        if !(1..=64).contains(&k) {
            return Err(Error::InvalidK(k));
        }
        let query = normalize_vector(query.as_ref(), self.dimension)?;
        let slots = self.filter(predicate);
        let mut best = BinaryHeap::with_capacity(k);
        let dot = dot_implementation();
        for &slot in &slots {
            let record = &self.records[slot as usize];
            let hit = RankedHit(SearchHit {
                id: record.id,
                distance: (1.0 - dot(&query, &record.vector)) as f32,
            });
            if best.len() < k {
                best.push(hit);
            } else if best.peek().is_some_and(|worst| hit < *worst) {
                *best.peek_mut().expect("heap contains k entries") = hit;
            }
        }
        let hits = best
            .into_sorted_vec()
            .into_iter()
            .map(|hit| hit.0)
            .collect();
        Ok(SearchOutput {
            hits,
            evaluated_rows: slots.len(),
        })
    }

    fn next_generation(&self) -> Result<u64, Error> {
        self.generation
            .checked_add(1)
            .ok_or(Error::GenerationExhausted)
    }
}

enum OwnedMutation {
    Add {
        id: u64,
        user_id: u64,
        timestamp: i64,
        vector: Vec<f32>,
    },
    Delete(u64),
}

// Only k distances are retained; the largest (distance, ID) is replaced first.
struct RankedHit(SearchHit);

impl PartialEq for RankedHit {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for RankedHit {}

impl PartialOrd for RankedHit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RankedHit {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .distance
            .total_cmp(&other.0.distance)
            .then_with(|| self.0.id.cmp(&other.0.id))
    }
}

/// Runtime-selected exact CPU distance implementation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CpuDistancePath {
    /// Portable float64 accumulation.
    Scalar,
    /// x86/x86-64 AVX2 with float64 accumulation and a scalar remainder.
    Avx2,
    /// x86/x86-64 AVX2 and FMA with two float64 accumulators.
    Avx2Fma,
    /// AArch64 NEON with float64 accumulation and a scalar remainder.
    Neon,
}

/// Report the distance implementation selected on this CPU.
pub fn cpu_distance_path() -> CpuDistancePath {
    #[cfg(target_arch = "aarch64")]
    {
        CpuDistancePath::Neon
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        if std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma") {
            return CpuDistancePath::Avx2Fma;
        }
        if std::is_x86_feature_detected!("avx2") {
            return CpuDistancePath::Avx2;
        }
        CpuDistancePath::Scalar
    }
}

fn dot_implementation() -> fn(&[f32], &[f32]) -> f64 {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if cpu_distance_path() == CpuDistancePath::Avx2Fma {
        // SAFETY: selected only after runtime AVX2 and FMA detection.
        return |left, right| unsafe { dot_avx2_fma(left, right) };
    }
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if cpu_distance_path() == CpuDistancePath::Avx2 {
        // SAFETY: selected only after runtime AVX2 detection. Both input lengths
        // are validated against the store dimension before this function is used.
        return |left, right| unsafe { dot_avx2(left, right) };
    }
    #[cfg(target_arch = "aarch64")]
    if cpu_distance_path() == CpuDistancePath::Neon {
        // SAFETY: NEON is part of the AArch64 baseline. Input lengths are
        // validated against the store dimension before this function is used.
        return |left, right| unsafe { dot_neon(left, right) };
    }
    dot_scalar
}

fn dot_scalar(left: &[f32], right: &[f32]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(&a, &b)| f64::from(a) * f64::from(b))
        .sum()
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn dot_avx2(left: &[f32], right: &[f32]) -> f64 {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;
    debug_assert_eq!(left.len(), right.len());
    let mut low_sum = _mm256_setzero_pd();
    let mut high_sum = _mm256_setzero_pd();
    let end = left.len() / 8 * 8;
    for offset in (0..end).step_by(8) {
        // SAFETY: each unaligned load stays within eight valid elements.
        let (a, b) = unsafe {
            (
                _mm256_loadu_ps(left.as_ptr().add(offset)),
                _mm256_loadu_ps(right.as_ptr().add(offset)),
            )
        };
        low_sum = _mm256_add_pd(
            low_sum,
            _mm256_mul_pd(
                _mm256_cvtps_pd(_mm256_castps256_ps128(a)),
                _mm256_cvtps_pd(_mm256_castps256_ps128(b)),
            ),
        );
        high_sum = _mm256_add_pd(
            high_sum,
            _mm256_mul_pd(
                _mm256_cvtps_pd(_mm256_extractf128_ps::<1>(a)),
                _mm256_cvtps_pd(_mm256_extractf128_ps::<1>(b)),
            ),
        );
    }
    let mut lanes = [0.0; 4];
    // SAFETY: lanes has room for all four doubles and unaligned stores are allowed.
    unsafe { _mm256_storeu_pd(lanes.as_mut_ptr(), _mm256_add_pd(low_sum, high_sum)) };
    lanes.iter().sum::<f64>() + dot_scalar(&left[end..], &right[end..])
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2,fma")]
unsafe fn dot_avx2_fma(left: &[f32], right: &[f32]) -> f64 {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;
    debug_assert_eq!(left.len(), right.len());
    let mut low_sum = _mm256_setzero_pd();
    let mut high_sum = _mm256_setzero_pd();
    let end = left.len() / 8 * 8;
    for offset in (0..end).step_by(8) {
        // SAFETY: each unaligned load stays within eight valid elements.
        let (a, b) = unsafe {
            (
                _mm256_loadu_ps(left.as_ptr().add(offset)),
                _mm256_loadu_ps(right.as_ptr().add(offset)),
            )
        };
        low_sum = _mm256_fmadd_pd(
            _mm256_cvtps_pd(_mm256_castps256_ps128(a)),
            _mm256_cvtps_pd(_mm256_castps256_ps128(b)),
            low_sum,
        );
        high_sum = _mm256_fmadd_pd(
            _mm256_cvtps_pd(_mm256_extractf128_ps::<1>(a)),
            _mm256_cvtps_pd(_mm256_extractf128_ps::<1>(b)),
            high_sum,
        );
    }
    let mut lanes = [0.0; 4];
    // SAFETY: lanes has room for all four doubles and unaligned stores are allowed.
    unsafe { _mm256_storeu_pd(lanes.as_mut_ptr(), _mm256_add_pd(low_sum, high_sum)) };
    lanes.iter().sum::<f64>() + dot_scalar(&left[end..], &right[end..])
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn dot_neon(left: &[f32], right: &[f32]) -> f64 {
    use std::arch::aarch64::*;
    // SAFETY: the function requires NEON and every load stays within the input.
    unsafe {
        debug_assert_eq!(left.len(), right.len());
        let mut low = vdupq_n_f64(0.0);
        let mut high = vdupq_n_f64(0.0);
        let end = left.len() / 4 * 4;
        for offset in (0..end).step_by(4) {
            let a = vld1q_f32(left.as_ptr().add(offset));
            let b = vld1q_f32(right.as_ptr().add(offset));
            low = vaddq_f64(
                low,
                vmulq_f64(vcvt_f64_f32(vget_low_f32(a)), vcvt_f64_f32(vget_low_f32(b))),
            );
            high = vaddq_f64(high, vmulq_f64(vcvt_high_f64_f32(a), vcvt_high_f64_f32(b)));
        }
        vaddvq_f64(vaddq_f64(low, high)) + dot_scalar(&left[end..], &right[end..])
    }
}

/// One canonical row decoded from durable storage.
#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct RestoredRecord {
    pub id: u64,
    pub user_id: u64,
    pub timestamp: i64,
    pub vector: Vec<f32>,
    pub live: bool,
}

/// Validate and unit-normalize a vector using a float64 norm accumulator.
pub fn normalize_vector(vector: &[f32], dimension: usize) -> Result<Vec<f32>, Error> {
    if vector.len() != dimension {
        return Err(Error::DimensionMismatch {
            expected: dimension,
            actual: vector.len(),
        });
    }
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(Error::NonFiniteVector);
    }
    let norm = vector
        .iter()
        .map(|value| f64::from(*value).powi(2))
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() || norm == 0.0 {
        return Err(Error::ZeroNormVector);
    }
    Ok(vector
        .iter()
        .map(|value| (*value as f64 / norm) as f32)
        .collect())
}

fn validate_stored_vector(vector: &[f32], dimension: usize) -> Result<(), Error> {
    if vector.len() != dimension {
        return Err(Error::DimensionMismatch {
            expected: dimension,
            actual: vector.len(),
        });
    }
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(Error::NonFiniteVector);
    }
    if vector.iter().all(|value| *value == 0.0) {
        return Err(Error::ZeroNormVector);
    }
    // Preserve the stored f32 bytes, but do not let restoration bypass the
    // invariant used by cosine scoring (1 - dot). Allow normalization rounding.
    let norm_squared: f64 = vector.iter().map(|&value| f64::from(value).powi(2)).sum();
    if (norm_squared - 1.0).abs() > 1e-5 {
        return Err(Error::NonUnitStoredVector);
    }
    Ok(())
}

fn remove_slot<K: Ord + Copy>(index: &mut BTreeMap<K, BTreeSet<u32>>, key: K, slot: u32) {
    let remove_key = index.get_mut(&key).is_some_and(|slots| {
        slots.remove(&slot);
        slots.is_empty()
    });
    if remove_key {
        index.remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> CoreStore {
        let mut store = CoreStore::new(2).unwrap();
        store.add(30, 7, i64::MIN, [1.0, 0.0]).unwrap();
        store.add(10, 7, 0, [0.0, 2.0]).unwrap();
        store.add(20, 8, i64::MAX, [1.0, 1.0]).unwrap();
        store
    }

    #[test]
    fn validates_and_normalizes_vectors() {
        let mut store = CoreStore::new(2).unwrap();
        assert_eq!(
            store.add(1, 1, 1, [1.0]),
            Err(Error::DimensionMismatch {
                expected: 2,
                actual: 1
            })
        );
        assert_eq!(
            store.add(1, 1, 1, [f32::NAN, 0.0]),
            Err(Error::NonFiniteVector)
        );
        assert_eq!(store.add(1, 1, 1, [0.0, 0.0]), Err(Error::ZeroNormVector));
        let slot = store.add(1, 1, 1, [3.0, 4.0]).unwrap();
        assert_eq!(store.record(slot).unwrap().vector(), &[0.6, 0.8]);
        assert_eq!(store.add(1, 2, 2, [1.0, 0.0]), Err(Error::DuplicateId(1)));
    }

    #[test]
    fn timestamp_range_is_lower_inclusive_upper_exclusive_at_extremes() {
        let store = store();
        let min_only = Predicate::new(None, TimestampRange::new(Some(i64::MIN), Some(0)));
        assert_eq!(store.filter(&min_only), vec![0]);
        let through_zero = Predicate::new(None, TimestampRange::new(Some(0), Some(i64::MAX)));
        assert_eq!(store.filter(&through_zero), vec![1]);
        let max_only = Predicate::new(None, TimestampRange::new(Some(i64::MAX), None));
        assert_eq!(store.filter(&max_only), vec![2]);
        assert!(
            store
                .filter(&Predicate::new(None, TimestampRange::new(Some(1), Some(1))))
                .is_empty()
        );
    }

    #[test]
    fn predicate_clauses_are_joined_by_and() {
        let store = store();
        let predicate = Predicate::new(Some(7), TimestampRange::new(Some(0), None));
        assert_eq!(store.filter(&predicate), vec![1]);
    }

    #[test]
    fn exact_search_breaks_computed_distance_ties_by_id() {
        let mut store = CoreStore::new(2).unwrap();
        store.add(9, 1, 0, [1.0, 0.0]).unwrap();
        store.add(2, 1, 0, [1.0, 0.0]).unwrap();
        let output = store.search([1.0, 0.0], &Predicate::ALL, 2).unwrap();
        assert_eq!(
            output.hits.iter().map(|hit| hit.id).collect::<Vec<_>>(),
            vec![2, 9]
        );
        assert_eq!(output.evaluated_rows, 2);
    }

    #[test]
    fn deletion_is_immediate_and_slots_are_not_reused() {
        let mut store = store();
        let generation = store.generation();
        store.delete(10).unwrap();
        assert_eq!(store.generation(), generation + 1);
        assert_eq!(store.live_len(), 2);
        assert!(store.record(1).is_some_and(|record| !record.is_live()));
        assert!(!store.filter(&Predicate::ALL).contains(&1));
        assert_eq!(store.delete(10), Err(Error::AlreadyDeleted(10)));
        assert_eq!(store.add(10, 7, 1, [1.0, 0.0]), Err(Error::DuplicateId(10)));
        assert_eq!(store.add(40, 7, 1, [1.0, 0.0]).unwrap(), 3);
    }

    #[test]
    fn search_handles_no_matches_fewer_than_k_and_invalid_k() {
        let store = store();
        let none = Predicate::new(Some(999), TimestampRange::ALL);
        assert_eq!(
            store.search([1.0, 0.0], &none, 10).unwrap(),
            SearchOutput::default()
        );
        let one = Predicate::new(Some(8), TimestampRange::ALL);
        assert_eq!(store.search([1.0, 0.0], &one, 10).unwrap().hits.len(), 1);
        assert_eq!(
            store.search([1.0, 0.0], &Predicate::ALL, 0),
            Err(Error::InvalidK(0))
        );
        assert_eq!(
            store.search([1.0, 0.0], &Predicate::ALL, 65),
            Err(Error::InvalidK(65))
        );
    }

    #[test]
    fn bounded_selection_orders_ties_for_every_supported_k() {
        let mut store = CoreStore::new(7).unwrap();
        for id in (0..100).rev() {
            store.add(id, 1, 0, [1.0; 7]).unwrap();
        }
        store.delete(0).unwrap();
        for k in 1..=64 {
            let output = store.search([1.0; 7], &Predicate::ALL, k).unwrap();
            assert_eq!(
                output.hits.iter().map(|hit| hit.id).collect::<Vec<_>>(),
                (1..=k as u64).collect::<Vec<_>>()
            );
            assert_eq!(output.evaluated_rows, 99);
        }
    }

    #[test]
    fn runtime_distance_agrees_with_scalar_including_vector_remainders() {
        for dimension in [1, 2, 3, 4, 7, 16, 31, 384, 768] {
            let left: Vec<_> = (0..dimension)
                .map(|i| ((i * 17 % 31) as f32 - 15.0) / 16.0)
                .collect();
            let right: Vec<_> = (0..dimension)
                .map(|i| ((i * 11 % 37) as f32 - 17.0) / 32.0)
                .collect();
            assert!(
                (dot_implementation()(&left, &right) - dot_scalar(&left, &right)).abs() < 1e-10
            );
        }
    }

    #[test]
    fn malformed_queries_are_rejected_even_when_filter_is_empty() {
        let store = store();
        let none = Predicate::new(Some(999), TimestampRange::ALL);
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                store.search([value, 0.0], &none, 1),
                Err(Error::NonFiniteVector)
            );
        }
        assert_eq!(store.search([0.0; 2], &none, 1), Err(Error::ZeroNormVector));
        assert!(matches!(
            store.search([1.0], &none, 1),
            Err(Error::DimensionMismatch { .. })
        ));
    }

    #[test]
    fn streaming_restore_preserves_generation_and_propagates_decode_errors() {
        let row = RestoredRecord {
            id: 5,
            user_id: 7,
            timestamp: i64::MIN,
            vector: vec![1.0, 0.0],
            live: false,
        };
        let restored = CoreStore::restore_iter(2, 17, [Ok::<_, Error>(row.clone())]).unwrap();
        assert_eq!(restored.generation(), 17);
        assert_eq!(restored.len(), 1);
        assert_eq!(restored.live_len(), 0);
        assert!(matches!(
            CoreStore::restore_iter(2, 17, [Ok(row), Err(Error::NonFiniteVector)]),
            Err(Error::NonFiniteVector)
        ));
    }

    #[test]
    fn restore_rejects_non_unit_vectors_but_preserves_extreme_normalized_inputs() {
        let mut row = RestoredRecord {
            id: u64::MAX,
            user_id: u64::MAX,
            timestamp: i64::MAX,
            vector: vec![3.0, 4.0],
            live: true,
        };
        assert!(matches!(
            CoreStore::restore(2, 1, vec![row.clone()]),
            Err(Error::NonUnitStoredVector)
        ));
        for vector in [[f32::MAX, f32::MAX], [f32::from_bits(1), 0.0], [3.0, 4.0]] {
            row.vector = normalize_vector(&vector, 2).unwrap();
            let restored = CoreStore::restore(2, 1, vec![row.clone()]).unwrap();
            assert_eq!(restored.record(0).unwrap().vector(), row.vector);
            let hit = restored.search(vector, &Predicate::ALL, 1).unwrap().hits[0];
            assert_eq!(hit.id, u64::MAX);
            assert!(hit.distance.abs() < 1e-6);
        }
    }

    #[test]
    fn exhausted_generation_rejects_mutations_without_partial_changes() {
        let mut store = CoreStore::restore(
            2,
            u64::MAX,
            vec![RestoredRecord {
                id: 1,
                user_id: 7,
                timestamp: 0,
                vector: vec![1.0, 0.0],
                live: true,
            }],
        )
        .unwrap();
        assert_eq!(
            store.add(2, 7, 0, [0.0, 1.0]),
            Err(Error::GenerationExhausted)
        );
        assert_eq!(store.delete(1), Err(Error::GenerationExhausted));
        assert_eq!(store.generation(), u64::MAX);
        assert_eq!(store.len(), 1);
        assert_eq!(store.live_len(), 1);
        assert_eq!(store.filter(&Predicate::ALL), [0]);
    }

    #[test]
    fn mutation_batch_is_ordered_and_atomic() {
        let mut store = CoreStore::new(2).unwrap();
        store.add(1, 7, 10, [1.0, 0.0]).unwrap();
        let generation = store.generation();
        let invalid = [
            Mutation::Add {
                id: 2,
                user_id: 7,
                timestamp: 11,
                vector: &[0.0, 1.0],
            },
            Mutation::Delete(999),
        ];
        assert_eq!(store.apply_batch(&invalid), Err(Error::UnknownId(999)));
        assert_eq!(store.generation(), generation);
        assert_eq!(store.len(), 1);

        let valid = [
            Mutation::Add {
                id: 2,
                user_id: 7,
                timestamp: 11,
                vector: &[0.0, 2.0],
            },
            Mutation::Delete(2),
        ];
        store.apply_batch(&valid).unwrap();
        assert_eq!(store.generation(), generation + 2);
        assert_eq!(store.len(), 2);
        assert_eq!(store.live_len(), 1);
        assert!(!store.record(1).unwrap().is_live());
    }

    #[test]
    fn sq8_is_bounded_and_preserves_dot_products_for_reranking() {
        let vector = normalize_vector(&[3.0, -4.0, 1.0], 3).unwrap();
        let quantized = Sq8Vector::quantize(&vector).unwrap();
        assert_eq!(quantized.dimension(), 3);
        assert_eq!(quantized.bytes(), 7);
        let exact = vector.iter().map(|value| value * value).sum::<f32>();
        let approximate = quantized.approximate_dot(&vector).unwrap();
        assert!((exact - approximate).abs() < 0.01);
        assert!(matches!(
            quantized.approximate_dot(&[1.0]),
            Err(Error::DimensionMismatch { .. })
        ));
        assert_eq!(Sq8Vector::quantize(&[0.0, 0.0]), Err(Error::ZeroNormVector));
    }
}
