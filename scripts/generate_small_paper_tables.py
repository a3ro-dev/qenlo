from __future__ import annotations

import csv
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MATRIX = ROOT / "research/artifacts/runpod-small-2026-09-05/report/performance-matrix.csv"
TABLES = ROOT / "paper/tables"


def rows() -> list[dict[str, str]]:
    with MATRIX.open(newline="", encoding="utf-8") as stream:
        return list(csv.DictReader(stream))


def ms(value: str) -> str:
    return f"{float(value) / 1_000_000:.3f}"


def write(name: str, text: str) -> None:
    (TABLES / name).write_text(text.rstrip() + "\n", encoding="utf-8")


def main() -> None:
    data = rows()
    fixed = [
        row
        for row in data
        if row["configuration"] == "rtx4090-eu-ro-secure-deep768-admission-fix"
        and row["status"] == "completed"
    ]
    labels = {
        "current-cpu": "Qenlo CPU",
        "current-gpu": "Qenlo WGPU",
        "faiss-flat": "FAISS Flat",
        "numpy": "NumPy",
        "torch-cpu": "Torch CPU",
        "torch-cuda": "Torch CUDA",
    }
    lines = []
    for workload in (
        "d4-r100000-d768-b1-k1-f1",
        "d5-r100000-d768-b8-k64-f01",
    ):
        for engine in labels:
            row = next(item for item in fixed if item["workload"] == workload and item["engine"] == engine)
            short = "100k, B=1, k=1, 100\\%" if workload.startswith("d4") else "100k, B=8, k=64, 10\\%"
            lines.append(
                f"{short} & {labels[engine]} & {ms(row['p50_completed_ns'])} & "
                f"{ms(row['p95_completed_ns'])} & {row['recall_at_k']} & {row['sample_count']} \\tabularnewline"
            )
    write("small_collection_headline.tex", "\n".join(lines))

    index = {(row["configuration"], row["workload"], row["engine"]): row for row in data}
    pairs = []
    for key, improved in index.items():
        configuration, workload, engine = key
        if engine != "current-gpu" or improved["status"] != "completed" or improved["qualified"] != "True":
            continue
        baseline = index.get((configuration, workload, "baseline-gpu"))
        if baseline and baseline["status"] == "completed" and baseline["qualified"] == "True":
            ratio = float(baseline["p95_completed_ns"]) / float(improved["p95_completed_ns"])
            pairs.append((configuration, workload, ratio))
    write(
        "before_after.tex",
        "\n".join(
            f"{config.replace('_', r'\_')} & {workload.replace('_', r'\_')} & {ratio:.3f} \\tabularnewline"
            for config, workload, ratio in pairs
        ),
    )

    claims = [
        ["C1", "All corrected 100k-by-768 engine rows completed with oracle recall 1.0", "performance-matrix.csv; deep768/...admission-fix/artifacts.tar.gz"],
        ["C2", "Qenlo WGPU P95 is 0.897 ms for 100k-by-768, batch one, k=1", "performance-matrix.csv row d4/current-gpu"],
        ["C3", "Torch CUDA is faster than Qenlo WGPU in both corrected 768-dimensional cells", "performance-matrix.csv rows d4,d5/current-gpu,torch-cuda"],
        ["C4", "The lane-minimum revision wins 5 and loses 7 of 12 qualified baseline-GPU pairs", "performance-report.md qualified paired comparisons"],
        ["C5", "No current Android or iOS performance result exists", "performance-report.md; campaign scope"],
        ["C6", "Final known Runpod spend is 0.9400778694252952 USD and zero pods remain", "ledger.jsonl final billing and pod deletion records"],
    ]
    with (TABLES / "claim-to-artifact.csv").open("w", newline="", encoding="utf-8") as stream:
        writer = csv.writer(stream)
        writer.writerow(["claim_id", "claim", "evidence"])
        writer.writerows(claims)


if __name__ == "__main__":
    main()
