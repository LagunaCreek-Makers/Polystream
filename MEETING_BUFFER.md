# Buffered live meetings

Live translation now keeps pending source speech in memory while recognition,
translation and spoken playback work through it in order. A busy speech queue waits
for space instead of stopping the session. Old buffered clauses no longer expire at
the former 10/15-second limits.

## Capacity and meeting format

The default is **60 minutes of pending source audio**, adjustable from 15 to 120
minutes before starting. Audio is retained as mono PCM16 at 16 kHz: about **115 MB**
for a full default buffer, plus working memory. Capacity is allocated as needed.
Processed source audio is released, so this is not a recording of the entire meeting.

This provides room for two 15-minute speakers plus another 30 minutes of exceptional
backlog. Ordinary 10–30-second transitions give playback a chance to catch up without
changing controls. A 5–7-minute musical break can drain several minutes of backlog.
Actual catch-up depends on how much pending translated speech needs to be spoken.

These are capacity assumptions, not a guarantee that translation stays near live.
If output consistently takes longer than input, delay still grows. The app shows
pending source duration, phrase count, and the age of the oldest pending speech,
and highlights backlogs over 30 seconds. The existing playback-buffer measurement
is separate from this source backlog.

## During a meeting

1. Start with the intended microphone, output, and target language selected.
2. Leave capture running through normal speaker transitions and short pauses.
3. Before music starts, click **Pause capture for music**. New audio is excluded
   while pending translations continue playing. Click **Resume speech capture**
   before the next speaker starts. Music exclusion is manual, not automatic.
4. At the end of a talk or meeting, **Finish pending translations** ends capture,
   keeps the partial final phrase, and speaks the remaining backlog. After playback
   finishes, the app returns to Start. Use music pause instead if another speaker
   will start before that backlog has drained.
   The output stream stays open for an additional second after draining to allow
   the audio device to finish its final output buffer.
5. **Stop & discard pending** is immediate cancellation. It clears playback and
   discards source speech that has not finished processing.

At capacity, the app preserves the clause crossing the threshold and already accepted
capture blocks, pauses further capture, displays a capacity warning, and drains the
buffer. This allows a small amount of headroom beyond the nominal capacity. New speech
after that visible pause is not captured; restart after the preserved backlog drains.

## Retention and recovery

Normal buffering writes no audio to disk. RAM contents are released as processing
completes and are discarded when you stop, close the app, or the process exits.
Playback may still contain up to 800 ms of generated speech after its source is
released. This is not protection against a process crash or power failure.

If an engine fails, capture and processing stop visibly and the open session retains
pending source audio. **Save pending source audio for recovery** explicitly exports
that backlog to a new WAV in the Windows temporary directory; its full path appears
in the status. The file remains until you delete it or the OS cleans temporary files.
Copy it elsewhere if it must be retained. Export does not discard the in-memory audio.
Recovery WAVs concatenate pending speech clauses in order; they are not full meeting
recordings and can include part of a phrase already spoken before an error. Automatic
retry/resume from a recovery file is not implemented.

Microphone disconnection, dropped capture blocks, model errors and audio-device
failures are still reported; buffering cannot reconstruct audio the device did not
deliver. Translation correctness remains dependent on recognition and model quality.

## Verification

The new `replay_buffered` example uses the same `MeetingCapture`, `MeetingPipeline`,
source buffer and playback buffer as the desktop app. It replays PCM16 mono 16 kHz
at real-time speed and then drains pending speech. The original `replay_session`
example remains available to reproduce the older two-item queue behavior.

```powershell
cargo build --locked --example replay_buffered
python scripts/start_offline.py --config offline.cuda.json --device cuda
# In another terminal, with a new output directory:
target/debug/examples/replay_buffered.exe offline.cuda.json INPUT.pcm es NEW_OUTPUT
```

This replay saves test audio and event logs explicitly. Ordinary live buffering does
not. Results from the supplied Title Page recording are saved alongside this document
in `evaluation/reports/windows-2026-10-23/buffered-*.json`.

The supplied 100.702-second Title Page recording was replayed at real-time speed:

| Result | Spanish | German |
|---|---:|---:|
| Accepted / recognized / translated / completed clauses | 38 / 38 / 38 / 38 | 38 / 38 / 38 / 38 |
| Playback finished at | 106.73 s | 106.46 s |
| Peak pending source audio | 10.74 s | 8.48 s |
| Peak age of oldest pending speech | 10.90 s | 10.48 s |
| Pending source at completion | 0 | 0 |

The earlier unbuffered Spanish replay stopped at 82 seconds. The new shared pipeline
completed both languages without queue overflow or expired buffered clauses. Playback
was simulated and saved to disk in these tests, not routed through physical speakers.
The GUI Start, music pause, resume and Finish controls were separately exercised on
Windows with a silent physical input. A continuous 30-minute live meeting has not
been run. Unit tests verify 30 minutes of pending speech fits with 30 more minutes
of capacity, limit handling, ordered retention/release, old speech preservation,
music-boundary flushing, and explicit recovery export after an engine error.
