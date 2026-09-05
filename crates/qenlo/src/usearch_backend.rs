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
    search_expansion: usize,
}

impl UsearchBackend {
    pub(crate) fn parameters(&self) -> (usize, usize, usize) {
        (
            self.index.connectivity(),
            self.index.expansion_add(),
            self.index.expansion_search(),
        )
    }

    pub(crate) fn new(dimension: usize) -> Result<Self, Error> {
        Ok(Self {
            dimension,
            index: make_index(dimension)?,
            search_expansion: 128,
        })
    }

    pub(crate) fn set_search_expansion(&mut self, value: usize) -> Result<(), Error> {
        if value == 0 {
            return Err(Error::Preparation(
                "ANN search expansion must be non-zero".into(),
            ));
        }
        self.index.change_expansion_search(value);
        self.search_expansion = value;
        Ok(())
    }

    pub(crate) fn rebuild(&mut self, store: &CoreStore) -> Result<(), Error> {
        let index = make_index(self.dimension)?;
        index.change_expansion_search(self.search_expansion);
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
        if !(1..=64).contains(&k) {
            return Err(Error::InvalidK(k));
        }
        let query = normalize_vector(query, self.dimension)?;
        let filtering_started = Instant::now();
        let eligible = eligible_ids(store, filter);
        let filtering = filtering_started.elapsed();
        let execution_started = Instant::now();
        let mut hits = if eligible.is_empty() {
            Vec::new()
        } else {
            let found = self
                .index
                .filtered_search(&query, k.min(eligible.len()), |id| eligible.contains(&id))
                .map_err(|error| Error::Search(error.to_string()))?;
            found
                .keys
                .into_iter()
                .zip(found.distances)
                .map(|(id, distance)| SearchHit { id, distance })
                .collect::<Vec<_>>()
        };
        let execution = execution_started.elapsed();
        let selection_started = Instant::now();
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
                scoring: Measurement::unavailable("USearch does not expose scoring separately"),
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
            gpu_row_preparation: None,
            predicate_traversals: 1,
            row_materialization: Measurement::unavailable(
                "USearch evaluates the predicate during graph traversal",
            ),
            materialized_rows: Measurement::unavailable(
                "USearch has no GPU eligible-row materialization",
            ),
            row_cache_hit: None,
            eligibility_predicate_kind: None,
            eligibility_representation: None,
            eligibility_generation: None,
            corpus_rows: Measurement::unavailable("USearch has no compiled eligibility plan"),
            eligible_selectivity: Measurement::unavailable(
                "USearch has no compiled eligibility plan",
            ),
            eligibility_transfer_bytes: Measurement::Available(0),
            eligible_contiguous_runs: Measurement::unavailable(
                "USearch has no compiled eligibility plan",
            ),
            eligibility_cacheable: None,
            eligibility_resident: None,
        })
    }
}

fn make_index(dimension: usize) -> Result<Index, Error> {
    new_index(&IndexOptions {
        dimensions: dimension,
        metric: MetricKind::Cos,
        quantization: ScalarKind::F32,
        connectivity: 16,
        expansion_add: 128,
        expansion_search: 128,
        ..Default::default()
    })
    .map_err(|error| Error::Preparation(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_indexes_and_search_configuration_survive_rebuilds() {
        let mut store = CoreStore::new(2).unwrap();
        let mut index = UsearchBackend::new(2).unwrap();
        assert_eq!(index.parameters(), (16, 128, 128));
        assert!(index.set_search_expansion(0).is_err());
        index.set_search_expansion(256).unwrap();
        index.rebuild(&store).unwrap();
        assert_eq!(index.parameters(), (16, 128, 256));
        assert!(
            index
                .search(&store, &[1.0, 0.0], &Predicate::ALL, 64)
                .unwrap()
                .hits
                .is_empty()
        );
        for id in [9, 2, 7] {
            store.add(id, 1, 0, [1.0, 0.0]).unwrap();
        }
        index.rebuild(&store).unwrap();
        let ids: Vec<_> = index
            .search(&store, &[1.0, 0.0], &Predicate::ALL, 64)
            .unwrap()
            .hits
            .iter()
            .map(|hit| hit.id)
            .collect();
        assert_eq!(ids, [2, 7, 9]);
        assert_eq!(index.parameters(), (16, 128, 256));
        store.delete(2).unwrap();
        // Even a stale graph cannot resurrect canonical tombstones.
        let hits = index
            .search(&store, &[1.0, 0.0], &Predicate::ALL, 64)
            .unwrap()
            .hits;
        assert_eq!(hits.iter().map(|hit| hit.id).collect::<Vec<_>>(), [7, 9]);
    }
}
