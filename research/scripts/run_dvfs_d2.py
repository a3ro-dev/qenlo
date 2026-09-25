"""Run D2 (research/experiments/dvfs-d2/protocol.md). Resumable; never overwrites."""
import os, random, sys, time
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
import run_dvfs_cell

OUT = Path(__file__).resolve().parents[1] / "data/raw/2026-09-25-dvfs-d2"
conds = [("dx12", e, n, 1, h) for e in ["gpu-rows", "gpu-predicate"] for n in [1000, 3000] for h in [0, 1]]
conds += [("vulkan", e, n, 16, h) for e in ["gpu-rows", "cpu"] for n in [1000, 3000] for h in [0, 1]]
for block in range(3):
    order = conds[:]
    random.Random(2000 + block).shuffle(order)
    for api, engine, eligible, batch, heat in order:
        dest = OUT / f"block-{block}" / f"{api}-{engine}-b{batch}-e{eligible}-h{heat}"
        if (dest / "run.json").exists():
            continue
        os.environ["WGPU_BACKEND_OVERRIDE"] = api
        rc = run_dvfs_cell.run(dest, engine, eligible, batch, 9101 + block, heater=(500, 1) if heat else None)
        print(time.strftime("%H:%M:%S"), block, api, engine, batch, eligible, heat, "rc", rc, flush=True)
        time.sleep(5)
print("D2 COMPLETE", flush=True)
