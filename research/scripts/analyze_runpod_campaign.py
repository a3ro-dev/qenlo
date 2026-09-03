"""Rebuild Runpod campaign tables, paired intervals, router profile, and figures."""

import argparse
import csv
import json
import random
import re
from pathlib import Path

import matplotlib.pyplot as plt


def properties(path):
    return dict(line.rstrip().split("=", 1) for line in path.read_text().splitlines() if "=" in line)


def lower_middle(values):
    values = sorted(values)
    return values[(len(values) - 1) // 2]


def paired_interval(left, right, seed=20260902, samples=20_000):
    pairs = list(zip(left, right))
    if not pairs:
        return None
    rng = random.Random(seed)
    estimates = []
    for _ in range(samples):
        draw = [pairs[rng.randrange(len(pairs))] for _ in pairs]
        estimates.append(lower_middle([a for a, _ in draw]) - lower_middle([b for _, b in draw]))
    estimates.sort()
    return [estimates[int(samples * 0.025)], estimates[int(samples * 0.975)]]


def load_runs(root):
    rows = []
    for path in root.glob("runs/*/result/runs.csv"):
        name = path.parents[1].name
        config = properties(path.with_name("configuration.txt"))
        with path.open(newline="") as stream:
            run = next(csv.DictReader(stream))
        row = {"name": name, "backend": config["backend"], "eligible": int(config["eligible_count"]),
               "seed": int(config.get("order_seed", 0)), "p95_ns": int(run["p95_batch_ns"]),
               "qps": float(run["qps"]), "recall": float(run["recall_at_10"]),
               "row_mode": config.get("gpu_row_preparation", "one-pass")}
        match = re.search(r"ef(\d+)", name)
        row["ef_search"] = int(match.group(1)) if match else None
        rows.append(row)
    for path in root.glob("runs/headline-faiss-flat-*/result/runs.csv"):
        name = path.parents[1].name
        replicate = int(re.search(r"r(\d+)$", name).group(1))
        reference_config = root / "runs" / f"headline-dense-cpu-r{replicate:02d}" / "result" / "configuration.txt"
        with path.open(newline="") as stream:
            run = next(csv.DictReader(stream))
        rows.append({"name": name, "backend": "faiss-flat", "eligible": 100000,
                     "seed": int(properties(reference_config)["order_seed"]),
                     "p95_ns": int(run["p95_ns"]), "qps": float(run["qps"]),
                     "recall": float(run["recall_at_10"]), "row_mode": "none", "ef_search": None})
    return rows


def write_csv(path, rows):
    if not rows:
        return
    with path.open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader(); writer.writerows(rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    rows = load_runs(args.input)
    write_csv(args.output / "run-level.csv", rows)

    compact = [r for r in rows if r["name"].startswith("compact-")]
    groups = {}
    for row in compact:
        key = (row["eligible"], row["backend"], row["row_mode"])
        groups.setdefault(key, []).append(row["p95_ns"])
    compact_summary = [{"eligible": key[0], "backend": key[1], "row_mode": key[2],
                        "runs": len(values), "median_p95_ns": lower_middle(values)}
                       for key, values in sorted(groups.items())]
    write_csv(args.output / "compact-summary.csv", compact_summary)
    for label, backend, mode in [("CPU", "cpu", "one-pass"), ("legacy", "gpu-rows", "legacy-two-pass"),
                                 ("one-pass", "gpu-rows", "one-pass"), ("cached", "gpu-rows", "cached")]:
        points = [(r["eligible"], r["median_p95_ns"] / 1e6) for r in compact_summary
                  if r["backend"] == backend and r["row_mode"] == mode]
        if points:
            plt.plot(*zip(*points), marker="o", label=label)
    plt.xscale("log"); plt.yscale("log"); plt.xlabel("Eligible rows")
    plt.ylabel("Run-level P95 latency (ms)"); plt.legend(); plt.tight_layout()
    plt.savefig(args.output / "compact-row-latency.pdf"); plt.close()

    ann = [r for r in rows if r["ef_search"] is not None]
    ann_summary = []
    for key in sorted({(r["eligible"], r["ef_search"]) for r in ann}):
        values = [r for r in ann if (r["eligible"], r["ef_search"]) == key]
        ann_summary.append({"eligible": key[0], "ef_search": key[1], "runs": len(values),
                            "median_p95_ns": lower_middle([r["p95_ns"] for r in values]),
                            "median_recall_at_10": lower_middle([r["recall"] for r in values])})
    write_csv(args.output / "ann-summary.csv", ann_summary)
    for eligible in sorted({r["eligible"] for r in ann_summary}):
        points = [r for r in ann_summary if r["eligible"] == eligible]
        plt.plot([r["median_p95_ns"] / 1e6 for r in points],
                 [r["median_recall_at_10"] for r in points], marker="o", label=f"E={eligible}")
    if ann_summary:
        plt.xlabel("Run-level P95 latency (ms)"); plt.ylabel("Recall@10")
        plt.legend(fontsize=7); plt.tight_layout(); plt.savefig(args.output / "ann-frontiers.pdf")
    plt.close()

    cpu_by_seed = {r["seed"]: r["p95_ns"] for r in rows if r["name"].startswith("headline-dense-cpu-")}
    faiss_by_seed = {r["seed"]: r["p95_ns"] for r in rows if r["backend"] == "faiss-flat"}
    paired_seeds = sorted(cpu_by_seed.keys() & faiss_by_seed.keys())
    cpu = [cpu_by_seed[seed] for seed in paired_seeds]
    faiss = [faiss_by_seed[seed] for seed in paired_seeds]
    matched = []
    if cpu and faiss:
        matched.append({"system": "Qenlo CPU exact", "runs": len(cpu), "median_p95_ns": lower_middle(cpu)})
        matched.append({"system": "FAISS IndexFlatIP", "runs": len(faiss), "median_p95_ns": lower_middle(faiss)})
        interval = paired_interval(cpu, faiss)
    else:
        interval = None
    write_csv(args.output / "faiss-matched.csv", matched)

    qualified = []
    for eligible in sorted({r["eligible"] for r in compact}):
        cpu_values = groups.get((eligible, "cpu", "one-pass"), [])
        gpu_values = groups.get((eligible, "gpu-rows", "one-pass"), [])
        ci = paired_interval(gpu_values, cpu_values)
        if ci and ci[1] < 0:
            qualified.append(eligible)
    threshold = min(qualified) if qualified else 4096
    adapter = "unknown"
    configs = list(args.input.glob("runs/compact-*/result/configuration.txt"))
    if configs:
        adapter = properties(configs[0]).get("gpu_adapter", "unknown").strip('"')
    profile = {"adapter_name": adapter, "dimension": 384, "batch_size": 1,
               "filter_mode": "gpu-rows", "cached_rows": False,
               "gpu_min_eligible_rows": threshold,
               "selection_rule": "smallest eligible count whose paired 95% bootstrap CI favors one-pass WGPU"}
    (args.output / "router-profile.txt").write_text("\n".join(f"{k}={v}" for k, v in profile.items()) + "\n")
    (args.output / "analysis.json").write_text(json.dumps({"run_count": len(rows),
        "faiss_minus_qenlo_p95_ns_ci": interval, "router_profile": profile,
        "claim_gate": "results support only the evaluated hardware, corpus, and resource envelope"}, indent=2) + "\n")


if __name__ == "__main__":
    main()
