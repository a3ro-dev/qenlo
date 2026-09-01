# Intel Arc device-lab submission

Source: manually supplied by the device owner on 2026-09-01. The JSON is
retained without changing measured values. The install ID is an opaque lab
identifier, not an account credential.

| Run ID | Embedded suite | Rows | Samples | Treatment |
| --- | --- | ---: | ---: | --- |
| `1788191220761-8960` | quick | 10,000 | 16 | quick evidence |
| `1788191227442-34176` | quick | 10,000 | 16 | second quick evidence; supplied under a “full” label but not represented as full |
| `1788192094423-26860` | soak | 100,000 | 512 | soak evidence |

All 21 cells passed with Recall@10 = 1.0 and no reported fallback. On the soak
run, exact GPU P95 was 4,444 µs versus exact CPU P95 16,486 µs (3.71× lower),
while IVF-Flat P95 was 2,704 µs (6.10× lower). IVF-SQ8 was slower than exact CPU
at P95. These are host-observed device-lab results on one Intel Arc Vulkan
adapter, not the predeclared 1M × 768 research gate.
