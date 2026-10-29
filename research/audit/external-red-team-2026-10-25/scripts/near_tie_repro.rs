//! Near-tie reproducer: qenlo-core CPU exact path (FP32 + FP64 rescoring) vs an emulation of the
//! WGSL `score` kernel's FP32 reduction order (32 lanes, stride-32 accumulation, 16..1 tree) and
//! `select_topk` ordering (f32 distance, then id). Build:
//!   rustc --edition 2024 -O --crate-type rlib --crate-name qenlo_core crates/qenlo-core/src/lib.rs -o libqenlo_core.rlib
//!   rustc --edition 2024 -O near_tie_repro.rs --extern qenlo_core=libqenlo_core.rlib -L . -o near_tie_repro
use qenlo_core::{CoreStore, Predicate};

fn lcg(s: &mut u64) -> f64 { *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); ((*s >> 11) as f64) / ((1u64 << 53) as f64) * 2.0 - 1.0 }
fn normalize(v: &[f32]) -> Vec<f32> { let n = v.iter().map(|&x| (x as f64).powi(2)).sum::<f64>().sqrt(); v.iter().map(|&x| (x as f64 / n) as f32).collect() }
fn gpu_dot(a: &[f32], b: &[f32]) -> f32 {
    let mut p = [0f32; 32];
    for lane in 0..32 { let mut d = lane; let mut acc = 0f32; while d < a.len() { acc += a[d] * b[d]; d += 32; } p[lane] = acc; }
    let mut stride = 16; while stride > 0 { for l in 0..stride { p[l] += p[l + stride]; } stride /= 2; }
    p[0]
}
fn main() {
    let (dim, k, rows_per_trial, trials) = (384usize, 10usize, 200usize, 2000usize);
    let mut seed = 7u64;
    let (mut set_diff, mut order_diff, mut oracle_vs_cpu) = (0, 0, 0);
    let mut example = None;
    for t in 0..trials {
        let q: Vec<f32> = normalize(&(0..dim).map(|_| lcg(&mut seed) as f32).collect::<Vec<_>>());
        // A cluster of rows that are all near-duplicates of one base vector => many near-ties at the cutoff.
        let base: Vec<f32> = (0..dim).map(|i| q[i] * 0.3 + lcg(&mut seed) as f32 * 0.05).collect();
        let mut store = CoreStore::new(dim).unwrap();
        let mut rows = Vec::new();
        for id in 0..rows_per_trial as u64 {
            let eps = 1e-6 * (1 + t % 4) as f64;
            let v: Vec<f32> = base.iter().map(|&x| x + (lcg(&mut seed) * eps) as f32).collect();
            store.add(id, 0, id as i64, &v).unwrap();
            rows.push(normalize(&v)); // identical to qenlo-core's normalize_vector
        }
        let cpu: Vec<u64> = store.search(&q, &Predicate::ALL, k).unwrap().hits.iter().map(|h| h.id).collect();
        let mut gpu: Vec<(f32, u64)> = rows.iter().enumerate().map(|(i, r)| (1.0 - gpu_dot(r, &q), i as u64)).collect();
        gpu.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let gpu: Vec<u64> = gpu[..k].iter().map(|x| x.1).collect();
        let mut f64o: Vec<(f64, u64)> = rows.iter().enumerate().map(|(i, r)| (1.0 - r.iter().zip(&q).map(|(&a, &b)| a as f64 * b as f64).sum::<f64>(), i as u64)).collect();
        f64o.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let oracle: Vec<u64> = f64o[..k].iter().map(|x| x.1).collect();
        let mut cs = cpu.clone(); cs.sort(); let mut gs = gpu.clone(); gs.sort(); let mut os = oracle.clone(); os.sort();
        if cs != gs { set_diff += 1; if example.is_none() { example = Some((t, cpu.clone(), gpu.clone(), f64o[k-1].0, f64o[k].0)); } }
        else if cpu != gpu { order_diff += 1; }
        if cs != os { oracle_vs_cpu += 1; }
    }
    println!("trials={trials} dim={dim} k={k} rows/trial={rows_per_trial}");
    println!("CPU(qenlo-core) vs emulated-GPU FP32: top-k SET differs in {set_diff} trials; same set but different order in {order_diff}");
    println!("CPU(qenlo-core) vs independent FP64 oracle: set differs in {oracle_vs_cpu} trials");
    if let Some((t, c, g, dk, dk1)) = example { println!("first set divergence trial {t}: cpu={c:?}\n  gpu={g:?}\n  fp64 d_k={dk:.12} d_k+1={dk1:.12} gap={:.3e}", dk1 - dk); }
}
