//! Portable canonical storage and exact filtered vector search for Qenlo.

use std::collections::{BTreeMap, BTreeSet, HashMap};
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    ZeroDimension,
    DimensionMismatch { expected: usize, actual: usize },
    NonFiniteVector,
    ZeroNormVector,
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
        let mut store = Self::new(dimension)?;
        for restored in records {
            if store.ids.contains_key(&restored.id) {
                return Err(Error::DuplicateId(restored.id));
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
        let mut hits: Vec<_> = slots
            .iter()
            .map(|&slot| {
                let record = &self.records[slot as usize];
                let dot = query
                    .iter()
                    .zip(&record.vector)
                    .map(|(left, right)| left * right)
                    .sum::<f32>();
                SearchHit {
                    id: record.id,
                    distance: 1.0 - dot,
                }
            })
            .collect();
        hits.sort_unstable_by(|left, right| {
            left.distance
                .total_cmp(&right.distance)
                .then_with(|| left.id.cmp(&right.id))
        });
        hits.truncate(k);
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
}
