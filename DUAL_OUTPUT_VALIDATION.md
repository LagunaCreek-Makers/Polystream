# Shared-input dual output and transcript validation

Implemented October 24, 2026. Dual output remains **experimental** pending a sustained
meeting test and physical device/channel verification.

## Available now

- One live input/recognition pass with independent Spanish and German workers.
- Separate device/channel selection for each output. Shared devices use one stream
  with separate interleaved channels; duplicate routes are rejected.
- Pending source remains retained until every requested language finishes synthesis.
  A processing/device/disk failure stops the session with pending source retained for
  the existing explicit recovery export. Automatic continuation after one language
  fails is not implemented.
- Automatic UTF-8 SRT export of recognized source and each translation, using original
  phrase timing rather than delayed playback timing. Completed cues are flushed and
  synced individually. Stop/discard does not remove existing transcript files.
- User-editable project-folder field. Files are placed in a unique
  `Meeting_YYYY-MM-DD_HH-MM-SS` subfolder with collision suffixes when necessary;
  filenames remain `English_YYYY-MM-DD.srt`, `Spanish_YYYY-MM-DD.srt`, and
  `German_YYYY-MM-DD.srt`. The date uses the computer's local meeting-start date.
- Per-language last delivery delay, current oldest pending age, pending/completed
  counts, and delay traces. The traces omit connections across source gaps over
  15 seconds. `delays.jsonl` saves per-language measurements beside the subtitles.
- Background-sampled system CPU and RAM meters. These are whole-system counters,
  not isolated app/engine counters. GPU/process accounting remains future work.

## How to use

1. Start with `Start-Translation-GPU.cmd` and select Live voice translation.
2. Choose the microphone/line input and English as source.
3. Choose Spanish and its output device/channel.
4. Enable the second output; select German and a different device/channel.
5. Enter the desired transcript project-folder path. Relative paths resolve against
   the program's working directory; use an absolute path for an unambiguous destination.
6. Start. The UI shows the transcript meeting-folder path. Use Pause for music/Resume
   as appropriate. Finish pending translations preserves and drains accepted speech;
   Stop/discard cancels pending audio but keeps already written subtitles.

Detailed mockup-based layout, a graphical project-folder picker, saved routing
preferences, and process/GPU resource counters are not included in this milestone.
Only the existing installed Spanish/German voices are enabled; language rollout stays
in the requested order in DEVELOPMENT_PLAN.md.

## Real GPU replay result

Input: the provided 100.702-second Book of Mormon title-page recording, decoded as
mono PCM16 at 16 kHz. Same MeetingCapture/MeetingPipeline used by the app; two paced
file playback sinks in place of physical devices. Translation cache disabled.
Whisper and Qwen share the installed RTX 5060 Ti. The existing single llama.cpp slot
was retained; duplicate GPU model stacks were not loaded.

| Output | Completed phrases | Median delay | p95 delay | Maximum delay |
|---|---:|---:|---:|---:|
| Spanish | 38/38 | 5.72 s | 11.13 s | 11.28 s |
| German | 38/38 | 5.64 s | 7.40 s | 7.41 s |

All 38 accepted source phrases completed for both outputs. Playback drained at
110.760 seconds, about 10.06 seconds after input ended. There was no ten-minute
cutoff. Delay is source phrase start to first queued PCM; it excludes physical
hardware latency and does not measure semantic quality. Medians in the replay
summary use the upper middle observation for an even-sized sample.

The test deliberately did not consume its capacity-one UI event channel: all three
SRT files still contained 38 cues, with identical source timestamps across languages.
FFmpeg parsed each file successfully. There were 76 persisted delay records, 38 per
target. This verifies transcript/metric saving does not depend on UI notifications.

Evidence on this development machine:

`C:\Users\Paul Lott\Documents\New project\dual-development\results-20261024-090818\replay`

Includes summary.json, subtitle-validation.json, dual-delay.png, output PCM sinks,
and the timestamped folder containing SRT/JSONL files.

## Automated checks

- 33 Rust tests passed serially, including retaining source until both outputs finish,
  interleaved route isolation and Stop behavior, subtitle Unicode/timestamp gaps,
  non-overwriting session folders, existing cancellation, device-stall, and buffer tests.
- Clippy passed for all targets with warnings denied.
- Updated debug desktop executable and replay_dual example built successfully.

Reproduce the software replay with:

```text
cargo run --locked --example replay_dual -- CONFIG PCM16_MONO_16KHZ NEW_OUTPUT_DIR
```

Start/warm the configured local engines first. The example performs one shared replay
into two outputs, rejects delay over 600 seconds, preserves test evidence, and verifies
all accepted source is released after both outputs finish. It does not implement
music-excluded fallback; use a separate long-test supervisor for that policy.

## Remaining validation

The previous two-hour results tested languages sequentially. They do not validate this
new simultaneous path. Next run sustained dual-language speech, then the complete
meeting, monitoring CPU/RAM/VRAM and per-language delays. Confirm actual output routing
on the intended hardware and have native speakers review transcript/audio quality.
The revised desktop layout has compiled but has not yet had an interactive visual QA
pass on the meeting equipment.
