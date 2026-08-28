"""Run one benchmark command, preserving logs and Windows process memory evidence.

Usage: python scripts/measure_command.py --output NEW.json -- PROGRAM ARGS...
Working set is host resident memory, not allocator totals or GPU VRAM. Sampling
adds a small external observer cost; the OS peak working set is also recorded.
Use the native benchmark executable directly: Windows Python virtualenv launchers
spawn a descendant and would measure only the launcher, not the Python workload.
"""

import argparse
import ctypes
from ctypes import wintypes
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import time


class MemoryCounters(ctypes.Structure):
    _fields_ = [("cb", wintypes.DWORD), ("PageFaultCount", wintypes.DWORD)] + [
        (name, ctypes.c_size_t) for name in (
            "PeakWorkingSetSize", "WorkingSetSize", "QuotaPeakPagedPoolUsage",
            "QuotaPagedPoolUsage", "QuotaPeakNonPagedPoolUsage", "QuotaNonPagedPoolUsage",
            "PagefileUsage", "PeakPagefileUsage")]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("a command is required")
    if os.name != "nt":
        parser.error("this measurement helper currently supports Windows only")
    log = args.output.with_suffix(".log")
    if args.output.exists() or log.exists():
        parser.error("measurement or log exists; choose a fresh path")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    psapi = ctypes.WinDLL("psapi", use_last_error=True)
    kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    kernel.OpenProcess.restype = wintypes.HANDLE
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    psapi.GetProcessMemoryInfo.argtypes = [wintypes.HANDLE, ctypes.POINTER(MemoryCounters), wintypes.DWORD]
    samples, peak, errors = [], 0, []
    started = time.perf_counter()
    with log.open("x", encoding="utf-8") as output:
        child = subprocess.Popen(command, stdout=output, stderr=subprocess.STDOUT)
        handle = kernel.OpenProcess(0x0400 | 0x0010, False, child.pid)
        try:
            while True:
                counters = MemoryCounters()
                counters.cb = ctypes.sizeof(counters)
                if handle and psapi.GetProcessMemoryInfo(handle, ctypes.byref(counters), counters.cb):
                    peak = max(peak, counters.PeakWorkingSetSize)
                    samples.append([round(time.perf_counter() - started, 3), counters.WorkingSetSize])
                elif not errors:
                    errors.append(f"GetProcessMemoryInfo unavailable: Windows error {ctypes.get_last_error()}")
                code = child.poll()
                if code is not None:
                    break
                time.sleep(0.25)
        finally:
            if handle:
                kernel.CloseHandle(handle)
    executable = Path(command[0])
    report = {
        "command": command, "cwd": str(Path.cwd()), "platform": platform.platform(),
        "exit_code": code, "observed_wall_seconds": time.perf_counter() - started,
        "peak_host_working_set_bytes": peak if samples else None,
        "host_working_set_samples_seconds_bytes": samples, "memory_errors": errors,
        "memory_scope": "child process only, OS working set; excludes GPU and child descendants",
        "observer_interval_seconds": 0.25,
        "environment": {key: os.environ.get(key) for key in ("WGPU_BACKEND", "CC", "CXX", "CARGO_INCREMENTAL")},
        "executable_sha256": hashlib.sha256(executable.read_bytes()).hexdigest() if executable.is_file() else None,
        "git_revision": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
    }
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"exit={code}; peak_host_working_set={report['peak_host_working_set_bytes']}; report={args.output}")
    return code


if __name__ == "__main__":
    sys.exit(main())
