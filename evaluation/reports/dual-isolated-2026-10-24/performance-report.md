# Sustained simultaneous translation validation

State: **completed**. Settings: 4 CPU threads per inference engine, 1 GPU translation slot(s).

| Attempt | Language | Completed phrases | Median / p95 / max delay (s) |
|---|---|---:|---|
| full-soundtrack | Spanish | 890 | 6.07 / 98.64 / 125.45 |
| full-soundtrack | German | 890 | 5.95 / 91.42 / 115.46 |

full-soundtrack: source 45.00 min; wall 45.10 min; drain 5.77 s; accepted/completed source phrases 890; excluded music 0.00 s.
Saved SRT cue counts: {"English_2026-10-24.srt": 890, "German_2026-10-24.srt": 890, "Spanish_2026-10-24.srt": 890}.

All timing comes from observed software replay. Unmeasured/cancelled sections are not extrapolated. Subtitle timestamps use the original source timeline. Physical devices, native-speaker accuracy and crash/power-loss recovery are not established by this test.
The cutoff is 600 seconds. A full-soundtrack cutoff triggers one speech-only retry with caption-marked music excluded while preserving the original wall-clock breaks. System/GPU resource logs are whole-system samples, including other workloads.
