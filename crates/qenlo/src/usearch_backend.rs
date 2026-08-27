use qenlo_core::{CoreStore, Predicate, SearchHit, normalize_vector};
use usearch::{Index, IndexOptions, MetricKind, ScalarKind, new_index};
use web_time::Instant;

use crate::{
    Algorithm, BackendKind, BackendOutput, Error, FilterExecution, Measurement, PhaseTimings,
    eligible_ids,
};

pub(crate) struct UsearchBackend {
    dimension: usize,
    index: Index,
}

impl UsearchBackend {
    pub(crate) fn new(dimension: usize) -> Result<Self, Error> {
        Ok(Self {
            dimension,
            index: make_index(dimension)?,
        })
    }

    pub(crate) fn rebuild(&mut self, store: &CoreStore) -> Result<(), Error> {
        let index = make_index(self.dimension)?;
        index
            .reserve(store.live_len())
            .map_err(|error| Error::Preparation(error.to_string()))?;
        for (_, record) in store.records().filter(|(_, record)| record.is_live()) {
            index
                .add(record.id(), record.vector())
                .map_err(|error| Error::Preparation(error.to_string()))?;
        }
        self.index = index;
        Ok(())
    }

    pub(crate) fn search(
        &self,
        store: &CoreStore,
        query: &[f32],
        filter: &Predicate,
        k: usize,
    ) -> Result<BackendOutput, Error> {
        let query = normalize_vector(query, self.dimension)?;
        let filtering_started = Instant::now();
        let eligible = eligible_ids(store, filter);
        let filtering = filtering_started.elapsed();
        let execution_started = Instant::now();
        let found = self
            .index
            .filtered_search(&query, k, |id| eligible.contains(&id))
            .map_err(|error| Error::Search(error.to_string()))?;
        let execution = execution_started.elapsed();
        let selection_started = Instant::now();
        let mut hits = found
            .keys
            .into_iter()
            .zip(found.distances)
            .map(|(id, distance)| SearchHit { id, distance })
            .collect::<Vec<_>>();
        hits.sort_by(|a, b| {
            a.distance
                .total_cmp(&b.distance)
                .then_with(|| a.id.cmp(&b.id))
        });
        hits.truncate(k);
        let selection = selection_started.elapsed();

        Ok(BackendOutput {
            hits,
            actual_backend: BackendKind::Usearch,
            algorithm: Algorithm::Hnsw,
            filter_execution: FilterExecution::GraphPredicate,
            phases: PhaseTimings {
                preparation: Measurement::unavailable("set by collection"),
                filtering: Measurement::Available(filtering),
                upload: Measurement::unavailable("CPU backend"),
                execution: Measurement::Available(execution),
                readback: Measurement::unavailable("CPU backend"),
                selection: Measurement::Available(selection),
            },
            upload_bytes: Measurement::Available(0),
            readback_bytes: Measurement::Available(0),
            dispatch_count: Measurement::Available(0),
            allocation_bytes: Measurement::unavailable(
                "USearch-owned allocations are not exposed by its Rust API",
            ),
            candidates: Measurement::unavailable(
                "USearch does not expose graph traversal counters",
            ),
        })
    }
}

fn make_index(dimension: usize) -> Result<Index, Error> {
    new_index(&IndexOptions {
        dimensions: dimension,
        metric: MetricKind::Cos,
        quantization: ScalarKind::F32,
        connectivity: 0,
        expansion_add: 0,
        expansion_search: 0,
        ..Default::default()
    })
    .map_err(|error| Error::Preparation(error.to_string()))
}
