#!/usr/bin/env python3
"""Reanalyze retained benchmark evidence without rerunning experiments.

The script deliberately keeps incompatible cohorts separate. It verifies the
four source archives used by the new synthesis, reduces their machine-readable
records, evaluates only explicitly defined routing counterfactuals, and writes
an inventory of every tracked or selected archive-internal ``samples.csv``.
"""
from __future__ import annotations

import csv
import hashlib
import io
import json
import math
import statistics
import subprocess
import tarfile
from collections import Counter, defaultdict
from pathlib import Path
from typing import Iterable


ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "research/data/processed/archive-reanalysis"

ARCHIVES = {
    "phase0": (
        ROOT / "research/artifacts/qenlo-phase0-runpod-complete-2026-09-04.tar.gz",
        "7292fd1cd0e7669c9febeaa2d49baedee747811ed65f6bc704ecbcb6e61f51ea",
    ),
    "phase1": (
        ROOT / "research/artifacts/qenlo-phase1-eligibility-2026-09-04.tar.gz",
        "26e0ccc057c59c0fb4687f8d85a923e88b7bb25257e4e551e22cffe13c1b821f",
    ),
    "phase2": (
        ROOT / "research/artifacts/qenlo-phase2-a40-2026-09-04-final.tar.gz",
        "ac3afda446f4943eab128cff458402f7559be53b96c3cf8ce44fcbbd7f8be4df",
    ),
    "alpha5_heldout": (
        ROOT / "research/data/raw/alpha5-router-heldout-rtx4050.tar.gz",
        "8e3b5fe6ec6ab9f295752ec894c89b094c4ddf4dd0ee38849d9d78c9a7fcc01e",
    ),
}


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def verify_archives() -> dict[str, dict[str, object]]:
    result = {}
    for name, (path, expected) in ARCHIVES.items():
        actual = sha256_file(path)
        if actual != expected:
            raise AssertionError(f"{name} archive hash mismatch: {actual} != {expected}")
        result[name] = {
            "path": path.relative_to(ROOT).as_posix(),
            "sha256": actual,
            "bytes": path.stat().st_size,
        }
    return result


def properties(text: str) -> dict[str, str]:
    return dict(
        line.split("=", 1)
        for line in text.splitlines()
        if "=" in line and not line.lstrip().startswith("#")
    )


def tar_bytes(archive: tarfile.TarFile, member: str) -> bytes:
    stream = archive.extractfile(member)
    if stream is None:
        raise FileNotFoundError(f"missing tar member: {member}")
    return stream.read()


def tar_text(archive: tarfile.TarFile, member: str) -> str:
    return tar_bytes(archive, member).decode("utf-8")


def write_csv(path: Path, rows: list[dict[str, object]]) -> None:
    if not rows:
        raise AssertionError(f"refusing to write empty reduction: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]), lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def lower_median(values: Iterable[float]) -> float:
    ordered = sorted(values)
    if not ordered:
        raise ValueError("empty median")
    return ordered[(len(ordered) - 1) // 2]


def phase0_rows() -> list[dict[str, object]]:
    path = ARCHIVES["phase0"][0]
    rows = []
    with tarfile.open(path, "r:gz") as archive:
        names = set(archive.getnames())
        for summary_name in sorted(n for n in names if n.endswith("/summary.txt") and "/phase0-" in n):
            directory = summary_name.rsplit("/", 1)[0]
            cell = directory.rsplit("/", 1)[1]
            if not cell.startswith("e") or "-" not in cell:
                continue
            config_name = directory + "/configuration.txt"
            if config_name not in names:
                continue
            config = properties(tar_text(archive, config_name))
            summary = properties(tar_text(archive, summary_name))
            engine = cell.split("-", 1)[1]
            rows.append({
                "cohort": "phase0-rtx4090-linux-vulkan",
                "source_archive_sha256": ARCHIVES["phase0"][1],
                "eligible_rows": int(config["eligible_count"]),
                "dimension": int(config["dimensions"]),
                "batch": int(config["batch"]),
                "k": int(config["k"]),
                "engine": engine,
                "p95_ns": int(float(summary["median_run_p95_batch_ns"])),
                "recall_at_10": float(summary["evaluation_recall_at_10"]),
                "runs": int(config["repetitions"]),
                "evaluation_queries_per_run": int(config["evaluation_range"].split("..")[1]) - int(config["evaluation_range"].split("..")[0]),
                "source_revision": config["git_revision"],
                "member": directory,
            })
    if len(rows) != 19:
        raise AssertionError(f"expected 19 Phase 0 cells, found {len(rows)}")
    return rows


def crossover_rows(phase0: list[dict[str, object]]) -> list[dict[str, object]]:
    rows = []
    with (ROOT / "research/data/processed/native_crossover_summary.csv").open(newline="", encoding="utf-8") as stream:
        for item in csv.DictReader(stream):
            cpu = float(item["cpu_p95_ms"]) * 1e6
            gpu = float(item["gpu_rows_p95_ms"]) * 1e6
            rows.append({
                "cohort": "h1-rtx4050-windows-dx12",
                "eligible_rows": int(item["eligible"]),
                "dimension": 384,
                "batch": 1,
                "k": 10,
                "cpu_p95_ns": round(cpu),
                "gpu_p95_ns": round(gpu),
                "winner": "cpu" if cpu < gpu else "gpu",
                "loser_over_winner": max(cpu, gpu) / min(cpu, gpu),
                "source": "research/data/raw/2026-09-02-native-crossover",
            })
    grouped: dict[int, dict[str, dict[str, object]]] = defaultdict(dict)
    for item in phase0:
        if item["engine"] in {"cpu", "gpu-rows"}:
            grouped[int(item["eligible_rows"])][str(item["engine"])] = item
    for eligible, engines in sorted(grouped.items()):
        if set(engines) != {"cpu", "gpu-rows"}:
            continue
        cpu = float(engines["cpu"]["p95_ns"])
        gpu = float(engines["gpu-rows"]["p95_ns"])
        rows.append({
            "cohort": "phase0-rtx4090-linux-vulkan",
            "eligible_rows": eligible,
            "dimension": 384,
            "batch": 1,
            "k": 10,
            "cpu_p95_ns": round(cpu),
            "gpu_p95_ns": round(gpu),
            "winner": "cpu" if cpu < gpu else "gpu",
            "loser_over_winner": max(cpu, gpu) / min(cpu, gpu),
            "source": ARCHIVES["phase0"][0].relative_to(ROOT).as_posix(),
        })
    if len(rows) != 14:
        raise AssertionError(f"expected 14 crossover rows, found {len(rows)}")
    return rows


def threshold_metrics(rows: list[dict[str, object]], threshold: float, field: str) -> dict[str, float | int]:
    regrets = []
    wrong = 0
    for row in rows:
        chosen = "gpu" if float(row[field]) >= threshold else "cpu"
        cpu = float(row["cpu_p95_ns"])
        gpu = float(row["gpu_p95_ns"])
        best = min(cpu, gpu)
        chosen_value = gpu if chosen == "gpu" else cpu
        regret = chosen_value / best - 1
        regrets.append(regret)
        wrong += regret > 0
    return {
        "wrong_routes": wrong,
        "median_regret": statistics.median(regrets),
        "mean_regret": statistics.fmean(regrets),
        "max_regret": max(regrets),
    }


def best_threshold(rows: list[dict[str, object]], field: str) -> dict[str, object]:
    values = sorted(set(float(row[field]) for row in rows))
    candidates = [values[0] - 1.0, *(value + 0.5 for value in values), values[-1] + 1.0]
    scored = []
    for threshold in candidates:
        metrics = threshold_metrics(rows, threshold, field)
        scored.append((metrics["max_regret"], metrics["wrong_routes"], metrics["mean_regret"], threshold, metrics))
    _, _, _, threshold, metrics = min(scored)
    return {"threshold": threshold, **metrics}


def monotone_witnesses(rows: list[dict[str, object]]) -> list[dict[str, object]]:
    return [
        {
            "k": lower["k"],
            "lower_work_gpu_winner": lower["name"],
            "lower_work_units": lower["work_units"],
            "higher_work_cpu_winner": higher["name"],
            "higher_work_units": higher["work_units"],
        }
        for lower in rows for higher in rows
        if lower["k"] == higher["k"]
        and lower["winner"] == "gpu" and higher["winner"] == "cpu"
        and int(lower["work_units"]) <= int(higher["work_units"])
    ]


def heldout_rows() -> tuple[list[dict[str, object]], dict[str, object]]:
    path = ARCHIVES["alpha5_heldout"][0]
    rows = []
    with tarfile.open(path, "r:gz") as archive:
        manifest_member = next(name for name in archive.getnames() if name.endswith("/manifest.json"))
        manifest = json.loads(tar_text(archive, manifest_member))
        prefix = manifest_member.rsplit("/", 1)[0]
        threshold = int(manifest["candidate"]["work_threshold"])
        for workload in manifest["workloads"]:
            values = {}
            for engine in ("cpu", "gpu-rows", "automatic"):
                summary = properties(tar_text(archive, f"{prefix}/{workload['name']}/{engine}/summary.txt"))
                values[engine] = int(float(summary["median_run_p95_batch_ns"]))
                if summary["recall_target_passed"] != "true" or int(summary["filter_violations"]) != 0:
                    raise AssertionError(f"held-out qualification failure: {workload['name']} {engine}")
            winner = "cpu" if values["cpu"] < values["gpu-rows"] else "gpu"
            chosen = "gpu" if workload["work_units"] >= threshold else "cpu"
            best = min(values["cpu"], values["gpu-rows"])
            selected = values["gpu-rows"] if chosen == "gpu" else values["cpu"]
            rows.append({
                **workload,
                "threshold": threshold,
                "cpu_p95_ns": values["cpu"],
                "gpu_p95_ns": values["gpu-rows"],
                "automatic_p95_ns": values["automatic"],
                "winner": winner,
                "threshold_choice": chosen,
                "choice_regret": selected / best - 1,
                "automatic_over_best": values["automatic"] / best,
            })
    metrics = threshold_metrics(rows, threshold, "work_units")
    if metrics["wrong_routes"] != 8 or not math.isclose(float(metrics["max_regret"]), 2.3565180285897163):
        raise AssertionError(f"held-out metrics changed: {metrics}")
    return rows, {
        "preregistered_threshold": threshold,
        **metrics,
        "posthoc_minimax_threshold": best_threshold(rows, "work_units"),
        "same_k_monotone_witnesses": monotone_witnesses(rows),
        "automatic_over_best_min": min(float(row["automatic_over_best"]) for row in rows),
        "automatic_over_best_max": max(float(row["automatic_over_best"]) for row in rows),
    }


def phase2_rows() -> list[dict[str, object]]:
    path = ARCHIVES["phase2"][0]
    rows = []
    with tarfile.open(path, "r:gz") as archive:
        names = set(archive.getnames())
        for role in ("baseline", "optimized"):
            for eligible in (1000, 4000, 100000):
                directory = f"artifacts/{role}/cells/cpu-e{eligible}"
                summary = properties(tar_text(archive, directory + "/summary.txt"))
                config = properties(tar_text(archive, directory + "/configuration.txt"))
                rows.append({
                    "role": role,
                    "engine": "qenlo-cpu",
                    "eligible_rows": eligible,
                    "p95_ns": int(float(summary["median_run_p95_batch_ns"])),
                    "recall_at_10": float(summary["evaluation_recall_at_10"]),
                    "runs": int(config["repetitions"]),
                    "source_revision": config["git_revision"],
                    "source_archive_sha256": ARCHIVES["phase2"][1],
                })
        routed_summary = properties(tar_text(archive, "artifacts/routed-dense/cpu-e100000/summary.txt"))
        rows.append({
            "role": "routed-reference-verification",
            "engine": "qenlo-cpu",
            "eligible_rows": 100000,
            "p95_ns": int(float(routed_summary["median_run_p95_batch_ns"])),
            "recall_at_10": float(routed_summary["evaluation_recall_at_10"]),
            "runs": 5,
            "source_revision": "hash-qualified routed source in archive",
            "source_archive_sha256": ARCHIVES["phase2"][1],
        })
        faiss = json.loads(tar_text(archive, "artifacts/faiss-flat/summary.json"))
        rows.append({
            "role": "external-dense-baseline",
            "engine": "faiss-cpu-flat-single-thread",
            "eligible_rows": int(faiss["corpus_rows"]),
            "p95_ns": int(faiss["median_run_p95_call_ns"]),
            "recall_at_10": float(faiss["mean_recall_at_10"]),
            "runs": int(faiss["repetitions"]),
            "source_revision": f"faiss {faiss['faiss_version']}",
            "source_archive_sha256": ARCHIVES["phase2"][1],
        })
    index = {(row["role"], row["eligible_rows"]): row for row in rows if row["engine"] == "qenlo-cpu"}
    for eligible in (1000, 4000, 100000):
        baseline = index[("baseline", eligible)]
        optimized = index[("optimized", eligible)]
        optimized["change_vs_baseline"] = float(optimized["p95_ns"]) / float(baseline["p95_ns"]) - 1
        baseline["change_vs_baseline"] = 0.0
    for row in rows:
        row.setdefault("change_vs_baseline", "")
    return rows


def classify_cohort(path: str) -> str:
    lowered = path.lower()
    if "alpha5-router-heldout" in lowered:
        return "alpha5-heldout"
    if "runpod-small-2026-09-05" in lowered or "small-matrix-local" in lowered:
        return "small-collection"
    if "native-crossover" in lowered:
        return "native-crossover"
    if "phase0" in lowered:
        return "phase0"
    if "phase1" in lowered:
        return "phase1"
    if "phase2" in lowered or "cpu-optimization" in lowered:
        return "cpu-optimization"
    if "strict-research-gate" in lowered:
        return "a6000-strict"
    if "sota-a6000" in lowered:
        return "a6000-provider"
    if "a6000-cuda" in lowered:
        return "a6000-synthetic"
    if "runpod-archive" in lowered or "2026-09-02-bb51987" in lowered:
        return "linux-library"
    if "device-lab" in lowered or "android" in lowered or "intel-arc" in lowered:
        return "device-lab"
    if "gpu-tuning" in lowered:
        return "gpu-tuning"
    if "/real/" in lowered or "\\real\\" in lowered:
        return "windows-endpoints"
    return "other-retained-benchmark"


def csv_row_count(data: bytes) -> tuple[int, str]:
    text = data.decode("utf-8-sig", errors="strict")
    reader = csv.reader(io.StringIO(text))
    try:
        header = next(reader)
    except StopIteration:
        return 0, ""
    return sum(1 for _ in reader), ",".join(header)


def sample_inventory() -> tuple[list[dict[str, object]], dict[str, object]]:
    tracked = subprocess.run(
        [
            "git", "ls-files", "-z", "--",
            "**/samples.csv", "**/raw_samples.csv", "**/lifecycle.csv",
        ],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
    ).stdout.decode("utf-8").split("\0")
    items: list[dict[str, object]] = []
    seen_locations = set()

    def add(location: str, data: bytes, container: str) -> None:
        if location in seen_locations:
            return
        seen_locations.add(location)
        rows, schema = csv_row_count(data)
        items.append({
            "location": location,
            "container": container,
            "cohort": classify_cohort(location),
            "sample_rows": rows,
            "schema_sha256": sha256_bytes(schema.encode("utf-8")),
            "content_sha256": sha256_bytes(data.replace(b"\r\n", b"\n")),
        })

    for relative in filter(None, tracked):
        path = ROOT / relative
        add(Path(relative).as_posix(), path.read_bytes(), "tracked-file")

    for archive_name, (path, _) in ARCHIVES.items():
        with tarfile.open(path, "r:gz") as archive:
            for member in archive.getmembers():
                if member.isfile() and member.name.endswith(("/samples.csv", "/raw_samples.csv", "/lifecycle.csv")):
                    add(f"{path.relative_to(ROOT).as_posix()}!{member.name}", tar_bytes(archive, member.name), archive_name)

    groups: dict[str, list[dict[str, object]]] = defaultdict(list)
    for item in items:
        groups[str(item["content_sha256"])].append(item)
    for group in groups.values():
        canonical = min(str(item["location"]) for item in group)
        for item in group:
            item["duplicate_of"] = "" if item["location"] == canonical else canonical
            item["disposition"] = "duplicate-mirror" if item["duplicate_of"] else "unique-retained-series"
    items.sort(key=lambda item: str(item["location"]))
    unique = [item for item in items if not item["duplicate_of"]]
    schema_counts = Counter(str(item["schema_sha256"]) for item in unique)
    cohort_counts = Counter(str(item["cohort"]) for item in unique)
    return items, {
        "sample_files_and_archive_members": len(items),
        "unique_sample_series": len(unique),
        "duplicate_sample_series": len(items) - len(unique),
        "unique_sample_rows": sum(int(item["sample_rows"]) for item in unique),
        "unique_csv_schemas": len(schema_counts),
        "unique_series_by_cohort": dict(sorted(cohort_counts.items())),
    }


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    archives = verify_archives()
    phase0 = phase0_rows()
    crossovers = crossover_rows(phase0)
    heldout, heldout_summary = heldout_rows()
    development_summary = json.loads(
        (ROOT / "research/data/processed/alpha5-routing/summary.json").read_text(encoding="utf-8")
    )
    if (development_summary["pair_count"] != 31
            or development_summary["candidate"]["wrong_routes"] != 0
            or development_summary["candidate_work_threshold"] != heldout_summary["preregistered_threshold"]):
        raise AssertionError(f"development/held-out routing identity changed: {development_summary}")
    phase2 = phase2_rows()
    inventory, inventory_summary = sample_inventory()

    write_csv(OUT / "phase0_cells.csv", phase0)
    write_csv(OUT / "crossover_portability.csv", crossovers)
    write_csv(OUT / "alpha5_heldout_router.csv", heldout)
    write_csv(OUT / "phase2_cpu_optimization.csv", phase2)
    write_csv(OUT / "sample_series_inventory.csv", inventory)

    by_cohort = defaultdict(list)
    for row in crossovers:
        by_cohort[str(row["cohort"])].append(row)
    crossover_summary = {
        cohort: {
            "rows": len(rows),
            "cpu_wins": sum(row["winner"] == "cpu" for row in rows),
            "gpu_wins": sum(row["winner"] == "gpu" for row in rows),
            "observed_reversal_bracket": [
                max(int(row["eligible_rows"]) for row in rows if row["winner"] == "cpu"),
                min(int(row["eligible_rows"]) for row in rows if row["winner"] == "gpu"),
            ],
        }
        for cohort, rows in by_cohort.items()
    }
    universal = best_threshold(crossovers, "eligible_rows")
    shipped = threshold_metrics(crossovers, 4096, "eligible_rows")
    contradictions = []
    for eligible in sorted(set(int(row["eligible_rows"]) for row in crossovers)):
        matching = [row for row in crossovers if int(row["eligible_rows"]) == eligible]
        if len({row["winner"] for row in matching}) > 1:
            contradictions.append({"eligible_rows": eligible, "winners": {row["cohort"]: row["winner"] for row in matching}})

    phase2_index = {(row["role"], row["eligible_rows"]): row for row in phase2}
    phase2_summary = {
        "optimized_change_vs_baseline": {
            str(eligible): phase2_index[("optimized", eligible)]["change_vs_baseline"]
            for eligible in (1000, 4000, 100000)
        },
        "faiss_dense_change_vs_qenlo_baseline": (
            float(phase2_index[("external-dense-baseline", 100000)]["p95_ns"])
            / float(phase2_index[("baseline", 100000)]["p95_ns"])
            - 1
        ),
        "faiss_recall_at_10": phase2_index[("external-dense-baseline", 100000)]["recall_at_10"],
        "qenlo_baseline_recall_at_10": phase2_index[("baseline", 100000)]["recall_at_10"],
    }

    summary = {
        "schema": "qenlo-archive-reanalysis-v1",
        "source_archives": archives,
        "sample_series_inventory": inventory_summary,
        "crossover_portability": {
            "cohorts": crossover_summary,
            "contradictory_equal_E_cells": contradictions,
            "posthoc_universal_minimax_threshold": universal,
            "static_4096_policy": shipped,
            "interpretation": "No E-only threshold can classify both retained cohorts because equal-E cells have different winners; thresholds are descriptive and source/environment conditioned.",
        },
        "alpha5_router": {
            "development": {
                "pairs": development_summary["pair_count"],
                "wrong_routes": development_summary["candidate"]["wrong_routes"],
                "max_regret": development_summary["candidate"]["max_regret"],
                "source": "research/data/processed/alpha5-routing/summary.json",
            },
            "heldout": heldout_summary,
        },
        "phase2_cpu_optimization": phase2_summary,
        "claims": [
            "The preregistered E*D*B threshold overfit development evidence: 0/31 development errors became 8/16 held-out errors with 235.7% maximum backend-choice regret.",
            "No increasing E*D*B threshold perfectly classifies the observed medians: a GPU winner at 749568 units precedes CPU winners at larger work with k=10. A post-hoc threshold has one error and 3.16% maximum regret; this does not reject every scalar policy at the release regret limits.",
            "Equal-E 3K and 4K cells have opposite CPU/GPU winners across retained RTX 4050 and RTX 4090 cohorts; a universal E-only threshold is therefore falsified on the observed cells.",
            "Certified FP32 CPU scoring improves sparse A40 cells but regresses the dense cell; dense single-thread FAISS CPU Flat remains 26.9% faster than the frozen Qenlo baseline at slightly lower recall.",
            "Automatic-route timing is not interchangeable with forced-route timing: held-out automatic P95 ranges from 0.412x to 5.393x the faster forced route.",
        ],
        "limitations": [
            "Cohorts differ in source revision, host, device, runtime, and execution order; no latency values are pooled.",
            "Threshold evaluations are workload-level descriptive counterfactuals over median-run P95, not per-query paired inference.",
            "The post-hoc minimax thresholds are diagnostics, not deployable calibrated policies.",
            "Sample rows across schemas are counted for archive coverage only and are not treated as independent observations.",
        ],
    }
    (OUT / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8", newline="\n")

    report = f"""# Full archive reanalysis

This reduction verifies and reads retained evidence only. It does not rerun a benchmark. Incompatible cohorts remain separate, exact duplicate sample series are counted once, and sample-row counts describe archive coverage rather than statistical independence.

## Archive coverage

- {inventory_summary['sample_files_and_archive_members']:,} tracked or selected archive-internal sample files were inspected.
- {inventory_summary['unique_sample_series']:,} content-distinct sample series remain after removing {inventory_summary['duplicate_sample_series']:,} exact mirrors.
- Those unique files contain {inventory_summary['unique_sample_rows']:,} CSV sample rows across {inventory_summary['unique_csv_schemas']} schemas.
- The four synthesis archives passed SHA-256 verification.

## New conclusions

1. **The preregistered scalar router failed held-out evaluation.** The 1,000,000-unit `E*D*B` threshold made no errors on 31 development pairs, then chose the wrong backend on {heldout_summary['wrong_routes']} of 16 held-out workloads. Median choice regret was {heldout_summary['median_regret']:.1%}, but maximum regret was {heldout_summary['max_regret']:.1%}. The release gate correctly rejected it.
2. **Perfect monotone classification and acceptable regret are different claims.** At `k=10`, `d768-b16-e61-k10` favors GPU at 749,568 work units, while `d384-b1-e1953-k10` favors CPU at 749,952 units. That ordering contradicts every increasing threshold on the observed medians; near-equal work alone would not. All same-k witnesses depend on the former cell, whose CPU/GPU medians differ by only 3.16%. The post-hoc threshold 1,499,904.5 has one error and 3.16% maximum regret, below the original regret limits but fitted to the test set. This is not a proof that all scalar policies fail a practical gate, nor a causal identification of query-shape effects.
3. **Eligible count alone is also non-identifying across environments.** The RTX 4050 cohort reverses between 2K and 3K eligible rows; the Phase 0 RTX 4090 cohort reverses between 4K and 6K. At both 3K and 4K, the two cohorts have opposite winners. No universal monotone E-only threshold can classify all 14 retained matched cells. The best post-hoc minimax E threshold still incurs {universal['max_regret']:.1%} maximum regret.
4. **CPU optimization must itself be routed.** The certified FP32 A40 path changes P95 by {phase2_summary['optimized_change_vs_baseline']['1000']:.1%} at E=1K, {phase2_summary['optimized_change_vs_baseline']['4000']:.1%} at E=4K, and {phase2_summary['optimized_change_vs_baseline']['100000']:.1%} at E=100K. Dense single-thread FAISS CPU Flat is {-phase2_summary['faiss_dense_change_vs_qenlo_baseline']:.1%} lower-latency than the frozen Qenlo baseline while recording recall 0.99984 rather than 0.99998.
5. **Backend choice and realized automatic latency are different objects.** Across the held-out suite, automatic-route P95 is {heldout_summary['automatic_over_best_min']:.3f}x to {heldout_summary['automatic_over_best_max']:.3f}x the faster forced-route P95. Sequential run drift and differences in execution paths or state remain competing explanations; this range does not identify routing overhead.

## Resulting paper thesis

The retained campaign rejects the frozen E*D*B rule at its preregistered gate and contradicts perfect universal E-only classification across the named environments. Perfect increasing E*D*B classification also fails descriptively, but the small margin and post-hoc low-regret fit preclude a general impossibility claim. Separate E, D, B, k, total N, predicate/representation, source/device identity, residency and mutation state are candidate covariates, not individually established necessary predictors. A new frozen policy must be evaluated end to end on fresh held-out workloads; no positive calibrated-router advantage is validated.
"""
    (OUT / "report.md").write_text(report, encoding="utf-8", newline="\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
