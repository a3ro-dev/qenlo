//! Feasibility probe (research/experiments/sq8-certified): can an int8 first pass with a
//! deterministic error bound replace the FP32 first pass of the certified exact scan?
//! Also measures heap slack of the `add` growth path against exact-reserve `restore_iter`.
//!
//! cargo run --release -p qenlo-core --example sq8_probe -- [path to ag-news-100k-384.f32]
//! Without a path it uses uniform random vectors.
use qenlo_core::{CoreStore, ExactScan, Predicate, RestoredRecord, TimestampRange};
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

const N: usize = 100_000;
const D: usize = 384;
const K: usize = 10;

fn lcg(s: &mut u64) -> u64 {
    *s = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *s >> 11
}

/// Int8 codes with one scale per vector: x = scale * code + e, |e_i| <= scale / 2.
struct Codes {
    codes: Vec<i8>,
    scale: f64,
    l1: f64,
}

fn quantize(x: &[f32]) -> Codes {
    let max = x.iter().map(|v| v.abs()).fold(0.0f32, f32::max) as f64;
    let scale = max / 127.0;
    Codes {
        codes: x.iter().map(|&v| (v as f64 / scale).round() as i8).collect(),
        scale,
        l1: x.iter().map(|&v| (v as f64).abs()).sum(),
    }
}

#[target_feature(enable = "avx2")]
unsafe fn dot_i8_avx2(a: &[i8], b: &[i8]) -> i32 {
    use std::arch::x86_64::*;
    let mut acc = _mm256_setzero_si256();
    let mut i = 0;
    while i + 16 <= a.len() {
        // SAFETY: i + 16 <= len for both slices (equal length).
        let (va, vb) = unsafe {
            (
                _mm256_cvtepi8_epi16(_mm_loadu_si128(a.as_ptr().add(i).cast())),
                _mm256_cvtepi8_epi16(_mm_loadu_si128(b.as_ptr().add(i).cast())),
            )
        };
        acc = _mm256_add_epi32(acc, _mm256_madd_epi16(va, vb));
        i += 16;
    }
    let mut lanes = [0i32; 8];
    // SAFETY: lanes holds eight i32.
    unsafe { _mm256_storeu_si256(lanes.as_mut_ptr().cast(), acc) };
    lanes.iter().sum::<i32>() + (i..a.len()).map(|j| a[j] as i32 * b[j] as i32).sum::<i32>()
}

fn dot_f64(a: &[f32], b: &[f32]) -> f64 {
    a.iter().zip(b).map(|(&x, &y)| x as f64 * y as f64).sum()
}

/// Keep the k smallest (distance, id), ordered like the core TopK.
fn offer(best: &mut Vec<(f32, u64)>, hit: (f32, u64)) {
    if best.len() == K && hit >= best[K - 1] {
        return;
    }
    let at = best.partition_point(|&h| h < hit);
    best.insert(at, hit);
    best.truncate(K);
}

fn main() {
    assert!(std::is_x86_feature_detected!("avx2"));
    let path = std::env::args().nth(1);
    let raw: Vec<f32> = match &path {
        Some(p) => std::fs::read(p)
            .unwrap()
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
            .collect(),
        None => {
            let mut s = 7;
            (0..(N + 6_000) * D)
                .map(|_| (lcg(&mut s) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0) as f32)
                .collect()
        }
    };
    // ag-news layout: corpus 0..100k, evaluation 101k..106k.
    let corpus = &raw[..N * D];
    let queries: Vec<&[f32]> = raw[101_000 * D..].chunks_exact(D).take(200).collect();
    let mut s = 1u64;
    let mut perm: Vec<usize> = (0..N).collect();
    for i in (1..N).rev() {
        perm.swap(i, (lcg(&mut s) % (i as u64 + 1)) as usize);
    }

    // 1. Heap: growth path vs exact reserve.
    let before = LIVE.load(Relaxed);
    PEAK.store(before, Relaxed);
    let mut store = CoreStore::new(D).unwrap();
    for (id, v) in corpus.chunks_exact(D).enumerate() {
        store.add(id as u64, 0, perm[id] as i64, v).unwrap();
    }
    let (add_heap, add_peak) = (LIVE.load(Relaxed) - before, PEAK.load(Relaxed) - before);
    // Baseline before the input copies: restore frees each input vector as it copies it in.
    let before = LIVE.load(Relaxed);
    let restored: Vec<RestoredRecord> = store
        .records()
        .map(|(_, r)| RestoredRecord {
            id: r.id(),
            user_id: r.user_id(),
            timestamp: r.timestamp(),
            vector: r.vector().to_vec(),
            live: true,
        })
        .collect();
    let exact = CoreStore::restore(D, store.generation(), restored).unwrap();
    let restore_heap = LIVE.load(Relaxed) - before;
    drop(exact);
    println!(
        "heap add={:.1}MiB ({} B/row, peak {:.1}MiB)  exact-reserve={:.1}MiB ({} B/row)  vectors alone={} B/row",
        add_heap as f64 / 1048576.0,
        add_heap / N,
        add_peak as f64 / 1048576.0,
        restore_heap as f64 / 1048576.0,
        restore_heap / N,
        D * 4
    );

    // 2. Int8 sidecar built from the canonical (normalized) rows.
    let codes: Vec<Codes> = (0..N as u32)
        .map(|slot| quantize(store.record(slot).unwrap().vector()))
        .collect();
    println!("sq8 sidecar = {} B/row", D + 16);

    println!("E,certified_med_ms,sq8_med_ms,ratio,mean_rescored,mismatches");
    for e in [1_000usize, 10_000, 30_000, 100_000] {
        let p = Predicate::new(None, TimestampRange::new(Some(0), Some(e as i64)));
        let (mut tc, mut ts, mut rescored, mut mismatches) = (vec![], vec![], 0usize, 0usize);
        for (round, q) in queries.iter().enumerate() {
            let t = Instant::now();
            let want = store.search_with_scan(q, &p, K, ExactScan::Certified).unwrap();
            tc.push(t.elapsed().as_secs_f64() * 1e3);

            let t = Instant::now();
            let q = qenlo_core::normalize_vector(q, D).unwrap();
            let slots = store.filter(&p);
            let qc = quantize(&q);
            let q_codes_l1: f64 = qc.codes.iter().map(|&c| (c as f64).abs()).sum();
            let mut best: Vec<(f32, u64)> = Vec::with_capacity(K + 1);
            for &slot in &slots {
                let c = &codes[slot as usize];
                // SAFETY: AVX2 asserted at startup.
                let int = unsafe { dot_i8_avx2(&qc.codes, &c.codes) } as f64;
                let approx = qc.scale * c.scale * int;
                // |q.x - approx| <= sq*(sx/2)*|q^|_1 + (sq/2)*|x|_1, padded for f64 rounding.
                let bound = (qc.scale * c.scale * 0.5 * q_codes_l1 + qc.scale * 0.5 * c.l1)
                    * 1.0001
                    + 1e-7;
                let lower = (1.0 - approx - bound) as f32;
                if best.len() < K || lower <= best[K - 1].0 {
                    rescored += 1;
                    let r = store.record(slot).unwrap();
                    offer(&mut best, ((1.0 - dot_f64(&q, r.vector())) as f32, r.id()));
                }
            }
            ts.push(t.elapsed().as_secs_f64() * 1e3);
            let got: Vec<u64> = best.iter().map(|h| h.1).collect();
            let want_ids: Vec<u64> = want.hits.iter().map(|h| h.id).collect();
            if got != want_ids && round < 200 {
                mismatches += 1;
            }
        }
        let med = |v: &mut Vec<f64>| {
            v.sort_by(f64::total_cmp);
            v[v.len() / 2]
        };
        let (c, q) = (med(&mut tc), med(&mut ts));
        println!(
            "{e},{c:.3},{q:.3},{:.2},{:.0},{mismatches}",
            q / c,
            rescored as f64 / queries.len() as f64
        );
    }
}
