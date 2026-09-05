# Feature matrix

This page describes Qenlo's current code and evidence. It does not infer competitor features or mobile support from desktop builds.

## Deployment profiles

| Capability | Embedded profile | Accelerated profile | Current evidence |
| --- | --- | --- | --- |
| Canonical Rust records, tombstones, generations | included | included | Rust storage/core tests |
| Exact CPU cosine search | included | included | x86 Windows and Linux cloud runs; ARM target build only |
| Portable WGPU exact search | optional | optional | Vulkan on RTX 2000 Ada, A40, RTX 4050/4090; Metal and DX12 need current release validation |
| USearch HNSW | excluded by default | optional feature | selected reference runs; approximate recall must be measured |
| PyTorch tensor index | excluded | optional Python extra | CPU and CUDA tested; MPS API exists but has no current hardware result |
| Durable `.qn` snapshots and WAL | included | included | create/open/recovery/corruption tests |
| Rich diagnostics and browser | excluded | optional packages | desktop source and tests |
| Network telemetry | never automatic | separate optional service | core and SDKs make no network request |

## SDK and packaging status

| Surface | Binding status | Distribution status |
| --- | --- | --- |
| Rust | native API | source crate; release publication pending |
| Python | native ABI, bulk float32 input, optional `TorchIndex` | wheel validation pending final release CI |
| TypeScript | native ABI | package validation pending final release CI |
| Go | cgo/native ABI | package validation pending final release CI |
| Kotlin/JVM | JNA/native ABI | JVM tests exist; this is not Android packaging evidence |
| Swift | C ABI wrapper | Apple device and simulator artifacts require separate CI validation |
| Android | bridge/tester source | physical-device and release-package evidence missing |
| iOS | wrapper/tester source | signing, simulator, and physical-device evidence missing |

## Search semantics

| Property | Exact CPU/WGPU | USearch | `TorchIndex` |
| --- | --- | --- | --- |
| Canonical source of truth | yes | no, derived | no, immutable derived snapshot |
| Exhaustive over eligible rows | yes | no | yes for snapshot rows |
| Distance | cosine over normalized FP32 storage | cosine, approximate traversal | matrix product converted to cosine distance |
| Tie order | distance then unsigned ID | final output sorted distance then ID | distance then unsigned ID |
| Mutation behavior | canonical generation; WGPU may update resident state | rebuild derived graph | reject or rebuild stale snapshot |
| Correctness claim | exhaustive; checked against FP64 oracle | report recall | checked against the same oracle |

## Measured small-collection result

The September 5, 2026 Runpod campaign contains 182 retained rows: 131 completed, 42 unavailable, seven failed, and two invalid-harness rows. On one RTX 4090/Vulkan host, exact WGPU P95 was 0.897 ms for 100k by 768, batch one, `k=1`, and 0.896 ms for batch eight, `k=64`, at 10% eligibility. PyTorch CUDA was faster in both cells. The revised lane-minimum selector won five and lost seven of 12 qualified frozen-baseline pairs.

These numbers select no mobile default. See [the performance report](../research/artifacts/runpod-small-2026-09-05/report/performance-report.md) and the paper for timing boundaries and limitations.
