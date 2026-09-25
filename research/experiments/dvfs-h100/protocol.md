# H100 protocol — datacenter-GPU generality check (user-approved, ~USD 2-3)

Written 2026-09-25 before any H100 data. One H100 SXM pod (Linux, Vulkan via EGL ICD). Same binary source,
dataset, heater (500 iters, 1 ms), B=1, k=10, 200 warmups, 3 x 5000 queries as D1.
Conditions: engine {gpu-rows, gpu-predicate} x E {1000, 3000, 10000} x heater {off, on}; 3 blocks; order shuffled
with seed 3000+block; order seed 9201+block. Telemetry 50 ms nvidia-smi.
Predictions (D1 P1-P3 transferred): rows heater on/off P50 ratio < 0.9 at E=1000 and 3000; predicate ratio >= 0.95;
rows ratio < predicate ratio. Also descriptive: median SM clock / P-state for rows vs predicate, heater off.
Refutation: rows ratio >= 0.95 at both E=1000 and 3000 => inversion not present on this datacenter GPU
(a legitimate boundary result, reported as such).
