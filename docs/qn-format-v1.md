# Qenlo portable file format (`.qn`) v1

Status: implemented and covered by round-trip, shape, version, checksum, tombstone,
and non-overwrite tests. Multi-byte values are little-endian. Readers must reject
unknown versions, non-zero reserved bytes, impossible shapes, duplicate IDs,
non-unit/non-finite vectors, and checksum mismatches.

## Layout

| Offset | Size | Field |
| ---: | ---: | --- |
| 0 | 8 | magic bytes `QENLODB\0` |
| 8 | 4 | format version (`1`, `u32`) |
| 12 | 4 | vector dimension (`u32`, non-zero) |
| 16 | 8 | canonical generation (`u64`) |
| 24 | 8 | total row slots including tombstones (`u64`) |
| 32 | 8 | live row count (`u64`) |
| 40 | variable | fixed-width row records |
| EOF - 4 | 4 | CRC32 of every preceding byte (`u32`) |

Each row is `32 + dimension * 4` bytes:

| Relative offset | Size | Field |
| ---: | ---: | --- |
| 0 | 8 | public ID (`u64`, unique and never reused) |
| 8 | 8 | user ID (`u64`) |
| 16 | 8 | timestamp (`i64`) |
| 24 | 1 | live flag (`0` or `1`) |
| 25 | 7 | reserved, all zero |
| 32 | `dimension * 4` | normalized IEEE-754 binary32 vector values |

The exact file length is:

```text
40 + rows * (32 + dimension * 4) + 4
```

## Semantics

- `.qn` is the canonical portable interchange artifact and MIME/file-association target.
- Export is create-only and atomic: Qenlo writes a sibling `.qn.pending`, syncs it,
  renames it, and syncs the parent directory. Existing targets are not overwritten.
- Import self-heals an interrupted export only when the target is absent and the
  complete pending file passes normal shape, record, and checksum validation. The
  recovery is reported in collection statistics. Invalid pending files are preserved.
- Import validates the complete file before returning a mutable in-memory collection.
- Tombstones and generation are retained, so ID non-reuse and deterministic ordering survive transfer.
- Derived CPU/GPU indexes, telemetry, locks, and embedding models are not serialized.
  They are local, replaceable state and rebuild after import.

## Durability boundary

Version 1 is a portable snapshot, not the live write-ahead-log container. Durable
collections continue to use Qenlo's tested directory snapshot/WAL protocol and can
be exported to `.qn` at a committed generation. A future live single-file revision
must define dual-superblock publication, recovery, locking, and torn-write behavior
before it can replace that protocol.
