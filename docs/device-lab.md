# Qenlo device lab

The same native suite is packaged for Linux, Windows, macOS, Android, and iOS. It tests canonical semantics before timing hardware, retains every result locally, and uploads only after an explicit action. The desktop packages use wgpu's Vulkan, DX12, or Metal implementation, so one binary covers NVIDIA, AMD, and Intel adapters supported by that operating system.

## Desktop

Run the quick suite:

```text
qenlo-lab run --profile quick --output qenlo-run.json
```

On Windows, double-clicking `qenlo-lab.exe` starts the quick suite, shows four
progress stages and the complete cell summary, saves `qenlo-lab-run.json` beside
the executable, and waits for Enter before closing. Command-line arguments retain
the non-interactive behavior used by automation.

Use `full` for the 100k × 384 matrix and `soak` for sustained batches. Select a GPU by setting `WGPU_ADAPTER_NAME` to a case-insensitive part of its driver-reported name. Set `WGPU_BACKEND` to `vulkan`, `dx12`, or `metal` to force an API. A requested adapter that is not present fails explicitly.

Submit during the run:

```text
qenlo-lab run --profile full --output qenlo-run.json --endpoint https://lab.example/api/v1/runs --token TOKEN
```

Or submit a retained result later:

```text
qenlo-lab upload --input qenlo-run.json --endpoint https://lab.example/api/v1/runs --token TOKEN
```

## Mobile

The Android application supports arm64 devices and reports `Build.SOC_MANUFACTURER` and `Build.SOC_MODEL` on Android 12+, allowing Snapdragon and MediaTek runs to be separated. The iOS application requires arm64 and Metal and reports the Apple machine identifier and thermal state.

Keep the app foregrounded during a suite. Android configuration changes do not recreate the runner activity. A crash, background kill, network failure, redirect, server rejection, or invalid token never deletes the last completed local report. Submission requires HTTPS, uses system certificate validation, and refuses redirects so a bearer token cannot be forwarded to another host. Tokens are held in memory and are not included in reports.

## Test profiles

| Profile | Corpus | Dimensions | Timed queries | Intended use |
|---|---:|---:|---:|---|
| quick | 10,000 | 384 | 16 | install and compatibility check |
| full | 100,000 | 384 | 64 | comparable device result |
| soak | 100,000 | 384 | 512 | thermal and sustained-throughput observation |

The suite covers invalid inputs, filter semantics, durable reopen, exact CPU search, exact GPU search, true batch-8 GPU execution, IVF-Flat and IVF-SQ8 at a Recall@10 ≥ 0.95 gate, an independent float64 truth calculation, P50/P95/P99 latency, transfer bytes, allocation bytes, dispatch count, routing, fallback, device/API identity, and retained failures. It precomputes truth and runs CPU, required-GPU, and automatic collections sequentially so full/soak profiles do not keep three corpora resident on a phone. Its deterministic clustered corpus makes the IVF cells repeatable; it is a device-comparison workload, not evidence of recall on arbitrary production embeddings.

## Retained device submissions

The [Intel Arc submission](../benchmarks/2026-08-31/device-lab/intel-arc/README.md)
retains two embedded `quick` reports and one `soak` report from an Intel Arc
Vulkan adapter. All 21 cells passed with Recall@10 = 1.0 and no fallback. The
soak result recorded exact GPU P95 of 4,444 µs versus exact CPU P95 of 16,486 µs.
One report was supplied under a “full” label but identifies itself as `quick`;
Qenlo preserves the embedded suite value and does not represent it as full.
These device-lab results do not satisfy the separate 1M × 768 investment gate.

## Telemetry server

Set a random token of at least 24 characters and place the server behind an HTTPS reverse proxy:

```text
QENLO_TELEMETRY_API_KEY=... QENLO_TELEMETRY_BIND=127.0.0.1:8787 QENLO_TELEMETRY_DB=qenlo.sqlite3 qenlo-telemetry
```

Open `/` for the results viewer. `POST /api/v1/runs`, `GET /api/v1/runs`, and `GET /api/v1/runs/{run_id}` require `Authorization: Bearer TOKEN`. The request body is capped at 1 MiB, unknown JSON fields are rejected, run IDs are unique, SQLite uses WAL mode, and database work runs outside the async executor.

Reports exclude vectors, queries, raw filter values, source data, hostnames, serial numbers, IP addresses, and credentials. The collector logs server errors without echoing submitted payloads.

## GitHub-only telemetry inbox

For a zero-server test round, submit retained reports through the repository's
[device lab report form](https://github.com/a3ro-dev/qenlo/issues/new?template=device-lab-report.yml).
Android and iOS provide **Copy report and open GitHub**; desktop testers paste the
contents of `qenlo-lab-run.json`. Reports remain visible and searchable as GitHub
issues. Because the repository is public, the form requires explicit consent.

This path needs a GitHub account and a final paste/submit action. GitHub Pages is
static hosting and cannot accept telemetry POSTs. Fully automatic background
submission still requires the HTTPS collector above; do not embed a repository
token in a tester binary.

## Build outputs

| Output | Intended devices | Installability |
|---|---|---|
| `qenlo-lab-linux-x86_64` | Linux NVIDIA/AMD/Intel | executable archive; normal OS permissions apply |
| `qenlo-lab-windows-x86_64` | Windows NVIDIA/AMD/Intel | executable archive; Authenticode recommended for external distribution |
| `qenlo-lab-macos-arm64` | Apple-silicon Macs | executable archive; Developer ID signing/notarization required to avoid Gatekeeper friction |
| `qenlo-lab-android-arm64` | Snapdragon and MediaTek arm64 phones | installable debug APK plus SHA-256 for controlled sideload testing |
| `qenlo-lab-ios-unsigned-validation` | A-series iPhones | build validation only; cannot be installed until provisioned and signed |
| `qenlo-telemetry-linux-x86_64` | results ingestion/viewer host | deploy behind an HTTPS reverse proxy |

## Distribution status

The `device lab packages` GitHub workflow produces all outputs above with SHA-256 manifests for desktop, Android, and server artifacts. Push a `lab-v*` tag, such as `lab-v0.1.0`, to build the packages and publish them as a GitHub Release automatically. Manual workflow runs build downloadable Actions artifacts without publishing a release. The Apple desktop and iOS builds share one macOS runner to control runner cost.

Production distribution still requires your code-signing identities: Authenticode for Windows, Developer ID/notarization for macOS, an Android release keystore, and Apple Developer provisioning/TestFlight for iOS. Those identities cannot safely be manufactured or committed by this repository. Do not send unsigned production packages as if they were trusted releases.

GitHub-hosted debug APKs may be signed with a different ephemeral debug key on a later workflow run. If Android refuses an update because signatures differ, uninstall the older tester (its retained local report will be removed) or use a stable release keystore. Upload the retained result before uninstalling.
