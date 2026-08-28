//! Small, deterministic correctness/recall checks; not a scale performance claim.
use std::{
    future::Future,
    task::{Context, Poll, Waker},
};

use qenlo::{BackendSelection, Collection, CollectionConfig, Filter, TimestampRange};

fn block_on<F: Future>(future: F) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

struct Row {
    id: u64,
    user: u64,
    timestamp: i64,
    vector: Vec<f32>,
    live: bool,
}

fn random_vector(state: &mut u64) -> Vec<f32> {
    (0..32)
        .map(|_| {
            *state ^= *state << 13;
            *state ^= *state >> 7;
            *state ^= *state << 17;
            ((*state >> 40) as f32 / (1u32 << 24) as f32) * 2.0 - 1.0
        })
        .collect()
}

// Independent of CoreStore normalization, filtering, distance, and selection.
fn oracle(rows: &[Row], query: &[f32], filter: Filter, k: usize) -> Vec<u64> {
    let query_norm = query
        .iter()
        .map(|&x| f64::from(x).powi(2))
        .sum::<f64>()
        .sqrt();
    let mut hits: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.live
                && filter.user_id.is_none_or(|user| user == row.user)
                && filter.timestamp.lower.is_none_or(|lo| row.timestamp >= lo)
                && filter.timestamp.upper.is_none_or(|hi| row.timestamp < hi)
        })
        .map(|row| {
            let norm = row
                .vector
                .iter()
                .map(|&x| f64::from(x).powi(2))
                .sum::<f64>()
                .sqrt();
            let dot = row
                .vector
                .iter()
                .zip(query)
                .map(|(&a, &b)| f64::from(a) * f64::from(b))
                .sum::<f64>();
            (1.0 - dot / (norm * query_norm), row.id)
        })
        .collect();
    hits.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    hits.into_iter().take(k).map(|(_, id)| id).collect()
}

async fn check_quality(backend: BackendSelection, exact: bool) {
    let mut config = CollectionConfig::cpu_exact(32);
    config.backend = backend;
    let collection = Collection::new(config).await.unwrap();
    let mut seed = 0x91b1_68ab_34a7_dce5;
    let mut rows = Vec::new();
    for id in 0..2048 {
        let row = Row {
            id,
            user: if id < 3 { 999 } else { id % 10 },
            timestamp: if id == 0 {
                i64::MIN
            } else if id == 1 {
                i64::MAX
            } else {
                id as i64 - 1024
            },
            vector: random_vector(&mut seed),
            live: true,
        };
        collection
            .add(row.id, row.user, row.timestamp, &row.vector)
            .unwrap();
        rows.push(row);
    }
    for id in [2, 11, 1024] {
        collection.delete(id).unwrap();
        rows[id as usize].live = false;
    }
    assert!(collection.add(2, 1, 0, &[1.0; 32]).is_err());
    assert!(collection.add(2049, 1, 0, &[1.0; 31]).is_err());
    for invalid in [vec![0.0; 32], vec![f32::NAN; 32], vec![f32::INFINITY; 32]] {
        assert!(collection.add(2049, 1, 0, &invalid).is_err());
        assert!(collection.search(&invalid, &Filter::ALL, 10).await.is_err());
    }
    for k in [0, 65, usize::MAX] {
        assert!(
            collection
                .search(&[1.0; 32], &Filter::ALL, k)
                .await
                .is_err()
        );
    }
    let filters = [
        Filter::ALL,
        Filter::new(Some(1), TimestampRange::ALL),
        Filter::new(None, TimestampRange::new(Some(-300), Some(700))),
        Filter::new(Some(5), TimestampRange::new(Some(-300), Some(700))),
        Filter::new(Some(999), TimestampRange::ALL),
        Filter::new(
            Some(999),
            TimestampRange::new(Some(i64::MIN), Some(i64::MAX)),
        ),
        Filter::new(Some(999), TimestampRange::new(Some(i64::MAX), None)),
        Filter::new(Some(99), TimestampRange::ALL),
        Filter::new(None, TimestampRange::new(Some(1), Some(1))),
    ];
    let mut query_seed = 0x1823_a346_55ff_0231;
    let queries: Vec<_> = (0..16).map(|_| random_vector(&mut query_seed)).collect();
    for (filter_number, filter) in filters.into_iter().enumerate() {
        let mut recall_sum = 0.0;
        for query in &queries {
            let expected = oracle(&rows, query, filter, 10);
            let response = collection.search(query, &filter, 10).await.unwrap();
            let actual: Vec<_> = response.results.iter().map(|hit| hit.id).collect();
            let eligible = collection.filter(&filter);
            assert!(actual.iter().all(|id| eligible.contains(id)));
            assert_eq!(actual.len(), expected.len());
            assert!(response.results.windows(2).all(|pair| {
                pair[0]
                    .distance
                    .total_cmp(&pair[1].distance)
                    .then_with(|| pair[0].id.cmp(&pair[1].id))
                    .is_le()
            }));
            let recall = if expected.is_empty() {
                1.0
            } else {
                actual.iter().filter(|id| expected.contains(id)).count() as f64
                    / expected.len() as f64
            };
            recall_sum += recall;
            if exact {
                assert_eq!(actual, expected);
            }
        }
        let mean_recall = recall_sum / queries.len() as f64;
        println!(
            "{backend:?} filter={filter_number} recall@10={mean_recall:.5} queries=16 rows=2048 dimension=32"
        );
        assert!(
            mean_recall >= 0.95,
            "filter {filter_number}: measured recall@10={mean_recall}"
        );
    }
    for k in 1..=64 {
        let response = collection
            .search(&queries[0], &Filter::ALL, k)
            .await
            .unwrap();
        assert_eq!(response.results.len(), k);
        if exact {
            assert_eq!(
                response.results.iter().map(|r| r.id).collect::<Vec<_>>(),
                oracle(&rows, &queries[0], Filter::ALL, k)
            );
        }
    }
}

#[test]
fn exact_cpu_matches_independent_eligible_set_oracle() {
    block_on(check_quality(BackendSelection::CpuExact, true));
}

#[cfg(feature = "usearch")]
#[test]
fn usearch_measures_filtered_recall_against_independent_oracle() {
    block_on(check_quality(BackendSelection::Usearch, false));
}
