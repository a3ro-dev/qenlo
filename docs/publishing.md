# Package publishing and registries

Automated release workflows publish Qenlo builds across public package registries.

## Release process

1. Update versions across all package manifests (`Cargo.toml`, `pyproject.toml`, `package.json`, and `build.gradle.kts`) in lockstep.
2. Run `python scripts/check_release_versions.py sdk-v<tag>` to ensure every version string matches canonical SemVer or PEP 440 formats.
3. Push an `sdk-v*` git tag to trigger the multi-platform CI build pipeline.
4. Run the publication workflow to push pre-built packages to their destination registries:
   - PyPI receives precompiled binary wheels (`manylinux`, `macosx`, `win_amd64`).
   - npm receives `@a3ro.dev/qenlo` with bundled native shared objects.
   - crates.io receives the `qenlo` and `qenlo-core` crates.

Rust, Python, and TypeScript are supported release surfaces. Publication is not
complete until the workflow resolves the exact new versions from crates.io,
PyPI, and npm. Go, Kotlin/JVM, and Swift remain preview artifacts; Maven Central
credentials or propagation do not block the supported alpha release.
