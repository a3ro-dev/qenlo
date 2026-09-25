"""Run one qenlo-bench cell while logging GPU clocks continuously (nvidia-smi, 50 ms).

    python research/scripts/run_dvfs_cell.py OUT_DIR ENGINE ELIGIBLE BATCH SEED [extra bench args...]

Writes OUT_DIR/{bench output}, OUT_DIR/clocks.csv, OUT_DIR/run.json. Local research tool;
it never overwrites an existing destination.
"""
import json, os, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EXE = ROOT / "target/release/qenlo-bench.exe"
DATA = ROOT / "data/ag-news/ag-news-100k-384.qnb"


def run(out: Path, engine: str, eligible: int, batch: int, seed: int, extra=(), reps=3, warmups=200, heater=None):
    """heater=(iters, sleep_ms) starts the duty-cycled gpu_heater example alongside the bench."""
    out = Path(out)
    if out.exists():
        raise SystemExit(f"refusing to overwrite {out}")
    out.mkdir(parents=True)
    log = open(out / "clocks.csv", "w")
    mon = subprocess.Popen(
        ["nvidia-smi", "--query-gpu=timestamp,clocks.sm,clocks.mem,pstate,utilization.gpu,power.draw,temperature.gpu",
         "--format=csv,noheader,nounits", "-lms", "50"], stdout=log, stderr=subprocess.DEVNULL)
    heat = None
    if heater:
        heat = subprocess.Popen([str(ROOT / "target/release/examples/gpu_heater.exe"), str(heater[0]), str(heater[1]), "100000"],
                                stderr=open(out / "heater.log", "w"), env={**os.environ, "WGPU_BACKEND": "vulkan"})
        time.sleep(2.0)
    time.sleep(0.5)
    cmd = [str(EXE), "run", "--dataset", str(DATA), "--output", str(out / "bench"), "--dimensions", "384",
           "--backend", engine, "--distribution", "independent", "--eligible-count", str(eligible),
           "--batch", str(batch), "--k", "10", "--warmups", str(warmups), "--repetitions", str(reps),
           "--recall-target", "0.99", "--order-seed", str(seed), "--diagnostics", "detailed",
           "--vector-budget-mib", "1024", "--gpu-budget-mib", "2048", *extra]
    t0 = time.time()
    p = subprocess.run(cmd, capture_output=True, text=True, env={**os.environ, "WGPU_BACKEND": os.environ.get("WGPU_BACKEND_OVERRIDE", "vulkan")})
    t1 = time.time()
    time.sleep(0.3)
    if heat:
        heat.terminate(); heat.wait()
    mon.terminate(); mon.wait(); log.close()
    json.dump({"command": cmd, "started_unix": t0, "heater": heater, "ended_unix": t1, "returncode": p.returncode,
               "stderr_tail": p.stderr[-2000:]}, open(out / "run.json", "w"), indent=1)
    return p.returncode


if __name__ == "__main__":
    a = sys.argv
    heater = tuple(map(int, os.environ["HEATER"].split(","))) if os.environ.get("HEATER") else None
    sys.exit(run(Path(a[1]), a[2], int(a[3]), int(a[4]), int(a[5]), a[6:], heater=heater))
