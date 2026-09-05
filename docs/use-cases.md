# Use cases

Qenlo is an embedded vector store for exact, filtered search over small collections. The strongest current evidence covers 1K--100K vectors on desktop CPU and portable GPU backends.

## Local desktop retrieval

Qenlo fits note search, code search, document retrieval, and application memory when the collection should live beside the application instead of in a separate service. Canonical records remain durable on disk; CPU, WGPU, and optional tensor indexes are execution choices around those records.

## Per-user and per-tenant collections

Use one durable collection per isolation boundary, or filter by user identifier, timestamp, and flags. Filtering occurs before result publication, and exact engines retain the distance-then-ID order. Application authorization must still be enforced outside the database.

## Agent memory

Atomic batches, tombstones, reopen checks, and compound filters support episodic or working-memory stores. A host can restrict a query to one user, time range, or status mask without maintaining a second source of truth.

## Optional desktop acceleration

WGPU is useful when completed-call latency and accelerator allocation fit the deployment budget. PyTorch integration is appropriate when an application already owns a CUDA or MPS runtime; it remains optional derived state, not durable storage. Automatic mode reports the backend it actually used and any fallback.

## Mobile and edge evaluation

The repository contains a C/JNI bridge plus Android and iOS tester source, but release packaging, signing, simulator coverage, and physical-device performance are not verified for this revision. Do not treat the desktop NVIDIA campaign as mobile evidence. Until device runs exist, mobile use is an integration target rather than a supported performance claim.

## Air-gapped applications

The core search and persistence path does not require a hosted service. Telemetry and inspection components are separate, optional packages. Deployments with regulatory or threat-model requirements must independently evaluate encryption, access control, filesystem behavior, and platform durability; Qenlo does not provide those controls itself.
