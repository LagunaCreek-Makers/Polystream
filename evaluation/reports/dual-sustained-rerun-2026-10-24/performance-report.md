# Sustained simultaneous translation validation

State: **failed**. Settings: 4 CPU threads per inference engine, 1 GPU translation slot(s).

| Attempt | Language | Completed phrases | Median / p95 / max delay (s) |
|---|---|---:|---|
| full-soundtrack | Spanish | incomplete | 6.87 / 71.41 / 98.26 |
| full-soundtrack | German | incomplete | 6.22 / 64.97 / 89.77 |

All timing comes from observed software replay. Unmeasured/cancelled sections are not extrapolated. Subtitle timestamps use the original source timeline. Physical devices, native-speaker accuracy and crash/power-loss recovery are not established by this test.
The cutoff is 600 seconds. A full-soundtrack cutoff triggers one speech-only retry with caption-marked music excluded while preserving the original wall-clock breaks. System/GPU resource logs are whole-system samples, including other workloads.

## Terminal interruption

Stopped with `Spanish speech synthesis failed: Output device stopped consuming audio; restart the session`. This was a paced file replay, so the message does not establish a physical-device fault. Root cause remains unconfirmed.

Last observed source input: 1852.10 seconds (30.87 of 45 minutes). No successful backlog drain was measured. The remainder and undelivered pending phrases are unvalidated. No 600-second cutoff occurred and no speech-only retry was triggered. Engines were cleaned up by the supervisor. Earlier failed attempt remains separate.

Subtitle counts and prefix alignment: {"English_2026-10-24.srt": {"cues": 638, "timestamps_match_english_prefix": true}, "German_2026-10-24.srt": {"cues": 608, "timestamps_match_english_prefix": true}, "Spanish_2026-10-24.srt": {"cues": 606, "timestamps_match_english_prefix": true}}. Counts differ because the attempt ended with work pending. Metrics contain 606 Spanish and 607 German first-PCM events; these can differ from the last periodic progress snapshot.
