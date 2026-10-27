# Live gain, volume, mute and level meters

Implemented October 24, 2026; included in the rebuilt debug desktop app.

- App input gain: -24 to +24 dB, default 0 dB, with Unity gain reset.
- Independent output faders: 0–100%, default 100%, and per-language mute.
- RMS bars and peak markers measured after gain/volume adjustment and clamping,
  immediately before capture processing or output sample-format conversion.
- Post-fader clipping counts, separately resettable. Original captured-source
  clipping remains visible separately, because reducing app gain cannot undo clipping
  that already occurred in the microphone/interface.
- Five-millisecond gain ramps reduce fader clicks. Stop remains immediately silent.
- Muted output continues draining its queue and saving transcripts; unmuting does not
  release a backlog of speech that was intentionally muted.

These are software signal controls. They do not change Windows device settings or
the physical microphone preamp. Gain is applied before VAD/recognition; changing it
can affect speech detection. Output volume is applied while consuming audio, so it
also affects speech that was queued before the fader was moved. Faders remain live
while device/language routing is locked during a session.

Meter/fader exchange uses atomics; callback metering adds no locks or allocations.
The pre-existing capture-buffer allocation and playback try-lock remain separate
real-time hardening work. Controls are currently in the prototype layout; the larger
PolyStream-style input/output/performance dashboard redesign is still pending.

## Verification

38 Rust tests pass, including post-gain RMS/clamping/non-finite input handling,
independent volumes/mute restoration, ramps, applying volume to already queued audio,
draining muted playback, Stop silence, and interleaved channel isolation. Clippy
passes for all targets with warnings denied. The debug desktop and replay example
build successfully.

An interactive UI check confirmed input and both language-output controls are
visible, including the second output row. No microphone session was started during
that check. Physical microphone/speaker gain and routing verification remains pending.

## Sustained-test interruption and rerun

The first 45-minute dual replay failed after approximately 21 minutes with a local
speech-audio timeout. Last source input was about 19.28 minutes; last measured delay
was roughly 46 seconds Spanish / 45 seconds German. It did not hit the 600-second
delay cutoff and is not counted as a completed test.

Resource/progress logs show a roughly 100-second observation gap. Whole-system used
RAM rose to about 124 GiB, but the machine has about 256 GiB; that was NOT near
capacity. An earlier progress message incorrectly described near-capacity RAM and
was corrected. The interruption's cause remains unconfirmed. Synthesis failures now
identify the language. The 15-second read-idle timeout was not simply increased.

Original evidence and partial plot/report remain in:
`C:\Users\Paul Lott\Documents\New project\dual-development\sustained-45min`

A fresh 45-minute run is in:
`C:\Users\Paul Lott\Documents\New project\dual-development\sustained-45min-rerun`

Keep these attempts separate. The rerun retains 4 CPU threads per inference engine,
one translation slot, and the 600-second cutoff with one music-excluded retry.

### Rerun result — terminal failure

The fresh attempt also failed: Spanish playback queue reported no consumer progress,
after 1852.10 seconds of observed source input (30.87/45 minutes). This was a paced
file replay; it does not establish a physical output-device failure. Root cause is
unconfirmed. No further automatic retry was started; monitor deleted.

First-PCM delay median/p95/max: Spanish 6.87/71.41/98.26 seconds (606 events),
German 6.22/64.97/89.77 seconds (607 events). The 600-second cutoff was not reached,
so the music-excluded fallback was not triggered. Backlog drain did not complete.
English/Spanish/German SRT counts: 638/606/608; saved target cue timestamps match
English prefixes. Text can be saved before first PCM, explaining differing counts.
The final 14.13 minutes and pending output remain unvalidated. Plot leaves those
sections unmeasured. Timing excludes physical acoustic latency.

Saved report: evaluation/reports/dual-sustained-rerun-2026-10-24/performance-report.md.
Next reliability work should investigate the paced consumer stall before treating
simultaneous long sessions as validated. UI redesign remains pending.
