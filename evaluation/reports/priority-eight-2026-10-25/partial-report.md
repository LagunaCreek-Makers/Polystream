# Eight-language rehearsal: partial failure

Japanese synthesis stopped the run on a symbol-only music cue (♪). A standalone reproduction returned no phonemes and an empty synthesis response. Original attempt and diagnostic stderr are preserved.

| Language | Audio clauses | Untranslated | Median / p95 / max delay (s) |
|---|---:|---:|---|
| es | 13 | 0 | 8.41 / 10.64 / 10.64 |
| de | 13 | 0 | 8.20 / 10.85 / 10.85 |
| zh-Hans | 13 | 0 | 6.65 / 7.82 / 7.82 |
| fr | 13 | 0 | 6.63 / 8.10 / 8.10 |
| ja | 13 | 0 | 5.11 / 9.04 / 9.04 |
| ru | 13 | 0 | 6.43 / 9.06 / 9.06 |
| ar | 9 | 1 | 15.63 / 18.21 / 18.21 |
| hi | 13 | 0 | 6.19 / 11.21 / 11.21 |

Last progress snapshot consumed 56.14 of 900 seconds. This is not a completed 15-minute test. No backlog drain was completed or measured; no cutoff or retry occurred. All saved SRT timestamps match prefixes of the English transcript; counts may differ because work was in flight.

Arabic rejected one introduced-Cyrillic translation and continued with an untranslated English subtitle. Japanese later failed during synthesis, which remains fatal. Successful clauses only contribute delay statistics. Pending and cancelled portions have no measured output delay.

Timing is source phrase start to first queued PCM, excluding physical acoustic latency. Resource samples are whole-system CPU/RAM and GPU counters, not application-only memory. These brief samples cannot establish sustained capacity or a hardware bottleneck.

Resource observations:
```json
{
  "system_cpu_percent": {
    "mean": 36.79000407002123,
    "maximum": 78.93420337139749
  },
  "system_used_ram_bytes": {
    "mean": 66042357444.92308,
    "maximum": 66373779456
  },
  "gpu_percent": {
    "mean": 31.884615384615383,
    "maximum": 87.0
  },
  "gpu_used_mib": {
    "mean": 6317.384615384615,
    "maximum": 6556.0
  }
}
```

Subtitle cue counts:
```json
{
  "Arabic_2026-10-25": 10,
  "English_2026-10-25": 14,
  "French_2026-10-25": 14,
  "German_2026-10-25": 13,
  "Hindi_2026-10-25": 13,
  "Japanese_2026-10-25": 14,
  "Mandarin Chinese_2026-10-25": 14,
  "Russian_2026-10-25": 14,
  "Spanish_2026-10-25": 14
}
```