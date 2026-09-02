# Package Publishing & Registries

Automated release and publishing pipeline for Qenlo across public package managers.

## Release Process

1. **Version Bump**: All manifests (`Cargo.toml`, `pyproject.toml`, `package.json`, `build.gradle.kts`) are updated in lockstep.
2. **Version Checker**: `python scripts/check_release_versions.py sdk-v<tag>` validates that versions match canonical SemVer / PEP 440 formats.
3. **Release Tag**: Pushing a git tag `sdk-v*` triggers the multi-platform CI build pipeline.
4. **Publishing**: The publication workflow publishes pre-built artifacts:
   * **PyPI**: Precompiled binary wheels (`manylinux`, `macosx`, `win_amd64`)
   * **npm**: `@a3ro-dev/qenlo` with bundled native shared objects
   * **crates.io**: `qenlo` and `qenlo-core` crates
