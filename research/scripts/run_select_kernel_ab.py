"""Light-path selection kernel A/B: k-pass (control) vs tiled, device scoring + selection time.

    python research/scripts/run_select_kernel_ab.py RAW_DIR SUMMARY_CSV [BLOCKS]

One fresh qenlo-bench process per (block, E, k, kernel); kernel order alternates per block.
Clocks are whatever the driver picks (not locked). Raw bench directories go to RAW_DIR,
which must not exist; the per-cell medians go to SUMMARY_CSV.
"""
import csv, statistics, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EXE = ROOT / "target/release/qenlo-bench.exe"
DATA = Path("D:/qenloDB/data/ag-news/ag-news-100k-384.qnb")
ELIGIBLE = [1_000, 3_000, 10_000, 30_000]
KS = [10, 64]
KERNELS = ["k-pass", "tiled"]


def cell(out, kernel, eligible, k):
    subprocess.run([str(EXE), "run", "--dataset", str(DATA), "--output", str(out),
                    "--dimensions", "384", "--backend", "gpu-rows", "--distribution", "independent",
                    "--eligible-count", str(eligible), "--batch", "1", "--k", str(k),
                    "--warmups", "200", "--repetitions", "3", "--recall-target", "0.99",
                    "--diagnostics", "detailed", "--vector-budget-mib", "1024",
                    "--gpu-budget-mib", "2048", "--gpu-select-kernel", kernel],
                   check=True, stdout=subprocess.DEVNULL)
    rows = list(csv.DictReader(open(out / "samples.csv")))
    assert all(float(r["recall_at_k"]) == 1.0 for r in rows), f"{out}: recall below 1"
    scoring = [int(r["device_scoring_ns"]) for r in rows]
    selection = [int(r["device_selection_ns"]) for r in rows]
    return {"scoring_ns": statistics.median(scoring), "selection_ns": statistics.median(selection),
            "total_ns": statistics.median(a + b for a, b in zip(scoring, selection))}


def main():
    raw, summary = Path(sys.argv[1]), Path(sys.argv[2])
    blocks = int(sys.argv[3]) if len(sys.argv) > 3 else 3
    raw.mkdir(parents=True)
    with open(summary, "w", newline="") as f:
        writer = csv.DictWriter(f, ["block", "eligible", "k", "kernel", "scoring_ns", "selection_ns", "total_ns"])
        writer.writeheader()
        for block in range(blocks):
            for eligible in ELIGIBLE:
                for k in KS:
                    for kernel in KERNELS[::-1] if block % 2 else KERNELS:
                        result = cell(raw / f"b{block}-e{eligible}-k{k}-{kernel}", kernel, eligible, k)
                        writer.writerow({"block": block, "eligible": eligible, "k": k, "kernel": kernel, **result})
                        f.flush()
                        print(block, eligible, k, kernel, result, flush=True)


if __name__ == "__main__":
    main()
