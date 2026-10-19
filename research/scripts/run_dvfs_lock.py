"""Run the E-LOCK protocol (research/experiments/dvfs-lock/protocol.md). Resumable; never overwrites.

Must run from an elevated (Administrator) shell on the laptop, plugged in:

    python research/scripts/run_dvfs_lock.py            # full run, 150 processes
    python research/scripts/run_dvfs_lock.py --dry-run  # checks only, no locks, no runs

Clocks are reset with nvidia-smi -rgc / -rmc after every run and on every exit path. If the
window is killed hard, run `nvidia-smi -rgc` and `nvidia-smi -rmc` (or reboot) to reset.
"""
import ctypes, hashlib, json, random, subprocess, sys, threading, time
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
from run_dvfs_cell import run, ROOT, EXE

OUT = ROOT / "research/data/raw/2026-10-19-dvfs-lock"
HEATER_EXE = ROOT / "target/release/examples/gpu_heater.exe"
HASHES = {EXE: "4f49932c080b2d37ee5b9219eba12e80c53b881ad79749671150ee1565d086c4",
          HEATER_EXE: "6fa819de50c8233fd970420d2f997e7d9e869abd7f4f35b07284aabaa000269a"}
CLOCKS = {"U": None, "Lhi": (2670, 8001), "Llo": (585, 810)}  # (SM MHz, memory MHz)
CONDS = [("U", 0), ("U", 1), ("Lhi", 0), ("Lhi", 1), ("Llo", 0)]
START_MAX_C, ABORT_C = 70, 90


def smi(*args):
    p = subprocess.run(["nvidia-smi", *args], capture_output=True, text=True)
    return p.returncode, (p.stdout + p.stderr).strip()


def query(fields):
    _, out = smi(f"--query-gpu={fields}", "--format=csv,noheader,nounits")
    return [x.strip() for x in out.splitlines()[0].split(",")]


def temp():
    return int(query("temperature.gpu")[0])


def on_ac():
    class S(ctypes.Structure):
        _fields_ = [("ac", ctypes.c_byte), ("flag", ctypes.c_byte), ("pct", ctypes.c_byte), ("r", ctypes.c_byte),
                    ("life", ctypes.c_ulong), ("full", ctypes.c_ulong)]
    s = S(); ctypes.windll.kernel32.GetSystemPowerStatus(ctypes.byref(s))
    return s.ac == 1


def reset():
    return {"rgc": smi("-rgc"), "rmc": smi("-rmc")}


def lock(sm, mem, with_mem):
    r = {"lgc": smi("-lgc", f"{sm},{sm}")}
    if with_mem:
        r["lmc"] = smi("-lmc", f"{mem},{mem}")
    return r


class Poller(threading.Thread):
    """1 s poll of clocks, reasons and temperature; resets clocks at ABORT_C (driver then manages thermals)."""
    FIELDS = "timestamp,clocks.sm,clocks.mem,pstate,temperature.gpu,clocks_event_reasons.active,pcie.link.gen.current"

    def __init__(self):
        super().__init__(daemon=True); self.rows, self.hot, self.stop = [], False, threading.Event()

    def run(self):
        while not self.stop.wait(1.0):
            try:
                row = query(self.FIELDS)
            except Exception:
                continue
            self.rows.append(row)
            if row[4].isdigit() and int(row[4]) >= ABORT_C and not self.hot:
                self.hot = True; reset()


def preflight(dry):
    pf = {"time": time.strftime("%Y-%m-%d %H:%M:%S"), "admin": bool(ctypes.windll.shell32.IsUserAnAdmin()),
          "ac_power": on_ac(), "temperature_c": temp(), "hashes_ok": {}, "lmc_supported": None}
    for path, want in HASHES.items():
        pf["hashes_ok"][str(path)] = hashlib.sha256(path.read_bytes()).hexdigest() == want
    problems = [k for k in ("admin", "ac_power") if not pf[k]] + [p for p, ok in pf["hashes_ok"].items() if not ok]
    if not dry and not problems:
        rc_g, out_g = smi("-lgc", "2670,2670")
        rc_m, out_m = smi("-lmc", "8001,8001")
        pf["probe"] = {"lgc": [rc_g, out_g], "lmc": [rc_m, out_m]}
        time.sleep(1.0)
        pf["probe_query"] = smi("-q", "-d", "CLOCK,PERFORMANCE")[1]
        pf["probe_reset"] = reset()
        if rc_g != 0:
            problems.append("nvidia-smi -lgc failed: " + out_g)
        pf["lmc_supported"] = rc_m == 0
    pf["problems"] = problems
    return pf


def main():
    dry = "--dry-run" in sys.argv
    pf = preflight(dry)
    print(json.dumps({k: v for k, v in pf.items() if k != "probe_query"}, indent=1), flush=True)
    if pf["problems"]:
        print("PREFLIGHT FAILED:", pf["problems"], flush=True)
        if not dry:
            sys.exit(2)
    if dry:
        print("dry run: checks only, nothing locked or run", flush=True); return
    OUT.mkdir(parents=True, exist_ok=True)
    stamp = time.strftime("%Y%m%d-%H%M%S")
    json.dump(pf, open(OUT / f"preflight-{stamp}.json", "w"), indent=1)
    log = open(OUT / "runner.log", "a")
    say = lambda *a: (print(*a, flush=True), print(*a, file=log, flush=True))
    cells = [(e, n) for e in ["gpu-rows", "gpu-predicate"] for n in [1000, 3000, 10000]]
    conds = [(e, n, c, h) for e, n in cells for c, h in CONDS]
    done = sum((OUT / f"block-{b}" / f"{e}-e{n}-{c}-h{h}" / "run.json").exists() for b in range(5) for e, n, c, h in conds)
    say(time.strftime("%H:%M:%S"), f"E-LOCK start, {done}/150 already done, lmc_supported={pf['lmc_supported']}")
    try:
        for block in range(5):
            order = conds[:]
            random.Random(4000 + block).shuffle(order)
            for engine, eligible, cond, heat in order:
                dest = OUT / f"block-{block}" / f"{engine}-e{eligible}-{cond}-h{heat}"
                if (dest / "run.json").exists():
                    continue
                if dest.exists():  # interrupted run: keep it aside, never reuse it
                    dest.rename(dest.with_name(dest.name + f".aborted-{stamp}"))
                if not on_ac():
                    say("STOP: laptop is not on AC power"); return
                waited = 0
                while temp() > START_MAX_C:
                    if waited >= 600:
                        say(f"STOP: GPU still above {START_MAX_C} C after 10 min"); return
                    time.sleep(10); waited += 10
                info = {"condition": cond, "heater": heat, "requested": CLOCKS[cond],
                        "temperature_start_c": temp(), "cooldown_wait_s": waited, "reset_before": reset()}
                if CLOCKS[cond]:
                    info["lock"] = lock(*CLOCKS[cond], pf["lmc_supported"])
                info["query_before"] = smi("-q", "-d", "CLOCK,PERFORMANCE")[1]
                poll = Poller(); poll.start()
                try:
                    rc = run(dest, engine, eligible, 1, 9301 + block, heater=(500, 1) if heat else None)
                finally:
                    poll.stop.set(); poll.join()
                    info["reset_after"] = reset()
                info.update(returncode=rc, thermal_abort=poll.hot, temperature_max_c=max(
                    (int(r[4]) for r in poll.rows if r[4].isdigit()), default=None))
                json.dump(info, open(dest / "lock.json", "w"), indent=1)
                with open(dest / "poll.csv", "w") as f:
                    f.write(Poller.FIELDS + "\n" + "".join(",".join(r) + "\n" for r in poll.rows))
                say(time.strftime("%H:%M:%S"), block, engine, eligible, cond, f"h{heat}", "rc", rc,
                    f"tmax {info['temperature_max_c']}C", "THERMAL ABORT" if poll.hot else "")
                if poll.hot:
                    say(f"STOP: GPU reached {ABORT_C} C; clocks reset. Let it cool, then rerun to resume."); return
                time.sleep(5)
        say(time.strftime("%H:%M:%S"), "E-LOCK COMPLETE")
    finally:
        r = reset()
        say("clocks reset:", r["rgc"][0], r["rmc"][0])


if __name__ == "__main__":
    main()
