"""Render the audited CSVs as a human-readable report and two figures."""
import csv
import json
import math
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np

OUT = Path(__file__).resolve().parent


def read(name):
    with (OUT / name).open(newline="", encoding="utf-8") as stream:
        return list(csv.DictReader(stream))


def ms(value):
    return f"{float(value)/1e6:.3f}"


def ci_cell(row):
    if row is None:
        return "—"
    if int(row["n_blocks"]) == 1:
        return f"{ms(row['median_ns'])} (n=1; CI unavailable)"
    return f"{ms(row['median_ns'])} [{ms(row['ci95_low_ns'])}, {ms(row['ci95_high_ns'])}]"


def main():
    audit = json.loads((OUT / "audit.json").read_text(encoding="utf-8"))
    blocks = read("block_metrics.csv")
    cover = read("coverage.csv")
    noise = read("e0_noise.csv")
    latency = read("latency_summary.csv")
    pairs = read("paired_comparisons.csv")
    index = {(r["experiment"], int(r["batch"]), int(r["eligible"]), r["engine"], r["metric"]):r for r in latency}
    pindex = {(int(r["batch"]),int(r["eligible"]),r["metric"],r["left"],r["right"]):r for r in pairs}
    text = []
    def add(s=""):
        text.append(s)

    add("# E0/E2 Runpod partial campaign analysis — 2026-09-24")
    add()
    add("## Executive summary")
    add()
    add("- The archive hash matches the supplied SHA-256. The preregistered matrix is **260 summaries**; **227 summaries** exist (E0 20/20; E2 207/240). There is no `COMPLETE` marker. This is a partial campaign, not 230/230 or a completed campaign.")
    add("- All 227 summaries parse and say `status=completed`; all report `recall_target_passed=true` and `filter_violations=0`. However, one completed summary has a recorded process exit code of 101, and one additional CPU directory has no summary. Both are retained and flagged below.")
    add("- E0 A/A variation is substantial. CPU P95 has one extreme block; gpu-rows P50/P95 have two unusually fast blocks. The original ±25% gate is not defensible as a universal noise tolerance. These are observations; GPU clocks, thermal state, and host contention were not experimentally isolated as causes.")
    add("- E2 shows gpu-rows clearly faster than gpu-predicate and gpu-mask at 0.1% and 1% eligibility for both batches and both P50/P95. At batch 1 and 100% eligibility, gpu-predicate is faster than gpu-rows. The exact crossover fraction is not identified. Batch 16 has no 100% cell.")
    add("- Retain the current production router pending a completed confirmatory campaign and E1/E3/E4 evidence. This dataset supports a sparse gpu-rows candidate for targeted validation, not a scalar eligible-count threshold or production fit.")
    add()
    add("## Integrity, provenance, and data quality")
    add()
    add("| Item | Finding | Verification / caveat |")
    add("|---|---|---|")
    add(f"| Archive SHA-256 | `{audit['archive_sha256'].upper()}` | Recomputed from the unchanged archive; matches supplied value. |")
    add(f"| Source revision | `{audit['manifest']['source_revision']}` | Manifest attestation; matches local `git rev-parse HEAD` at analysis time. Working tree has unrelated pre-existing changes. |")
    add(f"| Benchmark binary SHA-256 | `{audit['manifest']['binary_sha256']}` | Manifest attestation; executable is not in archive, so independent recomputation is unavailable. |")
    add(f"| Dataset SHA-256 | `{audit['manifest']['dataset_sha256']}` | Manifest attestation; dataset is not in archive. Configuration records CRC32 `beb3c848` in all completed runs. |")
    add(f"| Runner SHA-256 | `{audit['manifest']['runner_sha256']}` | Recomputed from `research/scripts/run_e0_e2_runpod.py`; matches manifest. |")
    add("| Runtime | Linux x86_64; `qenlo-bench` 0.1.0-alpha.6; WGPU Vulkan; NVIDIA GeForce RTX 4090 | Configurations, manifest, and 227 run records. The benchmark configuration itself says `git_revision=unavailable`; source revision comes from the manifest. Driver/CUDA version and CPU model are not recorded in the archive. |")
    add("| Workload | 100,000 rows × 384 dimensions; independent synthetic metadata; k=10; 200 warmups; 3 repetitions; recall target 0.99; batch 1 or 16 | All observed configurations agree. Timed latency is batch-call completion, with nearest-rank within-run percentiles. |")
    add("| GPU telemetry | SM clock snapshots 210–2775 MHz; 23–35 °C; 12.24–131.26 W | Before/after only, not continuous telemetry. Values do not establish a cause of latency variation. |")
    add("| Summary correctness | 227/227 completed, 227/227 target passed, 227/227 report zero filter violations | Evaluation recall@10 is exactly `1` in 207 summaries and `0.99998` in 20 (all batch 1, E=100,000). Tuning recall@10 is `1` in all 227. Across 2,181,693 sample rows, minimum per-sample recall@10 is 0.9; the aggregate target still passes. |")
    add("| Completion exception | E2 B=1 E=100 block 02 gpu-mask has summary and all 3 run rows, but run record says exit code 101 | Included in primary descriptive results and marked in `block_metrics.csv`. Omitting just this block changes rows/mask P95 ratio from 0.197 to 0.198; the finding is unchanged. The error cause is not evident from its record. |")
    add("| Unfinished destination | E2 B=16 E=30,000 block 01 CPU has configuration, metadata, truth, and tuning files only | No `runs.csv`, `samples.csv`, or `summary.txt`; counted as missing, never as a latency or failed correctness observation. |")
    add("| Runner/stopper logs | Runner log includes 150,941 repeated `can't open file` errors for a later missing script path; stopper log has a shell test syntax error | These are process/control-plane anomalies, not proof that completed benchmark summaries are invalid. The archive has no `COMPLETE` marker. |")
    add()
    add("## Complete coverage table")
    add()
    add("Counts are completed summaries per engine, with five planned blocks for each E2 engine and ten for each E0 engine. `—` means the engine was not planned for E0. Missing block identifiers are listed after the table.")
    add()
    add("| Experiment | B | E | f | CPU | gpu-rows | gpu-predicate | gpu-mask | Total |")
    add("|---|---:|---:|---:|---:|---:|---:|---:|---:|")
    for exp,b,e in sorted({(r["experiment"],int(r["batch"]),int(r["eligible"])) for r in cover}):
        rr={r["engine"]:r for r in cover if r["experiment"]==exp and int(r["batch"])==b and int(r["eligible"])==e}
        vals=[f"{rr[engine]['completed']}/{rr[engine]['expected']}" if engine in rr else "—" for engine in ("cpu","gpu-rows","gpu-predicate","gpu-mask")]
        done=sum(int(r["completed"]) for r in rr.values()); planned=sum(int(r["expected"]) for r in rr.values())
        add(f"| {exp.upper()} | {b} | {e:,} | {e/100000:.3f} | {' | '.join(vals)} | {done}/{planned} |")
    add()
    add("Missing: at B=16/E=30,000, CPU blocks 1–4 and all three GPU engines' blocks 2–4 (13 slots); at B=16/E=100,000, every engine's blocks 0–4 (20 slots). Thus the missing evidence is concentrated in the final **two** E2 cells. No measurements are imputed.")
    add()
    add("## Methods and uncertainty")
    add()
    add("For metric q∈{P50,P95}, `L[b,e,q]` is the **lower-middle** of the three `runs.csv` batch-latency quantiles for block b and engine e. Reported cell latency is the median of available block values `L`. P50 and P95 are calculated separately. Each block is the resampling unit; three repetitions within one block do not count as three independent blocks. The paired comparison uses only blocks containing both engines: `d_b=ln(L[b,left,q]/L[b,right,q])`; its point ratio is `exp(median_b d_b)`. A ratio below 1 favors the left engine. The 95% percentile bootstrap draws `n` block values or paired `d_b` values **with replacement**, computes the median for each of 20,000 draws (seed 20260924 plus deterministic cell offsets), and takes the 2.5th/97.5th percentiles. Ratios exponentiate the log interval endpoints. CIs describe repeat-block uncertainty for this one workload and host; they are not population-wide or causal intervals. n=1 has no meaningful interval; n=2 intervals are especially weak.")
    add()
    add("E0 A/A comparisons use same-engine block latencies. The five disjoint adjacent comparisons are `a_j=ln(L[2j+1]/L[2j])`, j=0,…,4; the all-pairs descriptive set has `ln(L[j]/L[i])` for i<j (45 dependent pairs per engine). The noise statistic is `|a|`, so `exp(|a|)` is the larger/smaller latency ratio. A 25% gate excursion is `exp(|a|)>1.25`. We bootstrap the median of the five disjoint absolute log ratios; the all-pairs 95th percentile is descriptive and is not treated as 45 independent replicates. An E2 advantage is called larger than the conservative E0 band only when the relevant paired 95% CI lies entirely beyond the **larger** of CPU and gpu-rows E0 all-pairs P95 absolute-log bands, metric by metric. This is a screening comparison, since E0 tested only B=1/E=3,000 and its noise may not transfer across E2 cells.")
    add()
    add("## E0 measurement noise")
    add()
    add("All latencies are milliseconds. CI is the block-bootstrap interval for median latency. Within-block spread is max/min among the three runs; all-pairs A/A quantiles use 45 dependent block pairs. Outlier blocks are retained.")
    add()
    add("| Engine | Metric | n blocks | Median latency [95% CI] ms | Median / max within-block spread | Adjacent median abs(log ratio) [95% CI] | All-pairs P95 / max abs(log ratio) | >25%: adjacent / all-pairs |")
    add("|---|---|---:|---:|---:|---:|---:|---:|")
    for r in noise:
        add(f"| {r['engine']} | {r['metric'].replace('_ns','').upper()} | 10 | {ms(r['median_ns'])} [{ms(r['ci95_low_ns'])}, {ms(r['ci95_high_ns'])}] | {float(r['median_within_block_max_min_ratio']):.3f} / {float(r['max_within_block_max_min_ratio']):.3f}× | {float(r['adjacent_abs_log_ratio_median']):.3f} [{float(r['adjacent_abs_log_ratio_median_ci95_low']):.3f}, {float(r['adjacent_abs_log_ratio_median_ci95_high']):.3f}] | {float(r['pairwise_abs_log_ratio_p95']):.3f} / {float(r['pairwise_abs_log_ratio_max']):.3f} | {r['adjacent_outside_25pct_count']}/5; {r['pairwise_outside_25pct_count']}/45 |")
    add()
    add("A robust modified-z rule, `0.6745(L−median(L))/MAD(L)`, threshold `|z|>3.5`, flags CPU block 06 P95 (0.756 ms), gpu-rows blocks 05 and 06 P50 (0.432 and 0.366 ms), and gpu-rows blocks 05 and 06 P95 (0.506 and 0.424 ms). No values are removed. The worst observed within-block P95 spread is 1.550× for CPU and 1.312× for gpu-rows. The conservative all-pairs 95th absolute-log bands are 0.661 (P50; 1.94×) and 0.688 (P95; 1.99×). The 25% tolerance misses this block-scale variation; its origin remains unmeasured. See [E0 block trace](e0-block-trace.png).")
    add()
    add("## E2 representation ablation")
    add()
    add("Entries are median batch latency in ms [95% block-bootstrap CI]; `n` is the count of completed blocks for each engine, shown in the coverage table. B=16/E=30,000 uses only 1 CPU block and 2 GPU blocks; B=16/E=100,000 is missing. [Latency plot](e2-latency.png).")
    for metric in ("p50_ns","p95_ns"):
        add()
        add(f"### {metric.replace('_ns','').upper()}")
        add()
        add("| B | E | f | CPU | gpu-rows | gpu-predicate | gpu-mask |")
        add("|---:|---:|---:|---:|---:|---:|---:|")
        for b in (1,16):
            for e in (100,1000,3000,10000,30000,100000):
                cells=[ci_cell(index.get(("e2",b,e,engine,metric))) for engine in ("cpu","gpu-rows","gpu-predicate","gpu-mask")]
                add(f"| {b} | {e:,} | {e/100000:.3f} | {' | '.join(cells)} |")
    add()
    add("### Paired ratios and crossover")
    add()
    add("Selected paired P95 ratios (left/right) show the transition without interpolating missing observations. Complete P50 and P95 comparisons, including CPU, are in `paired_comparisons.csv`.")
    add()
    add("| B | f | rows / predicate P95 ratio [95% CI] | rows / mask P95 ratio [95% CI] | paired n |")
    add("|---:|---:|---:|---:|---:|")
    for b in (1,16):
        for e in (100,1000,3000,10000,30000,100000):
            a=pindex.get((b,e,"p95_ns","gpu-rows","gpu-predicate"))
            c=pindex.get((b,e,"p95_ns","gpu-rows","gpu-mask"))
            def fmt(r):
                return f"{float(r['median_left_over_right']):.3f} [{float(r['ci95_low_ratio']):.3f}, {float(r['ci95_high_ratio']):.3f}]" if r else "—"
            add(f"| {b} | {e/100000:.3f} | {fmt(a)} | {fmt(c)} | {a['n_paired_blocks'] if a else 0} |")
    add()
    add("For B=1, rows/predicate favors rows at f=0.10 for P50 and P95 (paired CI below 1), is unresolved at f=0.30, and favors predicate at f=1.00 (paired CI above 1). Thus **f*(1) is not point-identified**; a broad observed transition bracket is (0.10, 1.00), with no supported interpolation through 0.30. Rows/mask P50 changes sign between f=0.10 and 0.30, but the high-side effect is modest relative to E0 noise; P95 is unresolved at 0.10 and 0.30, then mask wins at 1.00. For B=16, rows wins through f=0.03 on paired P50/P95 CIs, results near f=0.10 are unresolved, f=0.30 has only two GPU blocks, and f=1.00 is absent. **No supported f*(16) estimate** exists. Predicate versus mask differences are generally small; neither has a dependable sparse-regime advantage here.")
    add()
    add("Against the conservative E0 band, rows' advantage over both predicate and mask at f=0.001 and 0.01 clears the P50 and P95 threshold for both B=1 and B=16 (all relevant paired CIs lie below about 0.52 for P50 and 0.50 for P95). At f=0.03 or above, representation differences generally do not clear that stringent band, including the observed predicate advantage at B=1/f=1.00. This screen is deliberately conservative and should not be mistaken for an equivalence test. CPU is decisively fastest at B=1/f≤0.01; gpu-rows is decisively faster than CPU at B=16/f≥0.01 in observed complete cells. Near B=1/f=0.03, CPU/rows P95 ratio is uncertain (rows/CPU 1.211 [0.834, 2.004]); a scalar threshold cannot be read off that point.")
    add()
    add("## Router implications, limitations, and next action")
    add()
    add("The current automatic GPU representation should **not be changed in production from this archive alone**. The low-fraction gpu-rows advantage is a clear candidate for a controlled, held-out automatic-path test on the same binary and hardware. A single eligible-count threshold is unsupported: batch changes the CPU/GPU relation, representation matters, and the B=1/f=0.03 point sits inside a wide uncertainty interval. Eligibility fraction here is simply E/100,000 on one corpus; fixed E on different corpus sizes was not tested.")
    add()
    add("This partial E0/E2 campaign supports claims about repeatability and representation rankings at the measured cells on one RTX 4090 Vulkan environment. It does not establish generalization across hardware, filter shape, corpus size, concurrency, caching, or deployment load; it does not measure a production automatic-route policy or held-out regret. E1/E3/E4 and a complete, predeclared holdout are needed before fitting or recommending a production router. The missing B=16/f=1.00 cell removes the likely high-fraction endpoint for that batch, while the sparse B=16/f=0.30 cell has too few blocks to establish a transition. The one nonzero process return code and log anomalies warrant a targeted rerun or trace inspection, without retroactively treating all summaries as failures.")
    add()
    add("**Recommended next action:** finish the 33 missing E2 slots, rerun the anomalous B=1/E=100 gpu-mask block in a fresh destination, and repeat E0 noise controls interleaved with high-fraction E2 cells. Preserve the original archive, the partial status, and all raw run records; then predeclare a held-out automatic-path validation before any router change.")
    add()
    add("## Machine-readable outputs")
    add()
    add("`audit.json` contains provenance and exceptions. `coverage.csv`, `block_metrics.csv`, `e0_noise.csv`, `latency_summary.csv`, and `paired_comparisons.csv` contain all derived counts and statistics. Run `python analyze.py --extracted-root <temporary-extraction>/e0-e2`, then `python render_report.py` to reproduce the outputs. The original archive and existing research files were not changed.")
    (OUT / "report.md").write_text("\n".join(text)+"\n", encoding="utf-8")

    # E0 trace: show both quantiles and each individual block, including outliers.
    fig, axs = plt.subplots(1,2,figsize=(11,4),sharex=True)
    colors={"cpu":"#21618c","gpu-rows":"#c0392b"}
    for ax,metric,label in zip(axs,("p50_ns","p95_ns"),("P50","P95")):
        for engine in ("cpu","gpu-rows"):
            rr=sorted((r for r in blocks if r["experiment"]=="e0" and r["engine"]==engine),key=lambda r:int(r["block"]))
            ax.plot([int(r["block"]) for r in rr],[float(r[metric])/1e6 for r in rr],marker="o",label=engine,color=colors[engine])
        ax.set(title=f"E0 {label}: block-level latency",xlabel="Block",ylabel="Batch latency (ms)")
        ax.grid(alpha=.25)
    axs[0].legend()
    fig.tight_layout()
    fig.savefig(OUT/"e0-block-trace.png",dpi=170)
    plt.close(fig)

    fig,axs=plt.subplots(2,2,figsize=(12,8),sharex=True)
    colors={"cpu":"#21618c","gpu-rows":"#c0392b","gpu-predicate":"#2e8b57","gpu-mask":"#8e44ad"}
    for row,b in enumerate((1,16)):
        for col,metric in enumerate(("p50_ns","p95_ns")):
            ax=axs[row,col]
            for engine in ("cpu","gpu-rows","gpu-predicate","gpu-mask"):
                rr=[index.get(("e2",b,e,engine,metric)) for e in (100,1000,3000,10000,30000,100000)]
                rr=[r for r in rr if r]
                x=np.array([float(r["fraction"]) for r in rr]);y=np.array([float(r["median_ns"])/1e6 for r in rr])
                lo=y-np.array([float(r["ci95_low_ns"])/1e6 for r in rr]);hi=np.array([float(r["ci95_high_ns"])/1e6 for r in rr])-y
                ax.errorbar(x,y,yerr=[lo,hi],marker="o",capsize=2,label=engine,color=colors[engine])
            ax.set(xscale="log",yscale="log",title=f"E2 B={b} {metric.replace('_ns','').upper()}",xlabel="Eligible fraction",ylabel="Batch latency (ms)")
            ax.grid(alpha=.25,which="both")
            if b==16:ax.axvspan(.9,1.1,color="gray",alpha=.12)
    axs[0,0].legend(fontsize=8)
    fig.tight_layout()
    fig.savefig(OUT/"e2-latency.png",dpi=170)
    plt.close(fig)


if __name__=="__main__":
    main()
