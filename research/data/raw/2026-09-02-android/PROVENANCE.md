# Android device-lab provenance

The author supplied three schema-v1 benchmark JSON objects directly on 2026-09-02. They report app version 0.1.0, target `android-arm64`, Android 16/API 36, Mediatek MT6897 CPU, Mali-G615 MC6 GPU, and Vulkan. The reported run IDs are:

- quick: `1788360526431-12061`, 16 samples/cell, 10,000 rows
- full: `1788360541413-12061`, 64 samples/cell, 100,000 rows
- soak: `1788360620387-12061`, 512 samples/cell, 100,000 rows

All 21 cells reported `passed=true`, recall@10=1, no failures, and no fallback. The processed transcription is `research/data/processed/android_device_lab.csv`. Values are preserved as reported; microseconds were divided by 1,000 for the millisecond columns. `thermal_state` was the string `"0"` and `power_source` was null, so neither is assigned a semantic interpretation. These runs were not independently rerun by the paper-writing campaign and are labeled user-supplied device-lab evidence.
