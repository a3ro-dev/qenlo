#!/usr/bin/env python3
"""Advance the active Qenlo alpha version across release surfaces."""

from __future__ import annotations

import re
import subprocess
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SEMVER = re.compile(r"0\.1\.0-alpha\.(\d+)\Z")


def replace(path: str, old: str, new: str) -> None:
    target = ROOT / path
    original = target.read_bytes()
    count = original.count(old.encode())
    if count == 0:
        raise SystemExit(f"{path}: expected {old!r} to occur")
    target.write_bytes(original.replace(old.encode(), new.encode()))
    print(f"{path}: {count} replacement(s)")


def main(target_version: str) -> None:
    match = SEMVER.fullmatch(target_version)
    if not match:
        raise SystemExit("usage: python scripts/bump_alpha.py 0.1.0-alpha.N")
    current = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))[
        "workspace"
    ]["package"]["version"]
    current_match = SEMVER.fullmatch(current)
    if not current_match or int(match.group(1)) != int(current_match.group(1)) + 1:
        raise SystemExit(f"expected the next alpha after {current}, got {target_version}")
    old_python = current.replace("-alpha.", "a")
    new_python = target_version.replace("-alpha.", "a")
    for path in (
        "Cargo.toml",
        "Cargo.lock",
        "sdk/typescript/package.json",
        "sdk/kotlin/build.gradle.kts",
        "docs/_navbar.md",
        "docs/sdks/rust.md",
    ):
        replace(path, current, target_version)
    for path in (
        "sdk/python/pyproject.toml",
        "sdk/python/uv.lock",
        "sdk/python/README.md",
        "docs/sdks/python.md",
    ):
        replace(path, old_python, new_python)
    subprocess.run([sys.executable, str(ROOT / "scripts/build_llms_txt.py")], check=True)
    # CI fails when research evidence changes without a refreshed inventory.
    subprocess.run(
        [sys.executable, str(ROOT / "paper/scripts/inventory_evidence.py")],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    subprocess.run(
        [sys.executable, str(ROOT / "scripts/check_release_versions.py"), f"sdk-v{target_version}"],
        check=True,
    )
    subprocess.run([sys.executable, str(ROOT / "scripts/check_docs.py")], check=True)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: python scripts/bump_alpha.py 0.1.0-alpha.N")
    main(sys.argv[1])
