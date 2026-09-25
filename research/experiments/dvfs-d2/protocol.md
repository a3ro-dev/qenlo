# D2 protocol — API and batch replication of the D1 heater effect

Written 2026-09-25 before any D2 data, while D1 was running (D1 results not yet inspected).
Same host/binary/data/heater as D1. Runs after D1 completes.

Conditions: 3 blocks, order shuffled per block with seed 2000+block, order seed 9101+block.
 (a) API replication: WGPU_BACKEND=dx12; engines {gpu-rows, gpu-predicate} x E {1000, 3000} x heater {off,on}, B=1.
 (b) Batch: Vulkan; gpu-rows x E {1000, 3000} x B=16 x heater {off,on}; cpu x E {1000, 3000} x B=16 x heater {off,on}.
Predictions: (a) as D1 P1-P3 — rows heater/off P50 ratio < 0.9 at both E under DX12; predicate ratio >= 0.95.
 (b) exploratory (no directional prediction registered for B=16).
Refutation for (a): rows ratio >= 0.95 under DX12 at both E => effect is Vulkan-specific.
