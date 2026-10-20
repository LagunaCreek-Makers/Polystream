# General Conference evaluation results — v0.5.0

Collected text from April 2017 through October 2026. Runtime inference used local Qwen3-4B Q4_K_M through llama.cpp b11739, four CPU threads and no GPU offload.

## Corpus coverage

- 20 conference indexes, 718 linked entries.
- 717 trilingual text entries: 2,151 pages fetched, parsed and hash-verified.
- One video-only entry without a page transcript explicitly excluded.
- 21,172 candidate paragraph triplets; 84,688 possible directional cases.
- Alignment is unreviewed; full-corpus model inference has not been completed.

## Real-model pilot

One preselected talk per conference, one short paragraph per talk, four language directions: 80 attempted cases, 78 completed and two 30-second deadline failures. The general translation profile was used. All 20 conferences have completed cases. This is a small targeted sample, not a random or exhaustive accuracy estimate.

| Direction | Completed | chrF2 overlap | Median translation | p95 among successes |
|---|---:|---:|---:|---:|
| de-es | 18 | 53.46 | 12.70 s | 29.30 s |
| en-de | 20 | 53.03 | 12.83 s | 21.77 s |
| en-es | 20 | 64.58 | 14.78 s | 20.25 s |
| es-de | 20 | 52.30 | 13.63 s | 24.63 s |

Both failures were German → Spanish. Their IDs, errors and official references are retained in `conference-pilot-failures.json`. Latency statistics exclude failed cases and must be read with those failures. The production deadline censors slow results.

chrF2 is reference-character overlap, not a percentage of correct translations or doctrine. The exact SacreBLEU signature is retained in the JSON report. Native-speaker semantic review has not occurred. Background collection/build activity overlapped portions of this CPU pilot, so timings are illustrative rather than a controlled hardware benchmark.

## Religious terminology update

The supplied config now selects the Latter-day Saint profile. An eight-case targeted check (a 2022 terminology regression plus a 2024 development sanity check) completed 8/8 cases. Its reference-overlap score changed from 59.56 to 60.68.

Reference-terminology review flags changed from 8 to 1. Stake-conference wording improved in the targeted cases. One sacrament-related wording remains flagged for review. These are lexical diagnostics, not validated semantic error counts. This targeted tuning check is not independent held-out evidence of general accuracy improvement.

The profile adds directional Church terminology and instructions to preserve source content without adding doctrine or completing quotations. Ordinary literal senses remain permitted. The original general profile remains available, and old configs default to it.

## Performance probes

On four exact repeats, total request wall time was 8.50 s with the application cache disabled and 3.29 s enabled. There were three cache hits, with median cache-hit translation time 0.00516 ms. This measures translation retrieval, not first audible speech.

The cache holds 64 exact completed translations, respects cancellation and separates languages, models, ports and domain profiles. Corpus simulation showed fewer than 1% of source paragraphs repeating within that window. The repeated-workload improvement is not an estimate of full-corpus or live-session speedup.

A single eight-case ordering probe took 28.70 s interleaved and 25.36 s grouped. Outputs were identical, and both evaluated 260 prompt tokens. Default ordering was left unchanged: one short timing trial does not establish a stable throughput advantage.

The standard launcher explicitly requests one translation slot and a 4,096-token context, consistent with the current single speech worker and measured server setup.

## Still required

- Exhaustive inference for the 84,688 candidate cases and reviewed alignment quality.
- Independent bilingual review of meaning, doctrine, names, numbers and negation.
- CUDA and sustained latency tests on the specified Intel/AMD + RTX 4060-or-later systems.
- Actual audio, ASR, microphone/line capture, pronunciation, live segmentation and acoustic delay tests.
- Native Windows execution and current-release Windows build validation.

This release changes code/configuration, not model weights. It contains source, provenance and aggregate reports; full talks, generated translations, native engines, weights and compiled application binaries are excluded. See `../GENERAL_CONFERENCE.md` for exact preparation, resumption, scoring and reproduction commands.
