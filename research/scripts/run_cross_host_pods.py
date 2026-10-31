"""Run scripts/runpod/cross_host_bench.sh on a few RunPod hosts, fetch results, always delete pods.

python research/scripts/run_cross_host_pods.py <base.tgz> <cand.tgz> <out_dir>

Budget guard: each pod's worst case is price/hr * DEADLINE_MIN / 60, the sum must stay under
BUDGET_USD, and a pod whose returned price exceeds its cap is deleted at once.
"""
import json, subprocess, sys, threading, time
from pathlib import Path

DEADLINE_MIN = 35
BUDGET_USD = 1.00
NO_WINDOW = 0x08000000  # keep ssh/scp from opening console windows on Windows
ENV = '{"NVIDIA_DRIVER_CAPABILITIES":"compute,utility,graphics","NVIDIA_VISIBLE_DEVICES":"all"}'
GPU_IMAGE = "runpod/pytorch:1.0.3-cu1281-torch291-ubuntu2404"
PODS = [  # (name, gpu id or None for CPU, cloud, price cap $/hr)
    ("rtx3090", "NVIDIA GeForce RTX 3090", "COMMUNITY", 0.25),
    ("rtx2000ada", "NVIDIA RTX 2000 Ada Generation", "SECURE", 0.27),
    ("a6000", "NVIDIA RTX A6000", "COMMUNITY", 0.36),
    ("rtx4090", "NVIDIA GeForce RTX 4090", "COMMUNITY", 0.37),
    ("cpu", None, "SECURE", 0.20),
]
base_tgz, cand_tgz, out_root = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
script = Path(__file__).resolve().parents[2] / "scripts/runpod/cross_host_bench.sh"
spent = {"exposure": 0.0}
lock = threading.Lock()


def run(args, timeout=120):
    return subprocess.run(args, capture_output=True, text=True, timeout=timeout, creationflags=NO_WINDOW)


def ssh_args(info):
    return ["-o", "StrictHostKeyChecking=no", "-o", "UserKnownHostsFile=/dev/null", "-o", "ConnectTimeout=15",
            "-i", info["key"]]


def log(name, msg):
    line = f"{time.strftime('%H:%M:%S')} [{name}] {msg}"
    print(line, flush=True)
    with lock, open(out_root / "driver.log", "a") as f:
        f.write(line + "\n")


def pod(name, gpu, cloud, cap):
    out = out_root / name
    out.mkdir(parents=True, exist_ok=True)
    args = ["runpodctl", "pod", "create", "--name", f"qenlo-xh-{name}", "--cloud-type", cloud,
            "--container-disk-in-gb", "30", "--ports", "22/tcp", "-o", "json"]
    if gpu:
        args += ["--gpu-id", gpu, "--image", GPU_IMAGE, "--env", ENV]
        if cloud == "COMMUNITY":
            args += ["--public-ip"]
    else:
        args += ["--compute-type", "CPU", "--image", "runpod/base:1.0.2-ubuntu2404"]
    with lock:
        if spent["exposure"] + cap * DEADLINE_MIN / 60 > BUDGET_USD:
            log(name, "skipped: budget guard")
            return
        spent["exposure"] += cap * DEADLINE_MIN / 60
    created = run(args)
    if created.returncode != 0:
        log(name, f"create failed: {created.stderr.strip()[:300]}")
        return
    info = json.loads(created.stdout)
    pid = info["id"]
    deadline = time.time() + DEADLINE_MIN * 60
    try:
        price = float(info.get("costPerHr") or info.get("adjustedCostPerHr") or 0)
        log(name, f"created {pid} at ${price}/hr")
        (out / "pod.json").write_text(created.stdout)
        if price > cap:
            log(name, f"price {price} over cap {cap}, deleting")
            return
        ssh = None
        while time.time() < deadline - 25 * 60:
            r = run(["runpodctl", "ssh", "info", pid, "-o", "json"])
            try:
                s = json.loads(r.stdout)
                if s.get("ip") and s.get("port") and s.get("ssh_key", {}).get("path"):
                    ssh = {"host": s["ip"], "port": str(s["port"]), "key": s["ssh_key"]["path"]}
                    if run(["ssh", *ssh_args(ssh), "-p", ssh["port"], f"root@{ssh['host']}", "true"], 30).returncode == 0:
                        break
                    ssh = None
            except (json.JSONDecodeError, AttributeError):
                pass
            time.sleep(10)
        if not ssh:
            log(name, "ssh never became ready")
            return
        log(name, f"ssh ready {ssh['host']}:{ssh['port']}")
        target = f"root@{ssh['host']}"
        for local, remote in ((base_tgz, "/workspace/base.tgz"), (cand_tgz, "/workspace/cand.tgz"), (script, "/workspace/run.sh")):
            r = run(["scp", *ssh_args(ssh), "-P", ssh["port"], str(local), f"{target}:{remote}"], 300)
            if r.returncode != 0:
                log(name, f"upload failed {local.name}: {r.stderr[:200]}")
                return
        start = ("mkdir -p /workspace/base /workspace/cand && tar -xzf /workspace/base.tgz -C /workspace/base && "
                 "tar -xzf /workspace/cand.tgz -C /workspace/cand && nohup bash /workspace/run.sh >/dev/null 2>&1 &")
        run(["ssh", *ssh_args(ssh), "-p", ssh["port"], target, start], 60)
        log(name, "started")
        while time.time() < deadline - 4 * 60:
            time.sleep(30)
            r = run(["ssh", *ssh_args(ssh), "-p", ssh["port"], target,
                     "test -f /workspace/out/DONE && echo done; tail -n1 /workspace/out/log.txt"], 30)
            if r.stdout.startswith("done"):
                break
        run(["ssh", *ssh_args(ssh), "-p", ssh["port"], target, "tar -C /workspace -czf /workspace/out.tgz out"], 60)
        r = run(["scp", *ssh_args(ssh), "-P", ssh["port"], f"{target}:/workspace/out.tgz", str(out / "out.tgz")], 300)
        log(name, "fetched" if r.returncode == 0 else f"fetch failed: {r.stderr[:200]}")
    finally:
        r = run(["runpodctl", "pod", "delete", pid])
        log(name, f"deleted {pid} rc={r.returncode}")


out_root.mkdir(parents=True, exist_ok=True)
threads = [threading.Thread(target=pod, args=p) for p in PODS]
for t in threads:
    t.start()
for t in threads:
    t.join()
log("all", f"finished, worst-case exposure ${spent['exposure']:.2f}")
