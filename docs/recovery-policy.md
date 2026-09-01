# Recovery policy

Qenlo self-heals replaceable state. It never invents canonical data, conceals
corruption, or changes benchmark evidence. Every automatic recovery is bounded,
deterministic, and visible through statistics, routing reasons, fallback reasons,
or retained raw attempt data.

| Surface | Automatic recovery | Fail-closed boundary |
| --- | --- | --- |
| Durable collection | Replay a contiguous, checksummed WAL and promote only a complete snapshot whose generation is valid. | Missing generations, invalid checksums, malformed records, and ambiguous publication are errors. The original files remain available for diagnosis. |
| Portable `.qn` | If the target is absent, validate a complete sibling `.qn.pending`, atomically promote it, sync the directory, and set `recovered_interrupted_write`. | A corrupt pending file is preserved and rejected. An existing target is authoritative and is never overwritten. |
| Derived indexes | Discard and rebuild an index when its recorded generation does not match the canonical store. | Canonical vectors and metadata are never reconstructed from an index. |
| GPU execution | Mark a lost or invalid GPU device unhealthy. `Automatic` mode may route later work to the exact CPU backend with an explicit reason. | A required-GPU request returns an error; it never silently changes backend. |
| SDK loading | Prefer the package's platform library, then an explicitly configured library path, and return a typed load error with searched locations. | No unbounded retry and no download or execution of an unverified binary. |
| Embedding models | Resume into a temporary file, verify the declared digest and license metadata, then publish atomically while retaining the last verified model. | A missing or mismatched digest never replaces a working model. Models remain optional and outside the database core. |
| Browser and repair tools | Reopen safely after interruption, rebuild derived views, and create a verified backup before an explicit repair. | Repair is never automatic for canonical corruption and never runs without a recoverable source. |
| CI and benchmarks | Retry transient infrastructure setup within a small fixed limit and retain every attempt with environment and version metadata. | Correctness failures, recall misses, fallbacks, and performance regressions are not retried away or cherry-picked. |

## Operator contract

Recovery changes must be idempotent: running recovery twice yields the same
canonical result. Publication uses write, flush, file sync, atomic rename, then
directory sync where the platform supports it. When the final durability status
cannot be established, Qenlo returns a commit-uncertain error and requires reopen
to resolve which validated generation became authoritative.

Backups and exports are accepted as recovery sources only after the same shape,
record, normalization, generation, and checksum validation applied to normal
opens. The safest available copy wins by explicit generation rules, never by file
modification time.
