"""Run the E-SPLIT protocol (research/experiments/dvfs-split/protocol.md). Resumable; never overwrites.

Must run from an elevated (Administrator) shell on the laptop, plugged in:

    python research/scripts/run_dvfs_split.py            # full run, 120 processes (~2 h)
    python research/scripts/run_dvfs_split.py --dry-run  # checks only, no locks, no runs

If the window is killed hard, run `nvidia-smi -rgc` and `nvidia-smi -rmc` (or reboot) to reset clocks.
"""
import ctypes, hashlib, json, random, sys, time
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
from run_dvfs_cell import run, ROOT, EXE
from run_dvfs_lock import smi, temp, on_ac, reset, lock, Poller, START_MAX_C, ABORT_C

OUT = ROOT / "research/data/raw/2026-10-25-dvfs-split"
EXE_SHA = "1d0c143e702e771d152b5ab0691ce7f264b5e5465f89fc37b0e66b16889b461b"
CLOCKS = {"HH": (2670, 8001), "HL": (2670, 810), "LH": (585, 8001), "LL": (585, 810)}  # (SM, memory) MHz


def preflight(dry):
    pf = {"time": time.strftime("%Y-%m-%d %H:%M:%S"), "admin": bool(ctypes.windll.shell32.IsUserAnAdmin()),
          "ac_power": on_ac(), "temperature_c": temp(),
          "hash_ok": hashlib.sha256(EXE.read_bytes()).hexdigest() == EXE_SHA, "probe": {}, "settable": {}}
    problems = [k for k in ("admin", "ac_power", "hash_ok") if not pf[k]]
    if not dry and not problems:
        for cond, (sm, mem) in CLOCKS.items():
            r = lock(sm, mem, True)
            time.sleep(1.0)
            r["query"] = smi("-q", "-d", "CLOCK,PERFORMANCE")[1]
            r["reset"] = reset()
            pf["probe"][cond] = r
            pf["settable"][cond] = r["lgc"][0] == 0 and r["lmc"][0] == 0
        if not any(pf["settable"].values()):
            problems.append("no clock pair could be set")
    pf["problems"] = problems
    return pf


def main():
    dry = "--dry-run" in sys.argv
    pf = preflight(dry)
    print(json.dumps({k: v for k, v in pf.items() if k != "probe"}, indent=1), flush=True)
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
    conds = [(e, n, c) for e in ["gpu-rows", "gpu-predicate"] for n in [1000, 3000, 10000]
             for c in CLOCKS if pf["settable"][c]]
    total = 5 * len(conds)
    done = sum((OUT / f"block-{b}" / f"{e}-e{n}-{c}" / "run.json").exists() for b in range(5) for e, n, c in conds)
    say(time.strftime("%H:%M:%S"), f"E-SPLIT start, {done}/{total} already done, settable={pf['settable']}")
    try:
        for block in range(5):
            order = conds[:]
            random.Random(5000 + block).shuffle(order)
            for engine, eligible, cond in order:
                dest = OUT / f"block-{block}" / f"{engine}-e{eligible}-{cond}"
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
                info = {"condition": cond, "requested": CLOCKS[cond], "temperature_start_c": temp(),
                        "cooldown_wait_s": waited, "reset_before": reset()}
                info["lock"] = lock(*CLOCKS[cond], True)
                info["query_before"] = smi("-q", "-d", "CLOCK,PERFORMANCE")[1]
                poll = Poller(); poll.start()
                try:
                    rc = run(dest, engine, eligible, 1, 9301 + block)
                finally:
                    poll.stop.set(); poll.join()
                    info["reset_after"] = reset()
                info.update(returncode=rc, thermal_abort=poll.hot, temperature_max_c=max(
                    (int(r[4]) for r in poll.rows if r[4].isdigit()), default=None))
                json.dump(info, open(dest / "lock.json", "w"), indent=1)
                with open(dest / "poll.csv", "w") as f:
                    f.write(Poller.FIELDS + "\n" + "".join(",".join(r) + "\n" for r in poll.rows))
                say(time.strftime("%H:%M:%S"), block, engine, eligible, cond, "rc", rc,
                    f"tmax {info['temperature_max_c']}C", "THERMAL ABORT" if poll.hot else "")
                if poll.hot:
                    say(f"STOP: GPU reached {ABORT_C} C; clocks reset. Let it cool, then rerun to resume."); return
                time.sleep(5)
        say(time.strftime("%H:%M:%S"), "E-SPLIT COMPLETE")
    finally:
        r = reset()
        say("clocks reset:", r["rgc"][0], r["rmc"][0])


if __name__ == "__main__":
    main()
