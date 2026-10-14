//! Portable canonical storage and exact filtered vector search for Qenlo.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap};
use std::fmt;
use std::ops::Bound;

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

    pub const fn is_empty(self) -> bool {
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

/// A borrowed view of one canonical row. Vectors live in one contiguous store buffer.
#[derive(Clone, Copy, Debug)]
pub struct RecordRef<'a> {
    id: u64,
    user_id: u64,
    timestamp: i64,
    vector: &'a [f32],
    live: bool,
}

impl RecordRef<'_> {
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
        self.vector
    }

    pub fn is_live(&self) -> bool {
        self.live
    }

    /// Copy this row out of the store.
    pub fn to_record(&self) -> Record {
        Record {
            id: self.id,
            user_id: self.user_id,
            timestamp: self.timestamp,
            vector: self.vector.to_vec(),
            live: self.live,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct RowMeta {
    id: u64,
    user_id: u64,
    timestamp: i64,
    live: bool,
}

/// Eight f32 values on a 32-byte boundary, so AVX2 loads never split a cache line.
#[repr(C, align(32))]
#[derive(Clone, Copy, Debug, Default)]
struct Lane([f32; LANE]);

const LANE: usize = 8;

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
    InvalidEligibleRow(u32),
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
            Self::InvalidEligibleRow(row) => {
                write!(f, "eligible row {row} is stale or violates the predicate")
            }
            Self::CapacityExceeded => write!(f, "record count exceeds u32 row-slot capacity"),
            Self::GenerationExhausted => write!(f, "store generation is exhausted"),
        }
    }
}

impl std::error::Error for Error {}

/// Canonical record storage with metadata indexes and exact cosine search.
///
/// Vectors are canonical in one row-major, 32-byte-aligned buffer whose rows are zero-padded to a
/// multiple of eight floats. Exact search scans that buffer directly, so there is no second copy
/// to build after a write.
#[derive(Clone, Debug)]
pub struct CoreStore {
    dimension: usize,
    lanes_per_row: usize,
    rows: Vec<RowMeta>,
    vectors: Vec<Lane>,
    ids: HashMap<u64, u32>,
    users: BTreeMap<u64, BTreeSet<u32>>,
    // One flat ordered set: timestamps are usually unique, and a set per key costs ~100 B/row.
    timestamps: BTreeSet<(i64, u32)>,
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
            lanes_per_row: dimension.div_ceil(LANE),
            rows: Vec::new(),
            vectors: Vec::new(),
            ids: HashMap::new(),
            users: BTreeMap::new(),
            timestamps: BTreeSet::new(),
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
        let records = records.into_iter();
        // Snapshots know their row count; reserve once instead of growing by doubling.
        let expected = records.size_hint().0;
        store.rows.reserve_exact(expected);
        store
            .vectors
            .reserve_exact(expected.saturating_mul(store.lanes_per_row));
        store.ids.reserve(expected);
        for restored in records {
            let restored = restored?;
            if store.ids.contains_key(&restored.id) {
                return Err(Error::DuplicateId(restored.id).into());
            }
            validate_stored_vector(&restored.vector, dimension)?;
            let slot = u32::try_from(store.rows.len()).map_err(|_| Error::CapacityExceeded)?;
            store.ids.insert(restored.id, slot);
            if restored.live {
                store
                    .users
                    .entry(restored.user_id)
                    .or_default()
                    .insert(slot);
                store.timestamps.insert((restored.timestamp, slot));
                store.live_len += 1;
            }
            store.push_row(
                RowMeta {
                    id: restored.id,
                    user_id: restored.user_id,
                    timestamp: restored.timestamp,
                    live: restored.live,
                },
                &restored.vector,
            );
        }
        store.generation = generation;
        Ok(store)
    }

    fn push_row(&mut self, meta: RowMeta, vector: &[f32]) {
        debug_assert_eq!(vector.len(), self.dimension);
        let start = self.vectors.len();
        if self.vectors.capacity() - start < self.lanes_per_row {
            // Grow by half instead of doubling: the vector buffer dominates resident memory.
            self.vectors
                .reserve_exact(self.lanes_per_row.max(start / 2));
        }
        self.vectors
            .resize(start + self.lanes_per_row, Lane::default());
        for (lane, chunk) in self.vectors[start..].iter_mut().zip(vector.chunks(LANE)) {
            lane.0[..chunk.len()].copy_from_slice(chunk);
        }
        self.rows.push(meta);
    }

    /// The padded row (a multiple of eight floats, zero tail) as stored.
    fn padded_row(&self, slot: u32) -> &[f32] {
        let start = slot as usize * self.lanes_per_row;
        let lanes = &self.vectors[start..start + self.lanes_per_row];
        // SAFETY: `Lane` is `repr(C)` around `[f32; 8]` with size 32 and align 32, so a slice of
        // lanes is a contiguous, initialized run of `8 * lanes.len()` f32 values.
        unsafe { std::slice::from_raw_parts(lanes.as_ptr().cast::<f32>(), lanes.len() * LANE) }
    }

    fn row(&self, slot: u32) -> &[f32] {
        &self.padded_row(slot)[..self.dimension]
    }

    fn view(&self, slot: u32) -> RecordRef<'_> {
        let meta = &self.rows[slot as usize];
        RecordRef {
            id: meta.id,
            user_id: meta.user_id,
            timestamp: meta.timestamp,
            vector: self.row(slot),
            live: meta.live,
        }
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Total allocated row slots, including tombstones.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn live_len(&self) -> usize {
        self.live_len
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Bytes held by canonical vectors, including row padding and spare capacity.
    pub fn vector_bytes(&self) -> usize {
        self.vectors.capacity() * std::mem::size_of::<Lane>()
    }

    pub fn record(&self, slot: u32) -> Option<RecordRef<'_>> {
        ((slot as usize) < self.rows.len()).then(|| self.view(slot))
    }

    pub fn get(&self, id: u64) -> Option<RecordRef<'_>> {
        self.ids.get(&id).map(|&slot| self.view(slot))
    }

    pub fn slot_of(&self, id: u64) -> Option<u32> {
        self.ids.get(&id).copied()
    }

    pub fn records(&self) -> impl ExactSizeIterator<Item = (u32, RecordRef<'_>)> {
        (0..self.rows.len() as u32).map(|slot| (slot, self.view(slot)))
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
        self.add_normalized(id, user_id, timestamp, vector)
    }

    fn add_normalized(
        &mut self,
        id: u64,
        user_id: u64,
        timestamp: i64,
        vector: Vec<f32>,
    ) -> Result<u32, Error> {
        debug_assert!(!self.ids.contains_key(&id));
        debug_assert_eq!(vector.len(), self.dimension);
        let slot = u32::try_from(self.rows.len()).map_err(|_| Error::CapacityExceeded)?;
        let generation = self.next_generation()?;

        self.push_row(
            RowMeta {
                id,
                user_id,
                timestamp,
                live: true,
            },
            &vector,
        );
        self.ids.insert(id, slot);
        self.users.entry(user_id).or_default().insert(slot);
        self.timestamps.insert((timestamp, slot));
        self.live_len += 1;
        self.generation = generation;
        Ok(slot)
    }

    pub fn delete(&mut self, id: u64) -> Result<(), Error> {
        let slot = *self.ids.get(&id).ok_or(Error::UnknownId(id))?;
        let record = self.rows[slot as usize];
        if !record.live {
            return Err(Error::AlreadyDeleted(id));
        }
        let generation = self.next_generation()?;

        self.rows[slot as usize].live = false;
        let remove_user = self.users.get_mut(&record.user_id).is_some_and(|slots| {
            slots.remove(&slot);
            slots.is_empty()
        });
        if remove_user {
            self.users.remove(&record.user_id);
        }
        self.timestamps.remove(&(record.timestamp, slot));
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
                    self.add_normalized(id, user_id, timestamp, vector)?;
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
        let mut next_len = self.rows.len();
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
                    let live = states
                        .get(id)
                        .copied()
                        .or_else(|| self.ids.get(id).map(|slot| self.rows[*slot as usize].live));
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
                        .contains(self.rows[slot as usize].timestamp)
                })
                .collect(),
            None if predicate.timestamp == TimestampRange::ALL => self.scan_live(|_| true),
            None => {
                let range = self
                    .timestamp_range(predicate.timestamp)
                    .map(|&(_, slot)| slot);
                // Index traversal plus sort costs far more per row than a sequential slot-order
                // scan, so a broad range abandons the index after a bounded probe.
                // ponytail: fixed 1/16 probe, measured locally only; tune per host if it matters.
                let probe_limit = self.rows.len() / RANGE_SCAN_PROBE_DIVISOR;
                let mut slots = Vec::new();
                for slot in range {
                    if slots.len() == probe_limit {
                        return self
                            .scan_live(|record| predicate.timestamp.contains(record.timestamp));
                    }
                    slots.push(slot);
                }
                slots.sort_unstable();
                slots
            }
        }
    }

    /// Count matching live rows without materializing or sorting row slots.
    pub fn filter_count(&self, predicate: &Predicate) -> usize {
        if predicate.timestamp.is_empty() {
            return 0;
        }

        match predicate.user_id {
            Some(user_id) => self.users.get(&user_id).map_or(0, |slots| {
                slots
                    .iter()
                    .filter(|&&slot| {
                        predicate
                            .timestamp
                            .contains(self.rows[slot as usize].timestamp)
                    })
                    .count()
            }),
            None if predicate.timestamp == TimestampRange::ALL => self.live_len,
            None => self.timestamp_range(predicate.timestamp).count(),
        }
    }

    fn timestamp_range(
        &self,
        range: TimestampRange,
    ) -> std::collections::btree_set::Range<'_, (i64, u32)> {
        let lower = (range.lower.unwrap_or(i64::MIN), 0);
        // Slots start at zero, so (upper, 0) is the first entry at `upper`.
        let upper = range
            .upper
            .map_or(Bound::Unbounded, |upper| Bound::Excluded((upper, 0)));
        self.timestamps.range((Bound::Included(lower), upper))
    }

    fn scan_live(&self, keep: impl Fn(&RowMeta) -> bool) -> Vec<u32> {
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, record)| record.live && keep(record))
            .map(|(slot, _)| slot as u32)
            .collect()
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
        self.search_normalized_rows_optimized(&query, predicate, &slots, k)
    }

    /// FP64-accumulating reference search used to verify optimized exact routes.
    #[doc(hidden)]
    pub fn search_reference(
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
        self.search_normalized_rows_reference(&query, predicate, &slots, k)
    }

    /// Exact search over a previously compiled canonical eligibility list.
    ///
    /// Every row is revalidated against current canonical state and the predicate, so a stale or
    /// malformed derived plan fails closed rather than changing search semantics.
    pub fn search_rows(
        &self,
        query: impl AsRef<[f32]>,
        predicate: &Predicate,
        slots: &[u32],
        k: usize,
    ) -> Result<SearchOutput, Error> {
        if !(1..=64).contains(&k) {
            return Err(Error::InvalidK(k));
        }
        let query = normalize_vector(query.as_ref(), self.dimension)?;
        self.search_normalized_rows_optimized(&query, predicate, slots, k)
    }

    /// FP64-accumulating reference search over a compiled eligibility list.
    #[doc(hidden)]
    pub fn search_rows_reference(
        &self,
        query: impl AsRef<[f32]>,
        predicate: &Predicate,
        slots: &[u32],
        k: usize,
    ) -> Result<SearchOutput, Error> {
        if !(1..=64).contains(&k) {
            return Err(Error::InvalidK(k));
        }
        let query = normalize_vector(query.as_ref(), self.dimension)?;
        self.search_normalized_rows_reference(&query, predicate, slots, k)
    }

    /// Revalidate one compiled row: strictly ascending, live, and matching the predicate.
    fn eligible_row(
        &self,
        slot: u32,
        previous: &mut Option<u32>,
        predicate: &Predicate,
    ) -> Result<&RowMeta, Error> {
        if previous.is_some_and(|prior| slot <= prior) {
            return Err(Error::InvalidEligibleRow(slot));
        }
        *previous = Some(slot);
        match self.rows.get(slot as usize) {
            Some(row)
                if row.live
                    && predicate.user_id.is_none_or(|user| user == row.user_id)
                    && predicate.timestamp.contains(row.timestamp) =>
            {
                Ok(row)
            }
            _ => Err(Error::InvalidEligibleRow(slot)),
        }
    }

    fn search_normalized_rows_reference(
        &self,
        query: &[f32],
        predicate: &Predicate,
        slots: &[u32],
        k: usize,
    ) -> Result<SearchOutput, Error> {
        let mut best = TopK::new(k);
        let dot = dot_implementation();
        let mut previous = None;
        for &slot in slots {
            let row = self.eligible_row(slot, &mut previous, predicate)?;
            best.offer(row.id, (1.0 - dot(query, self.row(slot))) as f32);
        }
        Ok(SearchOutput {
            hits: best.into_hits(),
            evaluated_rows: slots.len(),
        })
    }

    fn search_normalized_rows_optimized(
        &self,
        query: &[f32],
        predicate: &Predicate,
        slots: &[u32],
        k: usize,
    ) -> Result<SearchOutput, Error> {
        if slots.len() > CERTIFIED_FP32_MAX_ROWS {
            return self.search_normalized_rows_reference(query, predicate, slots, k);
        }
        let Some(approximate_dot) = fp32_dot_implementation() else {
            return self.search_normalized_rows_reference(query, predicate, slots, k);
        };
        let Some(error_bound) = fp32_dot_error_bound(self.dimension) else {
            return self.search_normalized_rows_reference(query, predicate, slots, k);
        };
        // Pad the query like the stored rows so the FP32 pass runs whole lanes with no tail.
        let mut padded_query = vec![0.0; self.lanes_per_row * LANE];
        padded_query[..self.dimension].copy_from_slice(query);
        let mut approximate_best = TopK::new(k);
        let mut candidates = Vec::with_capacity(k * 2);
        let mut previous = None;
        for &slot in slots {
            let row = self.eligible_row(slot, &mut previous, predicate)?;
            let approximate_distance = 1.0 - approximate_dot(&padded_query, self.padded_row(slot));
            approximate_best.offer(row.id, approximate_distance);
            // Every row that could still reach the exact top k after FP32 error is re-ranked.
            if approximate_best
                .cutoff()
                .is_none_or(|cutoff| approximate_distance <= cutoff + 2.0 * error_bound)
            {
                candidates.push(slot);
            }
        }

        let exact_dot = dot_implementation();
        let mut best = TopK::new(k);
        for slot in candidates {
            best.offer(
                self.rows[slot as usize].id,
                (1.0 - exact_dot(query, self.row(slot))) as f32,
            );
        }
        Ok(SearchOutput {
            hits: best.into_hits(),
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

/// Bounded max-heap keeping the k smallest (distance, ID) pairs.
struct TopK {
    k: usize,
    heap: BinaryHeap<RankedHit>,
}

impl TopK {
    fn new(k: usize) -> Self {
        Self {
            k,
            heap: BinaryHeap::with_capacity(k),
        }
    }

    fn offer(&mut self, id: u64, distance: f32) {
        let hit = RankedHit(SearchHit { id, distance });
        if self.heap.len() < self.k {
            self.heap.push(hit);
        } else if let Some(mut worst) = self.heap.peek_mut()
            && hit < *worst
        {
            *worst = hit;
        }
    }

    /// The current k-th distance, once k rows have been offered.
    fn cutoff(&self) -> Option<f32> {
        if self.heap.len() < self.k {
            return None;
        }
        self.heap.peek().map(|worst| worst.0.distance)
    }

    fn into_hits(self) -> Vec<SearchHit> {
        self.heap
            .into_sorted_vec()
            .into_iter()
            .map(|hit| hit.0)
            .collect()
    }
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
    /// AVX2 float32 candidate scoring with a conservative FP64 boundary verification.
    Fp32Avx2Certified,
    /// AVX2/FMA float32 candidate scoring with a conservative FP64 boundary verification.
    Fp32Avx2FmaCertified,
}

// The certified two-pass path wins for selective scans but loses once its extra pass exceeds the
// measured benefit. This conservative boundary is replaced by the adaptive cost model later.
const CERTIFIED_FP32_MAX_ROWS: usize = 4_096;
/// A timestamp range yielding more than `rows / divisor` slots switches to a sequential scan.
const RANGE_SCAN_PROBE_DIVISOR: usize = 16;

/// Report the optimized exact distance implementation selected on this CPU.
pub fn cpu_distance_path() -> CpuDistancePath {
    cpu_distance_path_for_eligible_count(0)
}

/// Report the exact distance implementation selected for a known eligible-row count.
pub fn cpu_distance_path_for_eligible_count(eligible_rows: usize) -> CpuDistancePath {
    if eligible_rows > CERTIFIED_FP32_MAX_ROWS {
        return reference_cpu_distance_path();
    }
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma") {
        return CpuDistancePath::Fp32Avx2FmaCertified;
    }
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if std::is_x86_feature_detected!("avx2") {
        return CpuDistancePath::Fp32Avx2Certified;
    }
    reference_cpu_distance_path()
}

fn reference_cpu_distance_path() -> CpuDistancePath {
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
    if reference_cpu_distance_path() == CpuDistancePath::Avx2Fma {
        // SAFETY: selected only after runtime AVX2 and FMA detection.
        return |left, right| unsafe { dot_avx2_fma(left, right) };
    }
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if reference_cpu_distance_path() == CpuDistancePath::Avx2 {
        // SAFETY: selected only after runtime AVX2 detection. Both input lengths
        // are validated against the store dimension before this function is used.
        return |left, right| unsafe { dot_avx2(left, right) };
    }
    #[cfg(target_arch = "aarch64")]
    if reference_cpu_distance_path() == CpuDistancePath::Neon {
        // SAFETY: NEON is part of the AArch64 baseline. Input lengths are
        // validated against the store dimension before this function is used.
        return |left, right| unsafe { dot_neon(left, right) };
    }
    dot_scalar
}

type Fp32Dot = fn(&[f32], &[f32]) -> f32;

fn fp32_dot_implementation() -> Option<Fp32Dot> {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if cpu_distance_path() == CpuDistancePath::Fp32Avx2FmaCertified {
        // SAFETY: selected only after runtime AVX2 and FMA detection.
        return Some(|left, right| unsafe { dot_f32_avx2_fma(left, right) });
    }
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if cpu_distance_path() == CpuDistancePath::Fp32Avx2Certified {
        // SAFETY: selected only after runtime AVX2 detection.
        return Some(|left, right| unsafe { dot_f32_avx2(left, right) });
    }
    None
}

/// Conservative absolute error bound for a float32 dot of unit-normalized inputs.
///
/// Four times Higham's gamma bound covers lane-wise accumulation, horizontal reduction,
/// normalization rounding, and the final distance subtraction. If the bound would become too
/// loose, the caller uses the FP64 reference route instead.
fn fp32_dot_error_bound(dimension: usize) -> Option<f32> {
    let scaled_epsilon = dimension as f64 * f64::from(f32::EPSILON);
    if scaled_epsilon >= 0.25 {
        return None;
    }
    Some((4.0 * scaled_epsilon / (1.0 - scaled_epsilon) * 1.001) as f32)
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
#[target_feature(enable = "avx2")]
unsafe fn dot_f32_avx2(left: &[f32], right: &[f32]) -> f32 {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;
    debug_assert_eq!(left.len(), right.len());
    // Four independent accumulators hide the add latency; one chain would serialize every lane.
    let step = |acc: __m256, offset: usize| {
        // SAFETY: callers pass offsets with eight valid elements in both inputs.
        let (a, b) = unsafe {
            (
                _mm256_loadu_ps(left.as_ptr().add(offset)),
                _mm256_loadu_ps(right.as_ptr().add(offset)),
            )
        };
        _mm256_add_ps(acc, _mm256_mul_ps(a, b))
    };
    let (mut s0, mut s1, mut s2, mut s3) = (
        _mm256_setzero_ps(),
        _mm256_setzero_ps(),
        _mm256_setzero_ps(),
        _mm256_setzero_ps(),
    );
    let wide = left.len() / 32 * 32;
    for offset in (0..wide).step_by(32) {
        s0 = step(s0, offset);
        s1 = step(s1, offset + 8);
        s2 = step(s2, offset + 16);
        s3 = step(s3, offset + 24);
    }
    let end = left.len() / 8 * 8;
    for offset in (wide..end).step_by(8) {
        s0 = step(s0, offset);
    }
    let sum = _mm256_add_ps(_mm256_add_ps(s0, s1), _mm256_add_ps(s2, s3));
    let mut lanes = [0.0; 8];
    // SAFETY: lanes has room for all eight floats and unaligned stores are allowed.
    unsafe { _mm256_storeu_ps(lanes.as_mut_ptr(), sum) };
    lanes.iter().sum::<f32>()
        + left[end..]
            .iter()
            .zip(&right[end..])
            .map(|(&a, &b)| a * b)
            .sum::<f32>()
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

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2,fma")]
unsafe fn dot_f32_avx2_fma(left: &[f32], right: &[f32]) -> f32 {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;
    debug_assert_eq!(left.len(), right.len());
    // Four independent accumulators hide the FMA latency; one chain would serialize every lane.
    let step = |acc: __m256, offset: usize| {
        // SAFETY: callers pass offsets with eight valid elements in both inputs.
        let (a, b) = unsafe {
            (
                _mm256_loadu_ps(left.as_ptr().add(offset)),
                _mm256_loadu_ps(right.as_ptr().add(offset)),
            )
        };
        _mm256_fmadd_ps(a, b, acc)
    };
    let (mut s0, mut s1, mut s2, mut s3) = (
        _mm256_setzero_ps(),
        _mm256_setzero_ps(),
        _mm256_setzero_ps(),
        _mm256_setzero_ps(),
    );
    let wide = left.len() / 32 * 32;
    for offset in (0..wide).step_by(32) {
        s0 = step(s0, offset);
        s1 = step(s1, offset + 8);
        s2 = step(s2, offset + 16);
        s3 = step(s3, offset + 24);
    }
    let end = left.len() / 8 * 8;
    for offset in (wide..end).step_by(8) {
        s0 = step(s0, offset);
    }
    let sum = _mm256_add_ps(_mm256_add_ps(s0, s1), _mm256_add_ps(s2, s3));
    let mut lanes = [0.0; 8];
    // SAFETY: lanes has room for all eight floats and unaligned stores are allowed.
    unsafe { _mm256_storeu_ps(lanes.as_mut_ptr(), sum) };
    lanes.iter().sum::<f32>()
        + left[end..]
            .iter()
            .zip(&right[end..])
            .map(|(&a, &b)| a.mul_add(b, 0.0))
            .sum::<f32>()
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
    fn timestamp_ranges_match_brute_force_on_index_and_scan_paths() {
        let rows = 1_000_u64;
        let mut store = CoreStore::new(2).unwrap();
        for id in 0..rows {
            // Scrambled, partly duplicated timestamps plus both i64 extremes.
            let timestamp = match id {
                0 => i64::MIN,
                1 => i64::MAX,
                _ => (id * 7_919 % 500) as i64 - 250,
            };
            store.add(id, id % 3, timestamp, [1.0, id as f32]).unwrap();
        }
        for id in (0..rows).step_by(7) {
            store.delete(id).unwrap();
        }
        let brute = |range: TimestampRange| -> Vec<u32> {
            store
                .records()
                .filter(|(_, record)| record.is_live() && range.contains(record.timestamp()))
                .map(|(slot, _)| slot)
                .collect()
        };
        let bounds = [
            None,
            Some(i64::MIN),
            Some(-250),
            Some(-248),
            Some(-10),
            Some(0),
            Some(245),
            Some(i64::MAX),
        ];
        let mut index_path = false;
        let mut scan_path = false;
        for lower in bounds {
            for upper in bounds {
                let range = TimestampRange::new(lower, upper);
                let expected = if range.is_empty() {
                    Vec::new()
                } else {
                    brute(range)
                };
                // Probe limit is rows / 16 = 62 for this store.
                index_path |= !expected.is_empty() && expected.len() <= 62;
                scan_path |= expected.len() > 62;
                assert_eq!(
                    store.filter(&Predicate::new(None, range)),
                    expected,
                    "{range:?}"
                );
                assert_eq!(
                    store.filter_count(&Predicate::new(None, range)),
                    expected.len(),
                    "{range:?}"
                );
            }
        }
        assert!(
            index_path && scan_path,
            "both materialization paths must be exercised"
        );
    }

    #[test]
    fn filter_count_matches_materialization_for_user_and_timestamp_predicates() {
        let mut store = CoreStore::new(2).unwrap();
        for id in 0..257_u64 {
            store
                .add(
                    id,
                    id % 11,
                    (id.wrapping_mul(97) % 401) as i64 - 200,
                    [1.0, 0.5],
                )
                .unwrap();
        }
        for id in (0..257_u64).step_by(5) {
            store.delete(id).unwrap();
        }

        let bounds = [
            None,
            Some(i64::MIN),
            Some(-200),
            Some(-37),
            Some(0),
            Some(199),
            Some(i64::MAX),
        ];
        for user_id in (0..11).map(Some).chain([None]) {
            for lower in bounds {
                for upper in bounds {
                    let predicate = Predicate::new(user_id, TimestampRange::new(lower, upper));
                    assert_eq!(
                        store.filter_count(&predicate),
                        store.filter(&predicate).len()
                    );
                }
            }
        }
    }

    #[test]
    fn predicate_clauses_are_joined_by_and() {
        let store = store();
        let predicate = Predicate::new(Some(7), TimestampRange::new(Some(0), None));
        assert_eq!(store.filter(&predicate), vec![1]);
    }

    #[test]
    fn compiled_rows_match_filter_search_and_fail_closed_when_stale() {
        let mut store = store();
        let predicate = Predicate::new(Some(7), TimestampRange::ALL);
        let rows = store.filter(&predicate);
        assert_eq!(
            store.search([1.0, 0.0], &predicate, 2).unwrap(),
            store.search_rows([1.0, 0.0], &predicate, &rows, 2).unwrap()
        );
        store.delete(30).unwrap();
        assert_eq!(
            store.search_rows([1.0, 0.0], &predicate, &rows, 2),
            Err(Error::InvalidEligibleRow(rows[0]))
        );
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
    fn cpu_distance_route_uses_the_measured_selective_boundary() {
        assert_eq!(
            cpu_distance_path_for_eligible_count(CERTIFIED_FP32_MAX_ROWS),
            cpu_distance_path()
        );
        assert_eq!(
            cpu_distance_path_for_eligible_count(CERTIFIED_FP32_MAX_ROWS + 1),
            reference_cpu_distance_path()
        );
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
    fn optimized_exact_matches_fp64_oracle_across_randomized_filters_and_k() {
        fn random_f32(state: &mut u64) -> f32 {
            *state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let unit = (*state >> 40) as f32 / ((1u32 << 24) - 1) as f32;
            unit * 2.0 - 1.0
        }

        for dimension in [1, 2, 3, 7, 8, 15, 16, 31, 64, 384] {
            let mut state = 0x5eed_u64 ^ dimension as u64;
            let mut store = CoreStore::new(dimension).unwrap();
            for row in 0..257u64 {
                let mut vector = (0..dimension)
                    .map(|_| random_f32(&mut state))
                    .collect::<Vec<_>>();
                if vector.iter().all(|value| *value == 0.0) {
                    vector[0] = 1.0;
                }
                store
                    .add(row * 13 + 7, row % 5, row as i64 - 128, vector)
                    .unwrap();
            }
            for row in (0..257u64).step_by(17) {
                store.delete(row * 13 + 7).unwrap();
            }
            let predicates = [
                Predicate::ALL,
                Predicate::new(Some(3), TimestampRange::ALL),
                Predicate::new(None, TimestampRange::new(Some(-50), Some(70))),
                Predicate::new(Some(1), TimestampRange::new(Some(-75), Some(91))),
            ];
            for query_index in 0..8 {
                let mut query = (0..dimension)
                    .map(|_| random_f32(&mut state))
                    .collect::<Vec<_>>();
                if query.iter().all(|value| *value == 0.0) {
                    query[0] = 1.0;
                }
                for predicate in predicates {
                    for k in [1, 3, 10, 32, 64] {
                        let optimized = store.search(&query, &predicate, k).unwrap();
                        let reference = store.search_reference(&query, &predicate, k).unwrap();
                        assert_eq!(
                            optimized, reference,
                            "dimension={dimension} query={query_index} predicate={predicate:?} k={k}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn contiguous_rows_are_aligned_padded_and_mutation_safe() {
        let mut store = CoreStore::new(3).unwrap();
        store.add(9, 1, 0, [1.0, 2.0, 3.0]).unwrap();
        store.add(10, 1, 1, [3.0, 2.0, 1.0]).unwrap();
        assert_eq!(store.padded_row(1).as_ptr() as usize % 32, 0);
        assert_eq!(store.padded_row(1).len(), 8);
        assert!(store.padded_row(1)[3..].iter().all(|value| *value == 0.0));
        assert_eq!(store.record(1).unwrap().vector().len(), 3);
        assert_eq!(store.get(9).unwrap().to_record().vector(), store.row(0));
        let clone = store.clone();
        store.add(11, 1, 2, [0.0, 0.0, 1.0]).unwrap();
        assert_eq!(clone.len(), 2);
        let optimized = store.search([3.0, 2.0, 1.0], &Predicate::ALL, 3).unwrap();
        let reference = store
            .search_reference([3.0, 2.0, 1.0], &Predicate::ALL, 3)
            .unwrap();
        assert_eq!(optimized, reference);
        assert_eq!(optimized.hits[0].id, 10);
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

#[cfg(test)]
mod filter_timing {
    use super::*;

    /// Local timing probe for timestamp-range materialization (E0/E2 mechanism audit).
    /// `cargo test -p qenlo-core --release filter_timing -- --ignored --nocapture`
    #[test]
    #[ignore = "timing probe, not a correctness test"]
    fn timestamp_range_materialization_timing() {
        let rows = 100_000_u64;
        let mut store = CoreStore::new(384).unwrap();
        let mut vector = vec![0.0_f32; 384];
        // Independent metadata: timestamps are a fixed permutation of slot order.
        for id in 0..rows {
            vector[(id % 384) as usize] = 1.0;
            let timestamp = (id.wrapping_mul(48_271) % rows) as i64 - 50_000;
            store.add(id, id % 97, timestamp, &vector).unwrap();
            vector[(id % 384) as usize] = 0.0;
        }
        for eligible in [
            100_i64, 1_000, 3_000, 6_000, 8_000, 10_000, 12_000, 14_000, 20_000, 30_000, 100_000,
        ] {
            let predicate = Predicate {
                user_id: None,
                timestamp: TimestampRange {
                    lower: None,
                    upper: Some(eligible - 50_000),
                },
            };
            let mut samples = (0..51)
                .map(|_| {
                    let started = std::time::Instant::now();
                    let slots = std::hint::black_box(store.filter(&predicate));
                    assert_eq!(slots.len() as i64, eligible);
                    started.elapsed().as_nanos()
                })
                .collect::<Vec<_>>();
            samples.sort_unstable();
            println!("E={eligible} median_ns={}", samples[25]);
        }
    }
}
