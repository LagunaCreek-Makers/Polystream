# PolyStream dashboard — October 25, 2026

Implemented in src/main.rs, built for the existing Start-Translation-GPU.cmd launcher.
Dark navy theme follows the supplied PolyStream reference. Input card sits left,
output streams right; meeting controls, system counters, source-to-first-PCM delay
numbers/graphs and captions follow below. Window scrolls on smaller displays.

Input gain/meter and independent output volume/mute/meters remain live controls.
Device, channel, language, project and advanced settings lock during a session.
Two output streams are supported; no additional streams are advertised. Mandarin
remains experimental and requires its optional configuration. GPU telemetry and
CPU/memory history graphs are not implemented; system CPU/RAM meters are actual
whole-system samples. Per-language delay histories remain actual session data.

Music break/resume, finish/drain, immediate stop/discard, pending-source recovery,
source capacity warnings, transcript save location, captions and typed test modes
remain available. Advanced configuration/test modes are collapsed by default.
SRT files save automatically into a unique meeting subfolder of the project path;
this prototype uses a path field, not a native folder browser.

Validation: 38 existing Rust tests pass, Clippy all-targets -D warnings passes,
desktop debug build passes. Native window visually inspected with both Spanish
and German rows enabled. No inference or microphone session started for UI QA;
physical audio routing, gain/mute and live running/draining presentation remain
manual acceptance checks. Review visual polish with the user before further work.

The isolated 45-minute replay passed with 890/890 phrases per target, all three
SRTs 890 cues and identical timestamps. Spanish median/p95/max 6.07/98.64/125.45 s;
German 5.95/91.42/115.46 s. Final drain 5.77 s; no cutoff or speech-only retry.
See evaluation/reports/dual-isolated-2026-10-24. Earlier failed attempts remain
failures. One successful isolated run does not prove the causes of those failures.
Timing excludes physical acoustic latency; file replay does not validate devices.

## Styling refinement — October 25

Narrower input column and wider output area, compact horizontal stream cards,
rounded navy fields/cards, blue slider fill/handles, per-output On/Muted buttons,
waveform mark, Settings disclosure and stronger section headings. Installed Segoe
UI and Semibold provide Windows typography with existing fallback fonts preserved.
No operating-system fonts are redistributed.

CPU and memory cards retain up to 240 real samples, collected approximately once
per second while the UI runs; sparklines are 0–100% and reset on app restart.
GPU remains explicitly unavailable. Translation Delay displays the maximum of
last delivered delays among the streams, not current oldest pending age. The
per-language delay/queue details remain below; source-to-first-PCM timing excludes
physical device latency. Idle graph uses an empty grid with no fabricated data.

Settings reveals transcript project path and advanced/test controls, including
speech threshold. Route/language configuration still locks while running; faders
and mute remain live. Duplicate music/finish buttons below the graph were removed;
all meeting actions remain in the session toolbar. Existing launcher uses the
rebuilt debug executable. This is a styling build, not a new inference benchmark.

Validation: Clippy all-targets -D warnings, 38 regression tests, desktop build.
Native visual check covered both streams and Settings disclosure. Physical audio
and running/draining-state checks remain pending. Small windows scroll vertically.

## Requested stream details and memory scope

Input source language and input channel moved into Settings. Add Language now
opens a selection dialog with Apply/Cancel; duplicate target languages cannot be
applied. Existing stream language is shown as a fixed-width flag/title/native-name
area with a drawn kebab options menu for modification. Enabled targets currently
have Spanish/Español (Spain), German/Deutsch (Germany), and Mandarin Chinese/普通话
(China) badges. Flags are illustrative region badges, not exclusive language
coverage. Output channel routing remains beside each output device.

Mute is labeled above a green/gray symbol button; hover text describes its action.
The delay card uses a larger numeric reading without a graph. Delay history is
hidden until a session starts. Live Captions shows a waiting explanation while
empty and continues displaying recognized source text and language-tagged target
text during actual sessions; SRT saving is independent of this on-screen history.

Corrected the RAM measurement: it previously showed whole-system RAM. It now
samples only the current PolyStream process resident memory (sysinfo Process::memory),
with an automatically scaled MiB history. It explicitly EXCLUDES separate Whisper,
translation and speech engine processes and GPU memory; CPU remains whole-system.
Do not interpret the RAM card as total translation-stack memory.

Validation: 38 existing tests pass; final Clippy and desktop build pass. Native
check confirmed dialog opens and Apply adds German, source settings moved off the
input card, process memory around 129 MiB while idle, numeric-only delay card,
and no delay plot before a session. Physical audio/native-language validation not
rerun for these presentation changes.

## Separate Settings window and engine configuration

Header Settings opens a centered floating window rather than expanding Session.
Contains source language, input channel, transcript project path, audio/test modes,
speech threshold, pending capacity and model configuration path. Hardware selector
uses CPU + NVIDIA GPU by default; CPU-only is an explicit alternative. Advanced
engine controls: CPU threads, GPU layers, translation slots and context per slot.
Saved explicit CPU-only choices remain intentional overrides of the default.

Engine changes use Apply & restart engines only when launched by start_offline.py
and with no active session. The launcher receives app exit code 75, cleans up only
its own engine processes, reloads validated engine-settings.json and relaunches.
Saved hardware settings override launcher flags on subsequent launches. Custom
voice/model selections remain in the selected configuration; CPU/GPU executable
paths come from installed offline.json/offline.cuda.json profiles. Recheck routing
after engine restart; current audio/project selections are not persisted across it.
Direct executable launch cannot own/restart engines and explains use of the launcher.

Validation: 38 Rust tests, 10 launcher tests, Clippy and desktop build pass. Tests
cover saved CPU mode overriding CUDA arguments, thread/slot/context argv and invalid
parameter rejection. Native Settings window and advanced controls visually checked.
Real engine reload/CPU-versus-GPU inference was not rerun during UI development.

## Compact output stream rows

Output rows now follow: row number, flag, English/native language names, output
device, volume, mute toggle, output meter, final kebab menu. Channel routing has
moved into the kebab menu; underlying per-channel isolation and duplicate-route
validation remain. Add Language Stream is in the panel header and still opens the
selection dialog. Second-stream removal is available in its Modify language dialog.
The app still supports two outputs. Native inspection confirmed both rows fit and
routing menu opens; Clippy, 38 Rust tests and desktop build pass. This was UI-only
verification, without starting microphone/inference or claiming physical routing QA.

### Stream volume and meter spacing refinement

Tightened language column and capped the device box width, adding a drawn speaker
icon to the left of the slider and live integer percentage on its right. Output
meter now says only Output Level, in the same 13-point label size as Volume/Mute;
input meter retains its existing dBFS detail. Mute behavior unchanged. Native
preview visually checked icon/percentage and plain heading; Clippy/build pass.

### Empty stream lists and removal

Removed microphone gain Reset and the input meter's silence/dBFS suffix. Input
Level now uses the same strong body typography as Microphone gain.

Each output kebab menu now offers Remove stream. Deleting the first of two streams
promotes the second, preserving its language, device, channel, volume, mute and
AudioLevel instance. Deleting the only stream leaves zero streams; Start reports
that an output must be added. Add Language Stream creates the first row when empty.
Deletion is disabled during an active session. Removed slots reset their controls;
remaining streams are preserved. The two-output engine limit remains unchanged.

Validation: 38 library tests and 3 new UI-state regression tests pass; final Clippy
all-targets -D warnings and desktop build pass. New tests cover primary promotion,
secondary removal without touching primary, and empty-state start protection.
Physical audio was not used for these tests. Preview confirmed the simplified
input label; no claim of physical routing validation.

## Language catalog and Mandarin UI integration — October 25

Output selection now lists all registered requested languages, in priority order.
Planned choices are selectable in the dialog and display Not implemented yet, with
Apply disabled. They cannot become active streams or silently use another voice.
Mandarin remains experimental and usable. Selecting Mandarin automatically adds
its installed optional voice to the session configuration; active model paths,
ports, CPU/GPU launch settings and other configured voices are preserved. Missing
optional packs report installation guidance. No downloads occur from the app.
Huayan dataset licensing and native-speaker review remain unresolved; default
redistributable model manifests are unchanged.

Added native-name catalog metadata. Runtime guards still reject unsupported
languages. Cantonese stays separate from Mandarin; backend assessment started in
CANTONESE_DEVELOPMENT.md. Following languages remain French, Japanese, Russian,
Arabic, Hindi, Basque, Italian, Tibetan, Afrikaans, Hawaiian, Māori, plus the older
Hmong Daw and Fijian placeholders.

Validation: 43 Rust tests, Clippy all-targets -D warnings, desktop build. Optional
Mandarin pack's 3 files verified by install_mandarin.py. Config regression verifies
that adding the optional voice does not alter active translation model or ports,
and missing packs fail clearly. Prior 38-phrase Mandarin inference result remains
the evidence; this UI/catalog change did not rerun GPU inference or native review.

## 2026-10-25 eight-stream capacity checkpoint

Routes now use a vector with a maximum of eight independent outputs. Removing any row preserves remaining device/channel/volume/mute state. Short 1/2/3/4-stream calibration completed with all 38 clauses per language and aligned SRTs. A local hardware/model/settings fingerprint gates the dashboard estimates. Counts 5–8 are low-confidence projections, not measured capacity or steady-state guarantees. Earlier sustained dual replay reached 125 seconds. French pipeline works locally but language quality/native review remains open. Mandarin optional voice licensing remains unresolved; other unavailable voices remain selectable placeholders with an explanatory notice. See evaluation/reports/capacity-2026-10-25.

## 2026-10-25 further language validation

Russian optional Denis speech route is integrated and remains experimental: corrected 38/38 speech replay and 6/6 targeted name/speech checks passed; native/broad semantic review pending. Basque voice works but Qwen translation quality failed; Italian Serena is incompatible with current Piper. Both remain unavailable. Six-stream diagnostic failed on incomplete translation; preserve partial measurements rather than claiming six-language validation. Cantonese/Japanese and remaining voice/licensing gaps are documented in LANGUAGE_ROLLOUT.md. All 45 Rust tests, Clippy and desktop build passed.

## 2026-10-25 Italian/Japanese optional language milestone

Italian and Japanese now have optional experimental Piper 1.8 routes and installers, leaving the existing native Piper voices intact. Persistent CPU workers reduced short replay backlog significantly: 38/38 per language, aligned SRTs; Italian median/p95/max 3.40/5.47/6.85s, Japanese 4.71/11.44/11.46s. Eight final-adapter terminology/speech cases passed. Japanese name normalization was added after the full replay and validated separately; broad quality, sustained and physical audio review remain pending. Japanese voice dataset CC BY-NC-SA 4.0, not default redistribution. See LANGUAGE_ROLLOUT.md and evaluation/reports/italian-japanese-2026-10-25. 48 Rust tests, 10 launcher tests, Clippy and desktop build passed.

## 2026-10-25 Hindi and real eight-stream test

Hindi optional persistent Piper route: 38/38 aligned speech replay, median/p95/max 5.31/9.28/9.60s, drain 4.05s; six targeted name/speech cases passed. Native/semantic/sustained review remains pending. First real eight-language replay failed around 48s observed input on incomplete 1024-token generation; only partial output measured, not a capacity pass. Preserve it, display failed-combination feedback and isolate the failing request next. See LANGUAGE_ROLLOUT.md and evaluation/reports/hindi-eight-2026-10-25.

## 2026-10-25 clause-local translation recovery

An incomplete translation is a model response, not necessarily a dead server. Previously `?` propagated it through the output worker to the shared session failure handler, stopping all routes. Now any translation-request failure is clause-local unless cancellation is already active. It writes a quoted `"Untranslated: <original source text>"` target SRT/live-caption cue, a durable `failures.jsonl` record (language, clause, source interval, reason, null delay), increments untranslated counts and completes only that route's handling of the clause. No fallback audio is synthesized and no PCM timing is invented. Other routes and later clauses continue; buffered source is released only after every route handles it. This intentionally treats the durable untranslated marker as a handled clause, rather than retaining failed audio indefinitely.

Plot curves break at failed source intervals; isolated successful points remain visible, including beside gaps. Trailing gaps remain on the time axis. Untranslated clauses are excluded from delay statistics. Cancellation, ASR failure, transcript/metrics storage failure, synthesis/playback failure and panics still stop safely; this change does not claim fault tolerance for every subsystem. There is no unbounded retry loop or blind quotation completion.

The repeated eight-language 100.702s development replay completed with gaps: 303 audio deliveries out of 304 language-clause opportunities, plus one Hindi untranslated marker at clause 13 (source 33.96–38.16s). Later Hindi clauses resumed; all other seven languages delivered 38/38. English and all eight target SRT cue timestamps/counts aligned at 38 each. Final drain 26.18s. Delivered output maximum delay 28.22s; timing excludes acoustic/device latency. Native-language accuracy and sustained eight-stream capacity remain unvalidated. Both the previous aborted attempt and this completed-with-gaps attempt are preserved separately.

49 Rust tests, Clippy all targets with warnings denied, and the desktop build passed. An injected two-language test verifies consecutive length/HTTP failures, unread notification channels, durable English markers, zero fallback audio and source-buffer cleanup. Reports and blank-gap plot: evaluation/reports/fault-tolerance-2026-10-25.

## 2026-10-25 Arabic local evaluation and next rehearsal

Arabic Kareem/Piper 1.8 optional experimental route: full 38/38 replay with aligned SRTs, median/p95/max 10.64/23.70/23.98s and drain 25.04s. Six final Arabic name/number/speech cases passed; a narrow script validator rejects introduced Cyrillic using the existing English fallback. Six Hindi narrative checks also passed. Full replay predates final prompt/script changes; do not call it final broad accuracy validation. RTL/native UI, native-speaker review and dataset provenance remain open. See ARABIC_EXPERIMENTAL.md.

Prepared offline.priority-eight.json for the user priority mix: Spanish, German, Mandarin, French, Japanese, Russian, Arabic, Hindi. It drops Italian only from this test mix and leaves the selectable Italian route intact. Next validation is a longer speaker-length eight-output rehearsal, retaining untranslated intervals and the 600-second cutoff. No sustained run was launched by this checkpoint. Missing-voice Cantonese/Tibetan/Afrikaans/Hawaiian/Maori and Basque translation-quality work remain deferred/open under the existing instruction; generic interchangeable-workflow development remains deferred.

## 2026-10-26 pretested models, help and custom adapters

Settings now offers the4B speed baseline,8B best-tested translation candidate (experimental), and small.en/medium.en recognition candidates (experimental/not recommended). Candidate weights are installed in standard local paths on this development machine. Selection saves a new config, preserves voices/ports/domain, resets stock adapter overrides and activates through Apply & restart engines. Defaults are not silently changed. Custom JSON editing saves new files; managed launcher persists the selected path and preserves custom executable choices/argument arrays. Explicit custom-adapter flags allow descriptor files without disabling stock GGML/GGUF checks. Models help includes measured results, per-language recommendations, component caveats and full STT/translation/TTS adapter contracts, configuration steps, preflight and testing/rollback instructions in MODEL_ENGINE_GUIDE.md.

UI reviewed: settings labels, installed8B selection, and scrollable help. Direct-exe config discovery corrected to find project offline.json through executable ancestors. Tests cover preserving voices/ports/original files, missing model rejection, custom launch arguments/ports/model selection and stock-preset restart weights. Custom model architecture support requires a compatible local adapter; no claim that arbitrary checkpoints automatically run. Broader workflow-family adapters and native accuracy promotion remain separate.

2026-10-27: Pretested Models is now a complete named-workflow dropdown, with STT/translation/TTS summary and tested hardware parameters. Use this tested configuration stages the selection; managed Apply & restart activates it. Previewing alone changes no running engines. UI reviewed.
