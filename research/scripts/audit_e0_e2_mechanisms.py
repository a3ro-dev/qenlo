"""Mechanism reduction for the partial E0/E2 archive (read-only).

Reads the retained tarball in place (no extraction), verifies its SHA-256 and
coverage, and reduces per-call diagnostics that the latency-only analysis in
runpod-e0-e2-analysis-20260924/ does not use: upload bytes, eligibility
transfer bytes, row-materialization time, device selection/scoring time, CPU
distance path, and before/after GPU clock snapshots. Descriptive only: no
causal claims, no imputation, no outlier removal.

    python research/scripts/audit_e0_e2_mechanisms.py
"""
from __future__ import annotations

import csv
import hashlib
import io
import json
import math
import random
import re
import statistics
import tarfile
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ARCHIVE = ROOT / "research/data/raw/runpod-e0-e2-partial-20260924.tar.gz"
SHA256 = "e4ca1336757cf55a5f5150a8bcc601ab3538cb7e9e9c2270c137e5a72fd51ed0"
OUT = ROOT / "research/data/processed/runpod-e0-e2-mechanisms-20260924"
PLANNED = 20 + 2 * 6 * 4 * 5  # E0 2 engines x 10 blocks; E2 2 B x 6 E x 4 engines x 5 blocks
DIAG = (
    "upload_bytes", "eligibility_transfer_bytes", "row_materialization_ns",
    "device_scoring_ns", "device_selection_ns", "backend_execution_ns", "batch_latency_ns",
)
PATH = re.compile(r"e0-e2/(e[02])/b(\d+)-e(\d+)/block-(\d+)/([a-z-]+)/(summary\.txt|samples\.csv|runs\.csv|configuration\.txt)$")


def props(text):
    return dict(line.split("=", 1) for line in text.splitlines() if "=" in line)


def lower_middle(values):
    a = sorted(values)
    return a[(len(a) - 1) // 2]


def block_q(block, q):
    return lower_middle(int(r[f"{q}_batch_ns"]) for r in block["runs"])


def mhz(snapshot):
    return int(snapshot.split(", ")[3].split()[0])


def main():
    digest = hashlib.sha256(ARCHIVE.read_bytes()).hexdigest()
    assert digest == SHA256, digest
    blocks = defaultdict(dict)
    records = {}
    with tarfile.open(ARCHIVE, "r:gz") as tar:
        names = tar.getnames()
        assert not any(n.endswith("COMPLETE") for n in names), "unexpected COMPLETE marker"
        for member in tar:
            if member.name.endswith(".run.json"):
                records[member.name[:-9]] = json.load(tar.extractfile(member))
                continue
            m = PATH.search(member.name)
            if not m:
                continue
            exp, b, e, blk, eng, kind = m.groups()
            key = (exp, int(b), int(e), int(blk), eng)
            text = tar.extractfile(member).read().decode()
            if kind == "configuration.txt":
                blocks[key].setdefault("configured", True)
            elif kind == "summary.txt":
                blocks[key]["summary"] = props(text)
            elif kind == "runs.csv":
                blocks[key]["runs"] = list(csv.DictReader(io.StringIO(text)))
            else:
                rows = list(csv.DictReader(io.StringIO(text)))
                blocks[key]["n_samples"] = len(rows)
                blocks[key]["diag"] = {
                    c: statistics.median(float(r[c]) for r in rows if r.get(c)) if any(r.get(c) for r in rows) else None
                    for c in DIAG
                }
                blocks[key]["cpu_path"] = Counter(r["cpu_distance_path"] for r in rows).most_common(1)[0][0]
    complete = {k: v for k, v in blocks.items() if "summary" in v}
    partial = sorted(k for k, v in blocks.items() if "summary" not in v)
    exits = {}
    for k, v in complete.items():
        rec = records[f"e0-e2/{k[0]}/b{k[1]}-e{k[2]}/block-{k[3]:02d}/{k[4]}"]
        v["returncode"] = rec["returncode"]
        v["mhz_after"] = mhz(rec["clocks_after"])
        v["p95"] = lower_middle(int(r["p95_batch_ns"]) for r in v["runs"]) / 1e6
        if rec["returncode"]:
            exits["/".join(map(str, k))] = rec["returncode"]
    recalls = Counter(v["summary"]["evaluation_recall_at_10"] for v in complete.values())

    OUT.mkdir(parents=True, exist_ok=True)
    rows = []
    cells = defaultdict(list)
    for k, v in complete.items():
        cells[(k[0], k[1], k[2], k[4])].append(v)
    for (exp, b, e, eng), vs in sorted(cells.items()):
        row = {"experiment": exp, "batch": b, "eligible": e, "engine": eng, "n_blocks": len(vs),
               "cpu_distance_path": ";".join(sorted({v["cpu_path"] for v in vs}))}
        for c in DIAG:
            vals = [v["diag"][c] for v in vs if v["diag"][c] is not None]
            row[f"median_{c}"] = statistics.median(vals) if vals else ""
        rows.append(row)
    with (OUT / "diagnostics_by_cell.csv").open("w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0]))
        w.writeheader()
        w.writerows(rows)

    # Clock association: GPU blocks at B=1, split by end-of-run SM clock snapshot.
    clock_rows = []
    for k, v in sorted(complete.items()):
        if k[4] != "cpu" and k[1] == 1:
            clock_rows.append({"experiment": k[0], "eligible": k[2], "block": k[3], "engine": k[4],
                               "mhz_after": v["mhz_after"], "p95_ms": round(v["p95"], 6)})
    with (OUT / "gpu_clock_snapshots_b1.csv").open("w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=list(clock_rows[0]))
        w.writeheader()
        w.writerows(clock_rows)
    # Within each (experiment, E, engine) cell, does the fastest-P95 half have higher end clocks?
    concord = Counter()
    for (exp, b, e, eng), vs in cells.items():
        if eng == "cpu" or b != 1 or len(vs) < 2:
            continue
        for i in range(len(vs)):
            for j in range(i + 1, len(vs)):
                dp, dc = vs[i]["p95"] - vs[j]["p95"], vs[i]["mhz_after"] - vs[j]["mhz_after"]
                concord["tie" if dp == 0 or dc == 0 else ("concordant" if dp * dc < 0 else "discordant")] += 1

    # EDB ordering witness: CPU vs gpu-rows per block, paired by block, same host and binary.
    rng = random.Random(20260924)
    witness = []
    for exp, b, e in (("e0", 1, 3000), ("e2", 1, 3000), ("e2", 16, 100)):
        for q in ("p50", "p95"):
            pairs = []
            for blk in range(10):
                cpu, gpu = complete.get((exp, b, e, blk, "cpu")), complete.get((exp, b, e, blk, "gpu-rows"))
                if cpu and gpu:
                    pairs.append(math.log(block_q(gpu, q) / block_q(cpu, q)))
            boots = sorted(statistics.median(rng.choices(pairs, k=len(pairs))) for _ in range(20000))
            witness.append({
                "experiment": exp, "batch": b, "eligible": e, "edb": e * 384 * b, "metric": q,
                "paired_blocks": len(pairs), "cpu_faster_blocks": sum(d > 0 for d in pairs),
                "rows_over_cpu": round(math.exp(statistics.median(pairs)), 4),
                "ci95_low": round(math.exp(boots[499]), 4), "ci95_high": round(math.exp(boots[19499]), 4),
            })
    with (OUT / "edb_ordering_witness.csv").open("w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=list(witness[0]))
        w.writeheader()
        w.writerows(witness)

    audit = {
        "archive_sha256": digest,
        "planned_summaries": PLANNED,
        "present_summaries": len(complete),
        "complete_marker": False,
        "summary_less_destinations": ["/".join(map(str, k)) for k in partial],
        "nonzero_exit_with_summary": exits,
        "evaluation_recall_at_10_counts": dict(recalls),
        "recall_0_99998_cells": sorted({f"b{k[1]}-e{k[2]}/{k[4]}" for k, v in complete.items()
                                        if v["summary"]["evaluation_recall_at_10"] != "1"}),
        "b1_gpu_clock_vs_p95_pairs": dict(concord),
        "note": "concordant = the block with the higher end-of-run SM clock snapshot has the lower P95; "
                "within-cell pairs only; snapshot association, not continuous telemetry or causation",
    }
    (OUT / "audit.json").write_text(json.dumps(audit, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(audit, indent=2))


if __name__ == "__main__":
    main()
