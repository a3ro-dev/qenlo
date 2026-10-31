//! CPU exact-scan cost either side of the v2 paper's 4,096-row FP32/FP64 switch, per scan mode.
//! Eligible rows are scattered (shuffled timestamps). N=100k, D=384, k=10, 200 queries per cell.
//! Adapted from the red-team audit's `cpu_cliff_bench_scattered.rs`.
//!
//! cargo run --release -p qenlo-core --example cpu_cliff_bench
use qenlo_core::{CoreStore, ExactScan, Predicate, TimestampRange};
use std::time::Instant;

fn lcg(s: &mut u64) -> f32 {
    *s = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (((*s >> 11) as f64) / ((1u64 << 53) as f64) * 2.0 - 1.0) as f32
}

fn main() {
    let (n, dim) = (100_000usize, 384usize);
    let mut s = 1u64;
    let mut perm: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        perm.swap(i, (s % (i as u64 + 1)) as usize);
    }
    let mut store = CoreStore::new(dim).unwrap();
    for id in 0..n as u64 {
        let v: Vec<f32> = (0..dim).map(|_| lcg(&mut s)).collect();
        store.add(id, 0, perm[id as usize] as i64, &v).unwrap();
    }
    let queries: Vec<Vec<f32>> = (0..200)
        .map(|_| (0..dim).map(|_| lcg(&mut s)).collect())
        .collect();
    let modes = [
        ExactScan::Legacy,
        ExactScan::Certified,
        ExactScan::Parallel,
        ExactScan::Auto,
    ];
    println!("E,mode,median_ms,p95_ms,per_row_ns");
    for e in [
        1000usize, 2000, 3000, 4096, 4097, 6000, 10000, 20000, 30000, 100000,
    ] {
        let p = Predicate::new(None, TimestampRange::new(Some(0), Some(e as i64)));
        for scan in modes {
            for q in &queries[..20] {
                store.search_with_scan(q, &p, 10, scan).unwrap();
            }
            let mut t: Vec<f64> = queries
                .iter()
                .map(|q| {
                    let st = Instant::now();
                    store.search_with_scan(q, &p, 10, scan).unwrap();
                    st.elapsed().as_secs_f64() * 1e3
                })
                .collect();
            t.sort_by(f64::total_cmp);
            let (median, p95) = (t[t.len() / 2], t[t.len() * 95 / 100]);
            println!(
                "{e},{scan:?},{median:.3},{p95:.3},{:.1}",
                median * 1e6 / e as f64
            );
        }
    }
}
