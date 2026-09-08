#!/usr/bin/env python3
"""Check release-sensitive documentation against package metadata."""

from __future__ import annotations

import importlib.util
import json
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    cargo = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    version = cargo["workspace"]["package"]["version"]
    py_version = tomllib.loads(
        (ROOT / "sdk/python/pyproject.toml").read_text(encoding="utf-8")
    )["project"]["version"]
    npm_version = json.loads(
        (ROOT / "sdk/typescript/package.json").read_text(encoding="utf-8")
    )["version"]
    docs = {
        path.relative_to(ROOT).as_posix(): path.read_text(encoding="utf-8")
        for path in (ROOT / "docs").rglob("*.md")
    }

    assert npm_version == version
    assert py_version == version.replace("-alpha.", "a")
    assert f'qenlo = "{version}"' in docs["docs/sdks/rust.md"]
    assert f'qenlo=={py_version}' in docs["docs/sdks/python.md"]
    assert "match_record.score" not in docs["docs/sdks/rust.md"]
    assert "match_record.distance" in docs["docs/sdks/rust.md"]
    assert "@a3ro.dev/qenlo@alpha" in docs["docs/sdks/typescript.md"]
    assert "Rust (supported)" in docs["docs/feature-matrix.md"]
    assert "Python (supported)" in docs["docs/feature-matrix.md"]
    assert "TypeScript (supported)" in docs["docs/feature-matrix.md"]
    for language in ("Go", "Kotlin/JVM", "Swift"):
        assert f"{language} (preview)" in docs["docs/feature-matrix.md"]
    assert '.package(url: "https://github.com/a3ro-dev/qenlo.git"' not in docs["docs/sdks/swift.md"]
    sdk_readmes = {
        path.parent.name: path.read_text(encoding="utf-8")
        for path in (ROOT / "sdk").glob("*/README.md")
    }
    assert f'qenlo=={py_version}' in sdk_readmes["python"]
    assert "@a3ro.dev/qenlo@alpha" in sdk_readmes["typescript"]
    assert "mavenCentral()" not in sdk_readmes["kotlin"]
    assert '.package(url: "https://github.com/a3ro-dev/qenlo.git"' not in sdk_readmes["apple"]
    for text in sdk_readmes.values():
        assert "Dual-licensed" not in text

    spec = importlib.util.spec_from_file_location(
        "build_llms_txt", ROOT / "scripts/build_llms_txt.py"
    )
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    expected_index = module.build_llms_txt()
    expected_full = module.build_llms_full_txt(ROOT)
    for path in (ROOT / "llms.txt", ROOT / "docs/llms.txt", ROOT / ".well-known/llms.txt"):
        assert path.read_text(encoding="utf-8") == expected_index, f"stale {path}"
    for path in (ROOT / "llms-full.txt", ROOT / "docs/llms-full.txt"):
        assert path.read_text(encoding="utf-8") == expected_full, f"stale {path}"

    print(f"documentation matches {version}; supported and preview tiers are explicit")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
