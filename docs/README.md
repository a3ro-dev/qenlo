# Qenlo

Qenlo is an embedded vector store. It runs inside your application, keeps its records in one durable file, applies your metadata filters *before* ranking, and returns exact nearest neighbours. There is no server to run and no network access.

It is a research beta (`0.1.0-beta.2`). The storage format and API are unchanged since alpha.11; performance numbers on this site are tied to the hardware they were measured on.

## Install

| Language | Command |
| --- | --- |
| Rust | `cargo add qenlo` |
| Python | `pip install qenlo==0.1.0b2` |
| TypeScript | `npm install @a3ro.dev/qenlo@beta` |

Go, Kotlin and Swift bindings are previews. See [SDKs](sdks/rust.md).

## Where to go

<div class="card-grid">
<a href="#/quickstart"><strong>Quickstart</strong><span>Create a collection, filter, search, reopen it.</span></a>
<a href="#/concepts"><strong>Core concepts</strong><span>Records, filters, exact search and the execution report.</span></a>
<a href="#/browser"><strong>Collection browser</strong><span>Inspect a collection from the terminal, web or desktop.</span></a>
<a href="#/use-cases"><strong>Use cases</strong><span>Where Qenlo fits, and where it does not.</span></a>
<a href="#/architecture"><strong>Architecture</strong><span>Storage, recovery, derived indexes and routing.</span></a>
<a href="#/trade-offs"><strong>Trade-offs</strong><span>Memory, CPU versus GPU, and when to use something else.</span></a>
</div>

## What is new in beta.2

- Exact CPU search runs certified FP32 at any size and splits large queries across threads. The old 4,096-row cliff is gone.
- Exact GPU search selects top-k with a tiled kernel, 3 to 4.5 times faster on the device at 10k or more eligible rows.
- Results are unchanged: every mode is checked against an FP64 reference.

Full notes are in the [changelog](https://github.com/a3ro-dev/qenlo/blob/main/CHANGELOG.md).

## Research

Qenlo ships with its evidence. The [research paper](../QENLO-RESEARCH-PAPER.pdf ':ignore') studies GPU power states and the latency of light vector-search queries. The [benchmark protocol](benchmark-protocol.md), [measured results](results-2026-08-28.md) and [verification record](verification.md) say exactly what was run, where, and what it does not show.
