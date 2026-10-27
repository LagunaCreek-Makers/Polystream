# Qwen3-8B sustained eight-language comparison

Same 900-second original soundtrack including music, Whisper base, eight language routes, four CPU threads per engine and one translation slot. This is one run per model on the same machine, not a repeated controlled performance study. Whole-system load can differ. Timing is source phrase start to first queued PCM; physical/acoustic latency is excluded.

| Language | 4B median / p95 / max (s) | 8B median / p95 / max (s) | 4B / 8B untranslated |
|---|---|---|---|
| es | 11.60 / 29.81 / 41.43 | 9.71 / 31.00 / 33.52 | 1 / 1 |
| de | 13.19 / 29.24 / 37.90 | 12.22 / 28.37 / 35.14 | 1 / 1 |
| zh-Hans | 7.46 / 24.97 / 37.69 | 9.98 / 29.16 / 37.78 | 1 / 1 |
| fr | 17.67 / 31.85 / 39.83 | 17.37 / 34.00 / 36.22 | 1 / 1 |
| ja | 6.69 / 26.54 / 40.02 | 12.67 / 39.29 / 45.35 | 1 / 1 |
| ru | 23.06 / 37.17 / 44.57 | 27.09 / 44.40 / 47.42 | 1 / 1 |
| ar | 57.09 / 77.71 / 84.96 | 70.40 / 91.91 / 96.64 | 10 / 3 |
| hi | 5.05 / 13.58 / 19.44 | 5.66 / 16.57 / 19.13 | 4 / 1 |

8B completion: 2398 audio deliveries plus 10 untranslated markers out of 2,408 opportunities. Each route handled all 301 source clauses, and all nine SRTs have identical cue timestamps/counts. No cutoff, retry, engine failure or unmeasured input section. Final drain 77.83s versus 63.01s with 4B.

Eight 8B markers were the shared symbol-only music clause. Two additional Arabic translations introduced Cyrillic (Christopher Sim; closer to our homes and hearts). Hindi had no further incomplete outputs in this run. The 4B run had eight music markers, nine additional Arabic rejections and three additional Hindi incomplete outputs. Successful timing statistics exclude failed clauses; delay curves have gaps at them. Completion with fewer gaps is not translation-accuracy certification.

Qwen3-8B is not uniformly faster: Arabic and several other distributions increased. Its Arabic maximum reached 96.64s versus 84.96s with 4B. Native-language meaning errors identified in development controls remain open, so neither production accuracy nor a default replacement is approved by this test. Recognition remains unchanged; source-name errors persist.

Resources are whole-system counters, not application-only memory. Sampled GPU memory peak 8,259 MiB for 8B versus 6,753 MiB for 4B. CPU/GPU averages do not identify a bottleneck or show that a hardware upgrade would remove these delays.

8B resource observations:
```json
{
  "system_cpu_percent": {
    "mean": 38.74072861718018,
    "max": 79.30029154518951
  },
  "system_used_ram_bytes": {
    "mean": 52434847583.64206,
    "max": 54450262016
  },
  "gpu_percent": {
    "mean": 44.064876957494405,
    "max": 93.0
  },
  "gpu_used_mib": {
    "mean": 7749.404921700224,
    "max": 8259.0
  }
}
```

The Arabic doctrinal-foundation correction was developed after this sustained run. The 15-minute evidence therefore predates that source-scoped prompt change. Separate targeted runtime checks cover the change, not a new 15-minute replay.

Source verification: https://www.churchofjesuschrist.org/study/manual/general-handbook?lang=eng and https://www.churchofjesuschrist.org/study/manual/general-handbook?lang=ara provide the paired section heading. Matching October 2026 Arabic Gong page was title-only; Hindi retrieval failed, so neither was treated as an aligned translated talk. All these examined sources/cases are development material, not held-out evaluation.

## Implemented source-scoped Arabic term correction

The paired official General Handbook heading establishes doctrinal foundation = الأساس العقائدي. Six direct 8B comparisons before and after the targeted prompt showed all three matching phrases corrected, including the previous mythological substitution, with three unrelated controls unchanged. The provider now adds that fact only for English-to-Arabic, Latter-day Saint domain requests containing the two whole words doctrinal foundation. It does not rewrite model output, substitute expected transcripts or introduce missing source content. Other languages, general-domain requests, building foundations and mythological foundations receive no additional hint.

Actual post-change Rust translation-plus-synthesis checks: the three positive phrases produced the verified term and non-empty speech with both 4B and 8B. All six 8B cases completed. Five of six 4B cases completed; the unrelated mythological control was rejected by the pre-existing Cyrillic guard. That negative control receives no new hint, and the failure is preserved rather than counted as passed. No claim of all-controls accuracy is made.

52 Rust tests, Clippy all targets with warnings denied, and desktop/offline_benchmark builds passed. The test suite explicitly guards source/target/domain scope and whole-word boundaries. The 15-minute 8B run predates this new correction; targeted tests validate the change separately. Whisper base and Qwen3-4B remain the default models; Qwen3-8B remains an optional development candidate. Native review, broader Arabic/Hindi meaning checks and physical audio validation remain open.
