//! Measures qenlo-core CPU search time either side of CERTIFIED_FP32_MAX_ROWS (4,096). N=100k, D=384, k=10.
use qenlo_core::{CoreStore, Predicate, TimestampRange};
use std::time::Instant;
fn lcg(s: &mut u64) -> f32 { *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (((*s >> 11) as f64) / ((1u64 << 53) as f64) * 2.0 - 1.0) as f32 }
fn main() {
    let (n, dim) = (100_000usize, 384usize);
    let mut s = 1u64; let mut store = CoreStore::new(dim).unwrap();
    for id in 0..n as u64 { let v: Vec<f32> = (0..dim).map(|_| lcg(&mut s)).collect(); store.add(id, 0, id as i64, &v).unwrap(); }
    let queries: Vec<Vec<f32>> = (0..200).map(|_| (0..dim).map(|_| lcg(&mut s)).collect()).collect();
    for e in [1000usize, 3000, 4096, 4097, 6000, 10000] {
        let p = Predicate::new(None, TimestampRange::new(Some(0), Some(e as i64)));
        for q in &queries[..20] { store.search(q, &p, 10).unwrap(); }
        let mut t: Vec<f64> = queries.iter().map(|q| { let st = Instant::now(); store.search(q, &p, 10).unwrap(); st.elapsed().as_secs_f64() * 1e3 }).collect();
        t.sort_by(f64::total_cmp);
        println!("E={e:6} path={:?} median_ms={:.3} per_row_ns={:.1}", qenlo_core::cpu_distance_path_for_eligible_count(e), t[t.len()/2], t[t.len()/2]*1e6/e as f64);
    }
}
