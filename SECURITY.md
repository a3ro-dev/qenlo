# security

Qenlo assumes a trusted local application and a directory untrusted users cannot
modify. it is not a security boundary between tenants or processes. metadata
filters select rows; they do not authenticate callers.

## current protections and their limits

vectors are checked for dimension, finite values, and nonzero norm. snapshot
headers, shape, flags, generation, and checksums are checked during load. the
load/write budget limits accepted dataset shape; it does not guarantee allocator
success or cap process RSS.

CRC32 detects accidental corruption. it is not a signature or MAC. someone with
directory write access can rewrite both data and checksums, delete `HEAD`, or
replace lock paths. legacy snapshots without a watermark remain accepted for
compatibility. protect the whole directory with OS permissions; do not open
attacker-controlled files or paths as trusted collections.

files are unencrypted. vectors, IDs, user IDs, and timestamps are on disk.
tombstones retain vector and metadata bytes, so deletion is not secure erasure.
the previous canonical snapshot may remain too. backups, filesystem snapshots,
swap, and telemetry exports need their own access controls.

the file lock coordinates cooperating Qenlo handles. do not remove or replace its
file while a collection is open. it does not defend against a hostile process.
network-filesystem locking and durability have not been established by local
tests. Windows has no full directory-sync power-loss guarantee here.

## telemetry boundary

default library spans contain operation/backend context, not vectors,
credentials, raw user IDs, timestamps, or full predicates. the library installs
no global subscriber. applications can still expose data by logging inputs,
results, debug representations, errors, paths, or custom fields. audit those
separately. returned error strings can contain record IDs and are not inherently
safe to export.

the optional benchmark OTLP setup is host-owned, with a bounded trace queue and
request timeouts. collector failure may lose telemetry but must not change search
results. keep collector credentials outside repositories and configure them in
the host, never in vector metadata.

## reporting a vulnerability

do not attach real vectors, credentials, user data, or collection files to a
public issue. use the repository's private vulnerability-reporting channel if it
is enabled. otherwise ask the maintainer for a private contact using a minimal,
non-sensitive description; do not publish exploit details while waiting.

there is no promised security response SLA or maintained release matrix yet.
fixes should include a synthetic regression test and identify affected versions
and storage assumptions.
