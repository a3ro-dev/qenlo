# Qenlo small-collection performance matrix

Rows retained: 182. Completed: 131. 
P50/P95/P99 are lower medians of per-run completed-call percentiles. Failed and unavailable configurations remain in the CSV and JSON.

## Qualified paired P95 comparisons

| Configuration | Workload | Comparator | Comparator / improved GPU | Outcome |
|---|---|---:|---:|---|
| rtx2000-eu-ro-secure-pilot | p1-r1000-d128-b1-f1 | baseline-gpu | 0.777x | improved GPU slower |
| a40-ca-mtl-secure | c1-r1000-d128-b1-f1 | baseline-gpu | 0.881x | improved GPU slower |
| a40-ca-mtl-secure | c4-r10000-d384-b1-f001 | baseline-gpu | 0.855x | improved GPU slower |
| a40-ca-mtl-secure | c5-r100000-d128-b1-f001 | baseline-gpu | 1.043x | improved GPU faster |
| a40-eu-se-secure | c1-r1000-d128-b1-f1 | baseline-gpu | 1.295x | improved GPU faster |
| a40-eu-se-secure | c4-r10000-d384-b1-f001 | baseline-gpu | 1.096x | improved GPU faster |
| a40-eu-se-secure | c5-r100000-d128-b1-f001 | baseline-gpu | 1.205x | improved GPU faster |
| rtx4090-eu-ro-secure-reference-retry | c1-r1000-d128-b1-f1 | baseline-gpu | 0.840x | improved GPU slower |
| rtx4090-eu-ro-secure-reference-retry | c1-r1000-d128-b1-f1 | chroma | 3.700x | improved GPU faster |
| rtx4090-eu-ro-secure-reference-retry | c2-r1000-d384-b16-f001 | chroma | 25.018x | improved GPU faster |
| rtx4090-eu-ro-secure-reference-retry | c4-r10000-d384-b1-f001 | baseline-gpu | 0.853x | improved GPU slower |
| rtx4090-eu-ro-secure-reference-retry | c4-r10000-d384-b1-f001 | chroma | 35.832x | improved GPU faster |
| rtx4090-eu-ro-secure-reference-retry | c5-r100000-d128-b1-f001 | baseline-gpu | 1.073x | improved GPU faster |
| rtx4090-eu-ro-secure-reference-retry | c5-r100000-d128-b1-f001 | chroma | 305.944x | improved GPU faster |
| rtx4090-eu-ro-secure-deep | d1-r5000-d384-b8-k10-f01 | baseline-gpu | 0.889x | improved GPU slower |
| rtx4090-eu-ro-secure-deep | d1-r5000-d384-b8-k10-f01 | chroma-ef512 | 92.736x | improved GPU faster |
| rtx4090-eu-ro-secure-deep | d3-r50000-d384-b1-k10-compound | baseline-gpu | 0.617x | improved GPU slower |
| rtx4090-eu-ro-secure-deep | d3-r50000-d384-b1-k10-compound | chroma-ef512 | 446.397x | improved GPU faster |
| rtx4090-eu-ro-secure-deep | real-r100000-d384-b16-k10-f001 | chroma-ef512 | 3233.136x | improved GPU faster |

This report does not infer mobile performance from cloud NVIDIA hosts.
