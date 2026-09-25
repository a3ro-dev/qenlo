"""Reproduce the offline E0/E2 archive audit and block-level statistics."""
from __future__ import annotations

import csv
import argparse
import hashlib
import json
import math
import re
from collections import Counter, defaultdict
from pathlib import Path

import numpy as np

ARCHIVE = Path(r"D:\qenloDB\research\data\raw\runpod-e0-e2-partial-20260924.tar.gz")
ROOT = None
OUT = Path(__file__).resolve().parent
SEED = 20260924
N_BOOT = 20000
ENGINES = ("cpu", "gpu-rows", "gpu-predicate", "gpu-mask")
METRICS = ("p50_ns", "p95_ns")


def props(path):
    result = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            result[key] = value
    return result


def quantile(values, q):
    return float(np.quantile(np.asarray(values, dtype=float), q))


def lower_middle(values):
    a = sorted(values)
    return a[(len(a) - 1) // 2]


def median_ci(values, seed):
    a = np.asarray(values, dtype=float)
    rng = np.random.default_rng(seed)
    indices = rng.integers(0, len(a), (N_BOOT, len(a)))
    boots = np.median(a[indices], axis=1)
    return [quantile(boots, .025), quantile(boots, .975)]


def csv_write(name, rows):
    if not rows:
        return
    keys = list(rows[0])
    with (OUT / name).open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=keys)
        writer.writeheader()
        writer.writerows(rows)


def main():
    global ROOT
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--extracted-root", required=True, type=Path, help="Directory containing manifest.json, e0/, and e2/")
    args = parser.parse_args()
    ROOT = args.extracted_root.resolve()
    if not (ROOT / "manifest.json").is_file():
        parser.error("--extracted-root must contain manifest.json")
    OUT.mkdir(parents=True, exist_ok=True)
    with ARCHIVE.open("rb") as archive_stream:
        digest = hashlib.file_digest(archive_stream, "sha256").hexdigest()
    manifest = json.loads((ROOT / "manifest.json").read_text())
    expected = []
    for block in range(10):
        for engine in ("cpu", "gpu-rows"):
            expected.append(("e0", 1, 3000, block, engine))
    for batch in (1, 16):
        for eligible in (100, 1000, 3000, 10000, 30000, 100000):
            for block in range(5):
                for engine in ("gpu-rows", "gpu-predicate", "gpu-mask", "cpu"):
                    expected.append(("e2", batch, eligible, block, engine))
    expected_set = set(expected)
    rows = []
    issues = []
    config_values = defaultdict(set)
    run_gpu_names = Counter()
    clocks = []
    exact_recalls = Counter()
    sample_count = 0
    sample_recall_min = 1.0
    sample_backend_counts = Counter()
    summary_paths = sorted(ROOT.rglob("summary.txt"))
    for path in summary_paths:
        rel = path.relative_to(ROOT).parts
        try:
            exp, cell, block_name, engine, file = rel
            batch, eligible = map(int, re.fullmatch(r"b(\d+)-e(\d+)", cell).groups())
            block = int(re.fullmatch(r"block-(\d+)", block_name).group(1))
            key = (exp, batch, eligible, block, engine)
            if key not in expected_set:
                issues.append(f"unexpected summary: {path.relative_to(ROOT)}")
            summary = props(path)
            config = props(path.with_name("configuration.txt"))
            for name in ("platform", "package_version", "git_revision", "dataset_crc32", "source_crc32", "dimensions", "rows", "repetitions", "warmup_queries", "recall_target", "distribution", "gpu_row_preparation", "filter_mode", "percentile", "query_latency", "eligible_fraction_actual", "backend"):
                if name in config:
                    config_values[name].add(config[name])
            if summary.get("status") != "completed":
                issues.append(f"non-completed summary: {path.relative_to(ROOT)}")
            for field in ("tuning_recall_at_k", "evaluation_recall_at_k", "tuning_recall_at_10", "evaluation_recall_at_10"):
                exact_recalls[(field, summary.get(field, "MISSING"))] += 1
            if summary.get("recall_target_passed") != "true":
                issues.append(f"recall gate: {path.relative_to(ROOT)} {summary.get('recall_target_passed')}")
            if summary.get("filter_violations") != "0":
                issues.append(f"filter violations: {path.relative_to(ROOT)} {summary.get('filter_violations')}")
            if any(config.get(name) != str(value) for name, value in (("backend", engine), ("eligible_count", eligible), ("batch", batch), ("order_seed", 9001 + block))):
                issues.append(f"config mismatch: {path.relative_to(ROOT)}")
            run_path = path.with_name("runs.csv")
            with run_path.open(newline="", encoding="utf-8") as f:
                runs = list(csv.DictReader(f))
            if len(runs) != 3:
                issues.append(f"repetition count: {path.relative_to(ROOT)} {len(runs)}")
            p50s = [int(r["p50_batch_ns"]) for r in runs]
            p95s = [int(r["p95_batch_ns"]) for r in runs]
            if int(summary["median_run_p95_batch_ns"]) != lower_middle(p95s):
                issues.append(f"summary P95 disagrees with runs.csv: {path.relative_to(ROOT)}")
            if any(r.get("recall_target_passed") != "true" for r in runs):
                issues.append(f"run recall target failure: {path.relative_to(ROOT)}")
            record_path = path.parent.parent / (engine + ".run.json")
            if not record_path.is_file():
                issues.append(f"missing run record: {path.relative_to(ROOT)}")
                record = {}
            else:
                record = json.loads(record_path.read_text())
                if record.get("returncode") != 0:
                    issues.append(f"nonzero return code: {path.relative_to(ROOT)}")
                for label in ("clocks_before", "clocks_after"):
                    value = record.get(label, "")
                    parts = [x.strip() for x in value.split(",")]
                    if len(parts) >= 6:
                        run_gpu_names[parts[1]] += 1
                        clocks.append({"phase": label, "engine": engine, "sm_mhz": float(parts[3].split()[0]), "temperature_c": float(parts[4]), "power_w": float(parts[5].split()[0])})
            samples_path = path.with_name("samples.csv")
            with samples_path.open(newline="", encoding="utf-8") as f:
                for s in csv.DictReader(f):
                    sample_count += 1
                    if s.get("recall_at_10"):
                        sample_recall_min = min(sample_recall_min, float(s["recall_at_10"]))
                    sample_backend_counts[s.get("actual_backend", "MISSING")] += 1
            values = {
                "experiment": exp, "batch": batch, "eligible": eligible,
                "fraction": eligible / 100000, "block": block, "engine": engine,
                "n_repetitions": len(runs), "p50_ns": lower_middle(p50s),
                "p95_ns": lower_middle(p95s),
                "p50_spread_ratio": max(p50s) / min(p50s),
                "p95_spread_ratio": max(p95s) / min(p95s),
                "p50_min_ns": min(p50s), "p50_max_ns": max(p50s),
                "p95_min_ns": min(p95s), "p95_max_ns": max(p95s),
                "evaluation_recall_at_10": summary["evaluation_recall_at_10"],
                "tuning_recall_at_10": summary["tuning_recall_at_10"],
                "filter_violations": int(summary["filter_violations"]),
                "process_returncode": record.get("returncode"),
                "started_unix": record.get("started_unix"),
                "path": str(path.relative_to(ROOT)).replace("\\", "/"),
            }
            rows.append(values)
        except Exception as exc:
            issues.append(f"malformed {path.relative_to(ROOT)}: {type(exc).__name__}: {exc}")
    lookup = {(r["experiment"], r["batch"], r["eligible"], r["block"], r["engine"]): r for r in rows}
    missing = [k for k in expected if k not in lookup]
    incomplete = []
    for path in sorted(ROOT.rglob("configuration.txt")):
        if not path.with_name("summary.txt").exists():
            incomplete.append(str(path.parent.relative_to(ROOT)).replace("\\", "/"))
    csv_write("block_metrics.csv", rows)
    coverage = []
    for exp, batch, eligible in sorted(set(k[:3] for k in expected)):
        blocks = 10 if exp == "e0" else 5
        for engine in ENGINES:
            n_expected = sum(k[:3] == (exp,batch,eligible) and k[4] == engine for k in expected)
            if not n_expected: continue
            present = [k[3] for k in expected if k[:3] == (exp,batch,eligible) and k[4] == engine and k in lookup]
            coverage.append({"experiment":exp,"batch":batch,"eligible":eligible,"fraction":eligible/100000,
                             "engine":engine,"completed":len(present),"expected":n_expected,
                             "present_blocks":",".join(map(str,present)),
                             "missing_blocks":",".join(str(b) for b in range(blocks) if b not in present)})
    csv_write("coverage.csv", coverage)
    group_rows = []
    groups = defaultdict(list)
    for row in rows:
        groups[(row["experiment"],row["batch"],row["eligible"],row["engine"])].append(row)
    for key, group in sorted(groups.items()):
        exp,batch,eligible,engine=key
        for metric in METRICS:
            vals = [r[metric] for r in group]
            ci = median_ci(vals, SEED + batch + eligible + len(engine) + (0 if metric == "p50_ns" else 1))
            group_rows.append({"experiment":exp,"batch":batch,"eligible":eligible,"fraction":eligible/100000,
                "engine":engine,"metric":metric,"n_blocks":len(vals),"n_repetitions_per_block":3,
                "median_ns":float(np.median(vals)),"ci95_low_ns":ci[0],"ci95_high_ns":ci[1],
                "min_ns":min(vals),"max_ns":max(vals),"median_within_block_max_min_ratio":float(np.median([r[metric.replace('_ns','_spread_ratio')] for r in group])),
                "max_within_block_max_min_ratio":max(r[metric.replace('_ns','_spread_ratio')] for r in group)})
    csv_write("latency_summary.csv", group_rows)
    e0_noise=[]
    bands={}
    for engine in ("cpu","gpu-rows"):
        group=sorted(groups[("e0",1,3000,engine)], key=lambda x:x["block"])
        for metric in METRICS:
            vals=[r[metric] for r in group]
            adjacent=[math.log(vals[i+1]/vals[i]) for i in range(0,10,2)]
            all_pairs=[math.log(vals[j]/vals[i]) for i in range(10) for j in range(i+1,10)]
            abs_pairs=[abs(x) for x in all_pairs]
            bands[(engine,metric)]=quantile(abs_pairs,.95)
            ci=median_ci(vals,SEED+len(engine)+(metric=="p95_ns"))
            adjacent_ci=median_ci(np.abs(adjacent),SEED+100+len(engine)+(metric=="p95_ns"))
            e0_noise.append({"engine":engine,"metric":metric,"n_blocks":10,"n_run_repetitions":30,
                "median_ns":float(np.median(vals)),"ci95_low_ns":ci[0],"ci95_high_ns":ci[1],
                "block_min_ns":min(vals),"block_max_ns":max(vals),
                "median_within_block_max_min_ratio":float(np.median([r[metric.replace('_ns','_spread_ratio')] for r in group])),
                "max_within_block_max_min_ratio":max(r[metric.replace('_ns','_spread_ratio')] for r in group),
                "n_disjoint_adjacent_aa":len(adjacent),"adjacent_abs_log_ratio_median":float(np.median(np.abs(adjacent))),
                "adjacent_abs_log_ratio_median_ci95_low":adjacent_ci[0],"adjacent_abs_log_ratio_median_ci95_high":adjacent_ci[1],
                "adjacent_abs_log_ratio_max":max(abs(x) for x in adjacent),
                "n_all_pairwise_aa_dependent":len(all_pairs),"pairwise_abs_log_ratio_p95":quantile(abs_pairs,.95),
                "pairwise_abs_log_ratio_max":max(abs_pairs),
                "pairwise_outside_25pct_count":sum(abs(x)>math.log(1.25) for x in all_pairs),
                "adjacent_outside_25pct_count":sum(abs(x)>math.log(1.25) for x in adjacent)})
    csv_write("e0_noise.csv",e0_noise)
    comparisons=[]
    for batch in (1,16):
        for eligible in (100,1000,3000,10000,30000,100000):
            for left,right in (("gpu-rows","gpu-predicate"),("gpu-rows","gpu-mask"),("gpu-predicate","gpu-mask"),("gpu-rows","cpu"),("gpu-predicate","cpu"),("gpu-mask","cpu")):
                for metric in METRICS:
                    pairs=[(lookup[("e2",batch,eligible,b,left)][metric],lookup[("e2",batch,eligible,b,right)][metric]) for b in range(5) if ("e2",batch,eligible,b,left) in lookup and ("e2",batch,eligible,b,right) in lookup]
                    if not pairs:continue
                    logr=np.log(np.array([a/b for a,b in pairs]))
                    ci=median_ci(logr,SEED+batch+eligible+len(left)+len(right)+(metric=="p95_ns"))
                    band=max(bands[("cpu",metric)],bands[("gpu-rows",metric)])
                    comparisons.append({"batch":batch,"eligible":eligible,"fraction":eligible/100000,"metric":metric,
                        "left":left,"right":right,"n_paired_blocks":len(pairs),
                        "median_log_left_over_right":float(np.median(logr)),"ci95_low_log":ci[0],"ci95_high_log":ci[1],
                        "median_left_over_right":float(np.exp(np.median(logr))),"ci95_low_ratio":float(np.exp(ci[0])),"ci95_high_ratio":float(np.exp(ci[1])),
                        "e0_conservative_log_band":band,"left_advantage_exceeds_e0_band":ci[1] < -band,
                        "right_advantage_exceeds_e0_band":ci[0] > band})
    csv_write("paired_comparisons.csv",comparisons)
    log_lines = (ROOT/"runner.log").read_text(errors="replace").splitlines()
    audit={"archive_sha256":digest,"archive_hash_matches_user":digest.upper()=="E4CA1336757CF55A5F5150A8BCC601AB3538CB7E9E9C2270C137E5A72FD51ED0",
        "manifest":manifest,"expected_summaries":len(expected),"observed_summaries":len(summary_paths),"parsed_summaries":len(rows),
        "missing_expected":len(missing),"missing_by_cell":{f"{e}/b{b}-e{n}":sum(k[:3]==(e,b,n) for k in missing) for e,b,n in sorted(set(k[:3] for k in missing))},
        "incomplete_directories":incomplete,"issues":issues,"has_complete_marker":(ROOT/"COMPLETE").exists(),
        "config_values":{k:sorted(v) for k,v in config_values.items()},"gpu_names_from_run_records":dict(run_gpu_names),
        "gpu_clock_sm_mhz_range":[min(c["sm_mhz"] for c in clocks),max(c["sm_mhz"] for c in clocks)],
        "gpu_temperature_c_range":[min(c["temperature_c"] for c in clocks),max(c["temperature_c"] for c in clocks)],
        "gpu_power_w_range":[min(c["power_w"] for c in clocks),max(c["power_w"] for c in clocks)],
        "summary_recall_value_counts":[{"field":k[0],"value":k[1],"count":v} for k,v in sorted(exact_recalls.items())],
        "sample_rows_checked":sample_count,"minimum_sample_recall_at_10":sample_recall_min,"sample_actual_backend_counts":dict(sample_backend_counts),
        "runner_log_lines":len(log_lines),"runner_log_missing_script_errors":sum("can't open file" in x for x in log_lines),
        "run_start_unix_range":[min(r["started_unix"] for r in rows),max(r["started_unix"] for r in rows)],
        "continuation_log_lines":len((ROOT/"continuation.log").read_text(errors="replace").splitlines()),
        "stopper_log":(ROOT/"stopper.log").read_text(errors="replace").strip(),"bootstrap_draws":N_BOOT,"bootstrap_seed":SEED}
    (OUT/"audit.json").write_text(json.dumps(audit,indent=2)+"\n",encoding="utf-8")
    print(json.dumps({k:audit[k] for k in ("expected_summaries","observed_summaries","parsed_summaries","missing_by_cell","incomplete_directories","issues","summary_recall_value_counts","sample_rows_checked","minimum_sample_recall_at_10","runner_log_missing_script_errors")},indent=2))


if __name__=="__main__":
    main()
