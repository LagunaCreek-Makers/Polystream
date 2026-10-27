# Long speech replay — October 23, 2026

Input: the user's Book of Mormon Traditional Title Page English MP3, decoded locally
to PCM16 mono 16 kHz. Duration: **100.702 seconds**. Source SHA-256:
`60e989e9464f5e42354645c0270f42abb1207ad8a589fdc8e271c1332da4f4e4`.

Tested with the installed CUDA engines on the RTX 5060 Ti, using the religious
translation profile and no application translation-cache hits.

| Result | Spanish | German |
|---|---:|---:|
| Session result | Stopped: translation/speech queue full | Completed |
| Completed clauses | 26 | 38 |
| Wall-clock duration | 82.13 s before stop | 104.15 s |
| Last completed clause's source end | 72.92 s | 100.16 s |
| Median first PCM after clause end | 2.49 s | 2.14 s |
| Maximum first PCM after clause end | 5.98 s | 4.28 s |
| Mean recognition time | 97 ms | 89 ms |
| Mean translation time | 188 ms | 175 ms |

German completed playback about 3.45 seconds after the source recording ended.
Spanish failed at 82 seconds with `Local translation/speech cannot keep up`;
the remaining source audio was not translated. It must not be counted as a passed
full-recording test. Delay grew to roughly six seconds after a completed clause.

## What was exercised

The new `examples/replay_session.rs` diagnostic feeds decoded audio at real-time
speed through the production `Segmenter::live(0.015)`, `LocalEngines`,
`SessionControl`, `StreamingResampler`, and `PlaybackBuffer`. Recognition and
translation/speech use separate workers and two-item bounded queues. It uses the
app's 15-second recognition and 10-second speech staleness checks, four-second
maximum clauses, 800 ms playback capacity, and two-second playback stall check.
Trailing silence lets the production pause detector flush the last clause.

Playback is drained in 20 ms steps to simulate an audio device, with the rendered
PCM saved for local listening. This is a production-component replay harness;
it does not drive the GUI, microphone callback, physical speakers, or the app's
capture-block channel. It does not validate semantic accuracy or word coverage.
The standard Windows hardware checks are recorded separately in `WINDOWS_VALIDATION.md`.
The harness was formatted, built, and passed Clippy with warnings denied.

This is one run per language. A build/lint check overlapped the early portion of the
Spanish run, so these are functional stress-test observations, not a controlled
hardware benchmark. The checks finished before the Spanish failure.

## Interpretation

GPU recognition and translation remain fast. The observed backlog is downstream,
in the combined speech-generation/playback path. Repeated Piper startup and the
time required to speak each translated fragment are candidates for profiling.
This test does not isolate their individual contributions.

The next work should profile TTS startup and playback waits, improve sentence
segmentation, and rerun this recording repeatedly in both languages. Increasing
queue capacity alone would allow more delay rather than establishing real-time
performance. Native-speaker review is still needed for meaning and pronunciation.

## Repeating the test

With FFmpeg available, decode without overwriting an existing file:

```powershell
ffmpeg -n -i INPUT.mp3 -ac 1 -ar 16000 -f s16le INPUT.pcm
cargo build --locked --example replay_session
python scripts/start_offline.py --config offline.cuda.json --device cuda
```

In a separate terminal, run each language against a new output directory:

```powershell
target/debug/examples/replay_session.exe offline.cuda.json INPUT.pcm es NEW_ES_OUTPUT
target/debug/examples/replay_session.exe offline.cuda.json INPUT.pcm de NEW_DE_OUTPUT
```

Outputs include per-clause WAVs, `clauses.jsonl`, `playback.pcm` (mono PCM16, 16 kHz),
and `summary.json`. Keep generated recordings/transcripts outside version control.
The aggregate statistics for this run are in
`evaluation/reports/windows-2026-10-23/long-speech.json`.
