# Qenlo release checklist

The release is one version across Rust, Python, TypeScript, Kotlin, Swift, and
Go. Registry publication is irreversible enough that every check below is a
gate, not a suggestion.

## Access required

- crates.io ownership or a scoped publish token for `qenlo-core` and `qenlo`;
- PyPI ownership for `qenlo`, preferably configured as a GitHub Actions trusted
  publisher;
- npm publisher access to the `@qenlo` organization and an automation token;
- a Maven Central Portal account with the `dev.qenlo` namespace, a user token,
  and an ASCII-armored GPG private key plus password;
- GitHub permission to create signed tags and releases in `a3ro-dev/qenlo`.

Go uses the GitHub repository as its module registry. SwiftPM resolves a GitHub
tag and binary artifact, so neither needs a separate registry password.

## Version gate

1. Set the same version in the workspace, Python, npm, Kotlin, and release
   workflow. Confirm it is absent from every public registry.
2. Run `cargo fmt --all -- --check`, Clippy, the Rust workspace tests, SDK CI,
   package-content inspection, and native ABI conformance on the release commit.
3. Run `cargo package --list -p qenlo-core` and `cargo package --list -p qenlo`.
   Neither archive may contain benchmark corpora, local binaries, or credentials.
4. Build the Apple XCFramework as a release candidate. Run
   `swift package compute-checksum QenloFFI-<version>.xcframework.zip`, insert
   the value in the Swift binary target, commit that manifest, then create the
   final signed tag. Do not move the tag afterward.

## Registry order

Publish dependencies before dependants:

```text
qenlo-core -> qenlo -> native FFI artifacts -> Python/npm/Maven -> SwiftPM -> Go tag
```

For Maven Central, the build uses the Vanniktech plugin and expects these
environment-backed Gradle properties:

```text
ORG_GRADLE_PROJECT_mavenCentralUsername
ORG_GRADLE_PROJECT_mavenCentralPassword
ORG_GRADLE_PROJECT_signingInMemoryKey
ORG_GRADLE_PROJECT_signingInMemoryKeyPassword
```

For CI, store credentials only as environment-scoped repository secrets. Use
PyPI trusted publishing and npm provenance where the registry account supports
them. Never put a token, private key, or generated native library in git.

## Post-publication proof

Create empty consumer projects on Windows, Linux, and macOS. Install from the
public registry, not the repository checkout. Each must create an in-memory
three-dimensional collection, add one record, retrieve it through a filter,
inspect the execution report, and close without leaking a native handle. Record
the commands and public artifact digests in the GitHub release.

The paper artifact is separate from the package release. For ICLR review, use an
anonymous code archive or anonymous repository; the public GitHub URL and this
repository's authorship metadata must not be included in the anonymous bundle.
