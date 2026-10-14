//! Heap footprint and exact-search latency on this host.
//! `cargo run -p qenlo-core --release --example footprint`

use qenlo_core::{CoreStore, Predicate, TimestampRange};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::Instant;

struct Counting;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let live = LIVE.fetch_add(layout.size(), Relaxed) + layout.size();
        PEAK.fetch_max(live, Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn vector(seed: u64, dimension: usize) -> Vec<f32> {
    let mut state = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
    (0..dimension)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 40) as f32 / (1u64 << 24) as f32 - 0.5
        })
        .collect()
}

fn median_us(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}

fn main() {
    let dimension = 384;
    for rows in [2_000_u64, 100_000] {
        let before = LIVE.load(Relaxed);
        let started = Instant::now();
        let mut store = CoreStore::new(dimension).unwrap();
        for id in 0..rows {
            store
                .add(id, id % 4, id as i64, vector(id, dimension))
                .unwrap();
        }
        let add_us = started.elapsed().as_secs_f64() * 1e6 / rows as f64;
        let queries: Vec<_> = (0..101).map(|q| vector(1 << 40 | q, dimension)).collect();
        let search = |store: &CoreStore, predicate: &Predicate| {
            median_us(
                queries
                    .iter()
                    .map(|query| {
                        let t = Instant::now();
                        std::hint::black_box(store.search(query, predicate, 10).unwrap());
                        t.elapsed().as_secs_f64() * 1e6
                    })
                    .collect(),
            )
        };
        let all = search(&store, &Predicate::ALL);
        let user = search(&store, &Predicate::new(Some(1), TimestampRange::ALL));
        let recent = Predicate::new(None, TimestampRange::new(Some(rows as i64 - 1_000), None));
        let recent = search(&store, &recent);
        let heap = LIVE.load(Relaxed) - before;
        // qenlo-memory's pattern: one durable add, then a recall that sees it.
        let interleaved = median_us(
            (0..101)
                .map(|i| {
                    let t = Instant::now();
                    let id = rows + i;
                    store
                        .add(id, id % 4, id as i64, vector(id, dimension))
                        .unwrap();
                    std::hint::black_box(store.search(&queries[i as usize], &Predicate::ALL, 10))
                        .unwrap();
                    t.elapsed().as_secs_f64() * 1e6
                })
                .collect(),
        );
        println!(
            "rows={rows} dim={dimension} heap={:.1}MiB ({:.0} B/row) add={add_us:.2}us \
             search_all={all:.1}us search_user={user:.1}us search_recent1k={recent:.1}us \
             add+search={interleaved:.1}us",
            heap as f64 / 1048576.0,
            heap as f64 / rows as f64,
        );
        drop(store);
    }
    println!("peak heap={:.1}MiB", PEAK.load(Relaxed) as f64 / 1048576.0);
}
