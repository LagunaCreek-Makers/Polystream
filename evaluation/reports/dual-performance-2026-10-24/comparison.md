# Simultaneous translation CPU/GPU comparison

Source: 100.702 seconds, shared English recognition → Spanish and German.
Each case reloads and warms the models, disables translation caching, and preserves 4,096 context tokens per GPU slot.

| Case | Threads/engine | GPU slots | Spanish median / p95 / max (s) | German median / p95 / max (s) | Drain (s) |
|---|---:|---:|---|---|---:|
| 1 | 4 | 1 | 5.81 / 9.93 / 9.95 | 5.76 / 7.27 / 7.38 | 8.23 |
| 2 | 8 | 1 | 5.50 / 8.91 / 9.02 | 5.23 / 6.48 / 6.57 | 7.24 |
| 3 | 16 | 1 | 5.61 / 9.24 / 9.39 | 5.19 / 6.74 / 6.85 | 7.17 |
| 4 | 4 | 2 | 5.79 / 11.23 / 11.24 | 5.67 / 7.86 / 8.00 | 9.55 |
| 5 | 4 | 1 | 5.68 / 9.16 / 9.33 | 5.43 / 6.89 / 7.09 | 7.35 |

Repeated 4-thread/1-slot worst-language p95 range: 9.16–9.93 seconds.
Selected for sustained validation: **4 CPU threads per inference engine, 1 GPU translation slot(s)**.
The tested alternatives did not establish a clear improvement beyond the repeated-baseline variation/5% gate. Retain the existing defaults.

## Resource observations

| Case | System CPU mean / sampled max | System RAM sampled max (GiB) | GPU sampled max (MiB) |
|---|---|---:|---:|
| 1 | 25.5% / 68.5% | 65.85 | 6811 |
| 2 | 22.4% / 38.4% | 60.97 | 6307 |
| 3 | 23.0% / 39.4% | 63.84 | 6560 |
| 4 | 31.0% / 92.1% | 71.09 | 6959 |
| 5 | 28.5% / 59.0% | 66.02 | 6480 |

Counters are whole-system/whole-GPU, sampled approximately every two seconds. Other workloads are included and short peaks may be missed. This is not isolated process utilization.
Delay is source phrase start to first queued PCM, excluding physical audio hardware latency. These 100-second tests do not establish long-meeting stability or native-speaker accuracy.
Synthesis and the duration of the spoken translations can limit catch-up even when recognition/translation computation is fast. Additional GPU slots do not accelerate already-generated audio playback.
The earlier status-file-sharing failure is preserved in the sibling thread-slot-benchmark directory and is not counted as a completed matrix case.
