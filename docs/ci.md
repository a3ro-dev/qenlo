# CI and release map

Qenlo uses seven GitHub Actions workflows. A passing build does not mean packages were published to public registries.

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
2. Create an immutable `sdk-vX.Y.Z` tag with matching manifest versions.
3. Wait for `SDK release artifacts` to produce the GitHub Release and checksums.
4. Manually dispatch `Publish SDKs` with that exact tag.
5. Check each registry independently. A passing workflow can still contain deliberate skips, such as Maven Central when credentials are absent.

The publication workflow runs separately because published package versions cannot be replaced or deleted. Re-running it is only partly idempotent: PyPI uses `skip-existing`, while the Rust publish step suppresses failures with `|| true`. Because of that suppression, a green Rust job does not prove publication succeeded. Always check crates.io directly.

## Tag families

- `sdk-v*` tags trigger SDK artifact builds and releases.
- `lab-v*` tags trigger device lab package builds.
- `v*` tags are reserved for the browser workflow, which does not currently publish a GitHub Release.

## Required release configuration

The `package-registries` GitHub environment protects the release jobs. You must configure the following secrets in repository settings:

- `CARGO_REGISTRY_TOKEN`
- `PYPI_API_TOKEN` (or configured PyPI trusted publishing)
- `NPM_TOKEN` (or configured npm trusted publishing)
- `MAVEN_CENTRAL_USERNAME`
- `MAVEN_CENTRAL_PASSWORD`
- `MAVEN_SIGNING_KEY`
- `MAVEN_SIGNING_PASSWORD`

Never record secret values in logs, build artifacts, documentation, or release notes.
