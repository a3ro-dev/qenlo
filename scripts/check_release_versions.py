#!/usr/bin/env python3
"""Fail if Qenlo package versions do not match an sdk-vX.Y.Z tag."""

from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def to_pep440(version: str) -> str:
    v = re.sub(r"-alpha\.?(\d+)", r"a\1", version)
    v = re.sub(r"-beta\.?(\d+)", r"b\1", v)
    v = re.sub(r"-rc\.?(\d+)", r"rc\1", v)
    return v


def main(tag: str) -> int:
    match = re.fullmatch(r"sdk-v(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)", tag)
    if not match:
        raise SystemExit("tag must have the form sdk-vX.Y.Z")
    expected = match.group(1)
    py_expected = {expected, to_pep440(expected)}
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    python = tomllib.loads((ROOT / "sdk/python/pyproject.toml").read_text(encoding="utf-8"))
    npm = json.loads((ROOT / "sdk/typescript/package.json").read_text(encoding="utf-8"))
    kotlin = (ROOT / "sdk/kotlin/build.gradle.kts").read_text(encoding="utf-8")

    py_val = python["project"]["version"]
    if py_val not in py_expected:
        raise SystemExit(f"Python version {py_val} does not match expected {expected} (or {to_pep440(expected)})")

    values = {
        "Cargo workspace": workspace["workspace"]["package"]["version"],
        "npm": npm["version"],
        "Kotlin": re.search(r'^version = "([^"]+)"$', kotlin, re.MULTILINE).group(1),
    }
    mismatches = {name: value for name, value in values.items() if value != expected}
    if mismatches:
        detail = ", ".join(f"{name}={value}" for name, value in mismatches.items())
        raise SystemExit(f"release version {expected} does not match: {detail}")
    print(f"all package manifests match {expected} (Python: {py_val})")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: check_release_versions.py sdk-vX.Y.Z")
    raise SystemExit(main(sys.argv[1]))
