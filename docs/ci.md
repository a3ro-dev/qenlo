# CI and release map

Qenlo uses seven GitHub Actions workflows. A green build does not imply that packages were uploaded to public registries.

| Workflow | Trigger | Purpose | Publishes externally |
| --- | --- | --- | --- |
| `portable correctness` | Every push and pull request | Runs the Rust workspace on Linux, Windows, and macOS; cross-checks Windows ARM64 and research evidence | No |
| `SDK conformance` | SDK, FFI, core, lockfile, or workflow changes; manual dispatch | Tests the native ABI and Python, TypeScript, Go, Kotlin, Swift, docs, formatting, and linting | No |
| `SDK release artifacts` | `sdk-v*` tag or manual dispatch | Builds native libraries, wheels, language bundles, XCFramework, and checksums | GitHub Release only on a tag |
| `Publish SDKs` | Manual dispatch with an existing SDK tag | Verifies version alignment, then publishes independent registry jobs | crates.io, PyPI, npm; Maven only when credentials exist |
| `browser & desktop ci/cd` | Browser-related changes, `v*` tags, or manual dispatch | Tests the browser and builds CLI browser binaries and Tauri compilation checks | No release job currently exists |
| `device lab packages` | `lab-v*` tag or manual dispatch | Builds desktop, Android, Apple, and telemetry lab packages | GitHub Release only on a lab tag |
| `Deploy Static Landing to GitHub Pages` | Landing, docs, or asset changes on `main`; manual dispatch | Deploys the static repository site | GitHub Pages |

## SDK release sequence

1. Merge and verify `portable correctness` and `SDK conformance`.
2. Create an immutable `sdk-vX.Y.Z` tag whose manifest versions agree.
3. Wait for `SDK release artifacts` to create the GitHub Release and checksums.
4. Manually dispatch `Publish SDKs` with that exact tag.
5. Verify each registry independently. A green workflow can still contain an intentional skip, such as Maven Central when credentials are absent.

The registry workflow is deliberately separate because registry versions are immutable. Re-running it is partly idempotent: PyPI uses `skip-existing`, while the Rust commands currently suppress all publish failures with `|| true`. That suppression makes a green Rust job insufficient proof of publication, so crates.io must be checked directly.

## Tag families

- `sdk-v*`: SDK artifact releases.
- `lab-v*`: device-lab releases.
- `v*`: reserved by the browser workflow, although that workflow currently has no GitHub Release publishing job.

## Required release configuration

The `package-registries` GitHub environment gates the registry jobs. Expected secrets are:

- `CARGO_REGISTRY_TOKEN`
- `PYPI_API_TOKEN` or correctly configured PyPI trusted publishing
- `NPM_TOKEN` or correctly configured npm trusted publishing
- `MAVEN_CENTRAL_USERNAME`
- `MAVEN_CENTRAL_PASSWORD`
- `MAVEN_SIGNING_KEY`
- `MAVEN_SIGNING_PASSWORD`

Do not record secret values in logs, artifacts, documentation, or release notes.
