"""Run the D1 protocol (research/experiments/dvfs-d1/protocol.md). Resumable; never overwrites."""
import random, sys, time
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
from run_dvfs_cell import run

OUT = Path(__file__).resolve().parents[1] / "data/raw/2026-09-25-dvfs-d1"
conds = [(e, n, h) for e in ["cpu", "gpu-rows", "gpu-predicate"] for n in [100, 1000, 3000, 10000, 30000] for h in [0, 1]]
for block in range(5):
    order = conds[:]
    random.Random(1000 + block).shuffle(order)
    for engine, eligible, heat in order:
        dest = OUT / f"block-{block}" / f"{engine}-e{eligible}-h{heat}"
        if (dest / "run.json").exists():
            continue
        rc = run(dest, engine, eligible, 1, 9001 + block, heater=(500, 1) if heat else None)
        print(time.strftime("%H:%M:%S"), block, engine, eligible, heat, "rc", rc, flush=True)
        time.sleep(5)
print("D1 COMPLETE", flush=True)
