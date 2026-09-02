"""Render only directly observed retained 100k x 384 P95 values as SVG."""

import csv
from pathlib import Path


def median(values):
    return sorted(values)[(len(values) - 1) // 2]


def p95_ms(path: Path) -> float:
    with path.open(newline="", encoding="utf-8") as stream:
        return median([int(row["p95_batch_ns"]) for row in csv.DictReader(stream)]) / 1_000_000


def main():
    root = Path(__file__).resolve().parents[1]
    data = [
        ("all rows", "CPU exact", p95_ms(root / "benchmarks/2026-08-28/real/cpu-384-all-v4/runs.csv"), "#555"),
        ("all rows", "GPU predicate", p95_ms(root / "benchmarks/2026-08-28/real/gpu-predicate-384-all-v5/runs.csv"), "#1677c8"),
        ("1% eligible", "CPU exact", p95_ms(root / "benchmarks/2026-08-28/real/cpu-384-onepct/runs.csv"), "#555"),
        ("1% eligible", "GPU predicate", p95_ms(root / "benchmarks/2026-08-28/real/gpu-predicate-384-onepct-v4/runs.csv"), "#1677c8"),
    ]
    out = root / "benchmark-results/2026-09-02-runpod-archive/plots/retained-100k-384-p95.svg"
    out.parent.mkdir(parents=True, exist_ok=True)
    maximum = max(item[2] for item in data)
    rows = []
    for i, (group, label, value, color) in enumerate(data):
        y = 70 + i * 70
        width = round(value / maximum * 560, 1)
        rows.append(f'<text x="20" y="{y-18}" font-size="14">{group}: {label}</text><rect x="250" y="{y-32}" width="{width}" height="26" fill="{color}"/><text x="{260+width}" y="{y-13}" font-size="14">{value:.4f} ms</text>')
    out.write_text("""<svg xmlns="http://www.w3.org/2000/svg" width="900" height="390" viewBox="0 0 900 390">
<rect width="100%" height="100%" fill="white"/><text x="20" y="30" font-size="20" font-family="sans-serif">Retained Windows 100k × 384 median-of-run P95</text>
<text x="20" y="52" font-size="12" font-family="sans-serif">Exact paths; five repetitions; values are not the 1M × 768 gate.</text>""" + "".join(rows) + "</svg>\n", encoding="utf-8")


if __name__ == "__main__":
    main()
