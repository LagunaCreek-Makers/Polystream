# Meeting translation development plan

Updated: October 24, 2026 (America/Los_Angeles).

The first implementation milestone now includes experimental shared-input Spanish/German output, automatic SRT export, per-language delay traces, and system CPU/RAM meters. See DUAL_OUTPUT_VALIDATION.md for measured short-test results and remaining validation.

## Current implementation status — October 24, 2026

The five-case CPU/GPU comparison retained 4 inference threads and one translation slot; see evaluation/reports/dual-performance-2026-10-24/comparison.md. A 45-minute simultaneous replay is running. Mandarin has a separate experimental local target and voice configuration with a completed mechanical replay; semantic/name corrections, native-speaker review, and production voice licensing remain open. See MANDARIN_EXPERIMENTAL.md. Other new languages remain planned.

## 1. Shared-input Spanish and German feasibility and implementation

Capture and segment one English input stream. Recognize each accepted phrase once,
then distribute its text and source timestamps to separate Spanish and German
translation/synthesis/playback workers. Share the loaded recognition and translation
models rather than loading two full model stacks. Initially schedule translation
requests fairly through the existing single llama.cpp slot; measure before changing
GPU concurrency or context allocations.

Each output needs its own language, device, channel, playback queue, completed-phrase
count, current pending age, and delivery-delay history. Preserve phrase order within
each language. A slower output must not serialize the faster output's synthesis and
playback. Bound all per-language text queues as well as the shared source buffer;
retain source/recovery state until all requested outputs finish that phrase. Surface
failures by language and never silently drop one translation or substitute another
language. Define explicit operator actions for disabling a failed output.

Routing must support separate output devices and distinct channels on a multichannel
device. Use a single interleaved output stream for routes sharing a physical device;
independent streams that write silence into the other route's channel are not an
adequate routing implementation. Reject duplicate device/channel assignments before
capture starts. Start with two spoken outputs; determine practical limits by testing.

Validation sequence:

1. Automated fan-out, ordering, slow-output, cancellation, and failure tests.
2. Replay the existing 100.7-second speech clip into both language outputs together.
3. Run a 30–45-minute sustained speech/rehearsal test, including music-pause controls.
4. Replay the complete 116.41-minute meeting simultaneously into both outputs.
5. Verify physical routing and audible output with the actual meeting equipment.

Record source-to-first-queued-PCM median/p95/max delay, oldest pending source age,
queue sizes, recognition count, translations completed by language, end-of-session
drain, and resource usage. Verify one recognition pass, not two. Preserve original
meeting time during music breaks. Continue the previously authorized long-test policy:
if delay exceeds 600 seconds, cancel that stress attempt and retry with caption-marked
music excluded. A successful software replay does not establish physical latency or
semantic translation accuracy.

The previous sequential tests are the comparison baseline: Spanish median/p95/max
5.78/65.29/123.44 seconds; German 5.70/62.05/114.86 seconds. Both completed 2,251 accepted
phrases with zero pending source. These results do not prove simultaneous capacity.

## 2. Automatic meeting transcripts in SubRip format

The operator selects a project folder before starting the meeting. Save English and
every enabled translation automatically as UTF-8 `.srt`, using the requested pattern:

```text
English_2026-10-24.srt
Spanish_2026-10-24.srt
German_2026-10-24.srt
```

Use local meeting-start date (`YYYY-MM-DD`) consistently, including meetings crossing
midnight. Proposed collision policy: a unique meeting subfolder inside the selected
project, retaining the exact requested filenames and never overwriting a prior meeting.
Show the actual destination before capture starts. Keep project-folder selection
separate from the program/model installation directory.

Use source phrase timestamps relative to meeting start for English and translated
subtitles. Translation processing delays must not shift the subtitle timeline. Preserve
music breaks and silence as timeline gaps. Each file has consecutive cue numbers and
`HH:MM:SS,mmm --> HH:MM:SS,mmm` timestamps. Use the same source phrase identifier to
align language files; no artificial cue for unrecognized silence. Prevent malformed
cue text, reversed timestamps, and duplicate writes.

Write English when recognition completes and each target when translation completes,
independently of audio synthesis. Persist through a reliable writer path, not the
bounded best-effort UI notification channel. Flush completed cues incrementally so
normal stopping and inference failure leave usable partial transcripts. Surface disk
full, permission, and disconnected-drive errors prominently; do not claim successful
saving after a write failure. Keep saved transcripts when Stop discards pending audio.
No source audio recording is required for this feature.

Test Unicode (including Chinese, Arabic, and combining characters), timestamp
rounding/hour boundaries, pause/resume, midnight, repeated same-day meetings,
interrupted sessions, and write failure. Validate exported files with a SubRip reader.

## 3. Language rollout — exact requested priority

1. Mandarin Chinese
2. Cantonese Chinese
3. French
4. Japanese
5. Russian
6. Arabic
7. Hindi
8. Basque
9. Italian
10. Tibetan
11. Afrikaans
12. Hawaiian
13. Maori (Māori)

English remains the input language for this rollout. New-language speech recognition
is a separate requirement. Maintain distinct Mandarin and Cantonese routes and script
instructions; do not substitute Mandarin speech for Cantonese. Confirm Arabic variety
and Tibetan variety with the operator before declaring their voice packs validated.

For each language, verify text translation, native pronunciation/synthesis, script and
font display, UTF-8 SRT export, offline licensing/redistribution, installation integrity,
resource use, sustained latency, and native-speaker review. Add pinned/checksummed
optional voice packs to the installer. Show capability states accurately: planned,
text-only experimental, voice installed, or validated. A dropdown entry or model's
claimed language coverage is not completed language support.

The current Piper voice index includes Mandarin and several later-priority languages;
other languages may require a different offline speech backend. Evaluate compatible
backends without silently introducing cloud dependencies. Investigate each language
in priority order; if blocked, record the missing capability and continue independent
work on later languages rather than falsely marking the blocked one complete.

Primary references to re-check while implementing:

- Piper voice index: https://huggingface.co/rhasspy/piper-voices/blob/main/voices.json
- Qwen3 language coverage: https://qwenlm.github.io/blog/qwen3/

## 4. UI and session performance display

Use the user's forthcoming rasterized mockups for the detailed layout. Build data
interfaces and instrumentation first so UI work does not require another pipeline
rewrite. Keep the English input and all language output routes visible together.

Display input/output device and channel, input/output audio levels, output state,
buffer fill, current per-language delay in seconds, and an overlaid delay graph over
meeting time. Show queued-output duration separately from end-to-end delay. Distinguish
the last completed phrase's measured delivery delay from the oldest pending phrase's
current age; while no new output arrives, an unchanged last measurement must not look
like proof that the pipeline is keeping up. Mark pauses, missing measurements, failures,
and language enable/disable boundaries rather than interpolating them away.

Add CPU and memory meters covering both the Rust UI and its inference subprocesses,
with system totals shown separately. Include GPU utilization/VRAM when available on
the current NVIDIA hardware. Sample resource counters off the audio callbacks at a
modest rate. Persist session metrics for later review and graph export, alongside the
transcripts. Keep Start, Pause for music, Resume, Finish and drain, and Stop/discard
states clear. Show transcript destination and saving status during the meeting.

## Delivery order

Implement the shared-input fan-out and short simultaneous test first. Add reliable
SRT writing while that pipeline is being introduced, then validate sustained dual
output. Begin language packs in the requested order on that foundation. Integrate the
detailed UI layout when mockups arrive; per-language telemetry is part of the pipeline
work, not deferred until the final layout. Report measured limits and unfinished
capabilities at every milestone.

## Official Church reference and UI direction

The target audience is Church members. Follow evaluation/CHURCH_TERMINOLOGY.md for broad official paired text/media sourcing, context-sensitive religious names, and separate development/held-out evaluation. After the prior tasks, use the user-provided PolyStream dashboard image as the UI design reference, integrating required music, finish/drain, routing and transcript controls.

## Latest development checkpoint

App-level input gain, per-output volume/mute, post-fader RMS/peak meters and clipping counters are implemented; see AUDIO_LEVELS.md. There are 38 passing Rust tests and an interactive UI visibility check; physical microphone/speaker verification remains pending. The targeted Mandarin name correction now passes its 38-phrase replay; see MANDARIN_EXPERIMENTAL.md. The first sustained dual run failed with a synthesis timeout and an unexplained 100-second observation gap; preserve it as failed. A fresh sustained-45min-rerun is running, with the same 4-thread/1-slot settings. System RAM was elevated but not near capacity; the machine has about 256 GiB.

## 2026-10-25 eight-stream capacity checkpoint

Routes now use a vector with a maximum of eight independent outputs. Removing any row preserves remaining device/channel/volume/mute state. Short 1/2/3/4-stream calibration completed with all 38 clauses per language and aligned SRTs. A local hardware/model/settings fingerprint gates the dashboard estimates. Counts 5–8 are low-confidence projections, not measured capacity or steady-state guarantees. Earlier sustained dual replay reached 125 seconds. French pipeline works locally but language quality/native review remains open. Mandarin optional voice licensing remains unresolved; other unavailable voices remain selectable placeholders with an explanatory notice. See evaluation/reports/capacity-2026-10-25.

## Deferred phase: interchangeable AI workflow profiles (2026-10-25)

User-approved backlog item. Begin only after the current development plan is completed; this does not replace or interrupt the language rollout, UI/session work, or sustained capacity and quality validation. Voice gaps may be deferred under the existing user instruction rather than blocking available languages indefinitely.

Provide Settings > AI Workflow with saved profiles, a shared speech-recognition engine, default translation and speech-synthesis engines, and per-language translation/TTS overrides. Keep the current Whisper Base / whisper.cpp -> Qwen3 4B / llama.cpp -> Piper workflow as the baseline. Include installation and compatibility status, CPU/GPU placement with CPU/GPU as the default, advanced parameters and restore defaults. Engine adapters must preserve buffering, cancellation/drain, routing, aligned SRTs and session telemetry.

Candidates to evaluate, not promises of compatibility or availability:
- Recognition: faster-whisper/CTranslate2, specifying both runtime and Whisper model size.
- Translation: Meta NLLB and Cohere North Small Translate. Check checkpoint licenses, language coverage, terminology handling and hardware requirements. North Small Translate is a large MoE; do not assume local 16 GB GPU suitability. External APIs require explicit user choice; preserve local processing by default.
- Synthesis: Qwen3-TTS, CosyVoice2, GPT-SoVITS, Kokoro, Chatterbox, Parler, Coqui/XTTS and OmniVoice. Identify the exact project/checkpoint meant by "Multilingual-TTS" before integration. Check each model/voice license independently, Windows support, streaming, language/script coverage and resource needs.

Add Benchmark this workflow: compare identical input and language combinations, changing one stage at a time against the baseline. Measure recognition, translation, synthesis, first queued PCM delay, backlog growth/drain, CPU/process-tree RAM and GPU/VRAM. Use held-out Church material for terminology/quality review and sustained sessions for capacity; model-specific speed claims do not establish whole-pipeline performance. Label untested profiles and projections clearly. GPU TTS contention with recognition/translation must be measured.

Suggested evaluation sequence after the current plan: baseline, faster-whisper, NLLB, then Kokoro and Qwen3-TTS/CosyVoice. Revisit ordering using actual language gaps and hardware results. No alternative engine is implemented or installed by this planning entry.

## 2026-10-25 further language validation

Russian optional Denis speech route is integrated and remains experimental: corrected 38/38 speech replay and 6/6 targeted name/speech checks passed; native/broad semantic review pending. Basque voice works but Qwen translation quality failed; Italian Serena is incompatible with current Piper. Both remain unavailable. Six-stream diagnostic failed on incomplete translation; preserve partial measurements rather than claiming six-language validation. Cantonese/Japanese and remaining voice/licensing gaps are documented in LANGUAGE_ROLLOUT.md. All 45 Rust tests, Clippy and desktop build passed.

## 2026-10-25 Italian/Japanese optional language milestone

Italian and Japanese now have optional experimental Piper 1.8 routes and installers, leaving the existing native Piper voices intact. Persistent CPU workers reduced short replay backlog significantly: 38/38 per language, aligned SRTs; Italian median/p95/max 3.40/5.47/6.85s, Japanese 4.71/11.44/11.46s. Eight final-adapter terminology/speech cases passed. Japanese name normalization was added after the full replay and validated separately; broad quality, sustained and physical audio review remain pending. Japanese voice dataset CC BY-NC-SA 4.0, not default redistribution. See LANGUAGE_ROLLOUT.md and evaluation/reports/italian-japanese-2026-10-25. 48 Rust tests, 10 launcher tests, Clippy and desktop build passed.

## User distribution intent — 2026-10-25

The user clarified that PolyStream is entirely noncommercial, open source and shared. Treat noncommercial voice/model candidates as eligible for evaluation under this intended use; an NC label alone is not a reason to skip a language. Review the exact checkpoint, dataset and runtime terms before redistribution, preserve upstream attribution/license notices and identify component-specific conditions separately. Do not assume a dataset label alone establishes the weights' complete licensing status. Unknown licensing remains an unresolved question, not an automatic permission grant.

The application currently declares MIT in Cargo.toml. This clarification records intended use; it does not change the source license or relicense third-party models/runtime components. Do not apply a blanket noncommercial restriction to the source code without an explicit licensing decision.

Revisit Hindi Pratham next: its noncommercial dataset label no longer blocks evaluation based solely on project intent. Japanese remains experimental because sustained/native quality review is pending, not merely because its dataset is noncommercial. Keep optional packs and their notices identifiable while preparing reproducible installation/sharing.

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

## 2026-10-25 priority-eight sustained attempt: symbol-only synthesis failure

The planned 900-second Spanish/German/Mandarin/French/Japanese/Russian/Arabic/Hindi test stopped after the last progress snapshot at 56.14 seconds. Japanese's next translated cue was only ♪; a standalone exact-cue reproduction produced no phonemes and an empty synthesis response, exiting the persistent worker. This was a synthesis failure, not a 600-second capacity cutoff. Arabic had one rejected introduced-Cyrillic translation and successfully used the clause-local untranslated marker. Preserve all original evidence; no completed drain or sustained capacity pass is claimed. Saved SRTs match prefixes of the English timeline. Partial statistics and plot are in evaluation/reports/priority-eight-2026-10-25; unmeasured remainder is blank.

Translation validation now rejects output containing no Unicode letters or numbers before synthesis, using the existing durable quoted English untranslated marker and allowing later clauses to continue. It does not invent speech, silently suppress lyrics, or claim arbitrary synthesis errors are recoverable. Unicode letters/numbers in Japanese, Arabic, Hindi, Mandarin and ordinary languages remain accepted. 51 Rust tests, Clippy all targets with warnings denied, and application/replay builds passed. A bounded 70-second eight-language regression is being checked before any new sustained attempt.

The bounded regression completed with gaps: all eight languages handled all 18 source clauses, and all nine SRT files have identical cue counts/timestamps. 135 audio deliveries plus nine untranslated cues cover all 144 opportunities. Each language rejected the exact music-symbol-only clause 13; later clauses resumed (17/18 audio each, Arabic 16/18 plus its separate script rejection at clause 8). Final drain 16.85s, maximum delivered delay 18.76s (Arabic); no safety cutoff. This verifies the reproduced failure recovery only, not the remaining 15-minute sustained input. Regression report/plot: evaluation/reports/priority-eight-2026-10-25/symbol-regression. Owned engines cleaned up at completion.

## 2026-10-25 completed priority-eight 15-minute rehearsal

The fixed full-soundtrack 900-second run completed with gaps and drained in 63.01s. All eight routes handled 301 source clauses; English plus all target SRTs align at 301 cues. 2,390 speech deliveries and 18 untranslated markers out of 2,408 language-clause opportunities. Eight markers were the shared symbol-only music cue; nine additional Arabic requests introduced Cyrillic, and three Hindi requests reached the incomplete-output limit. No engine crash, cutoff or retry. The original failed attempt is preserved separately.

Delivered median/p95/max seconds: es 11.60/29.81/41.43; de 13.19/29.24/37.90; zh-Hans 7.46/24.97/37.69; fr 17.67/31.85/39.83; ja 6.69/26.54/40.02; ru 23.06/37.17/44.57; ar 57.09/77.71/84.96; hi 5.05/13.58/19.44. Timing excludes physical acoustic latency. Untranslated intervals are blank in the plot and excluded from successful-delay statistics.

Saved report, statistics, plot, transcripts and failures: evaluation/reports/priority-eight-2026-10-25/sustained-15min. capacity-profile.json records this as historical sustained evidence, without changing existing UI calibration guards or promising a delay on other hardware/speech. Whole-system counters cannot establish a cause for Arabic's extra delay. Source recognition/name errors and native-language accuracy remain open; completion is not production quality approval. Next targeted work is Arabic/Hindi rejected phrases plus English proper-name recognition, before another longer capacity test or promotion from experimental status.

## 2026-10-25 targeted quality diagnostics

Preserved 12 actual Arabic/Hindi failures and compared three prompt variants: current Arabic/Hindi rejected 9/2, concise 5/0, native-language 2/1. Rejection counts are not accuracy; completed outputs still show meaning changes. No production prompt changed. English name hints corrected Kearon and Soares but regressed correct Oaks spelling, including a full-name-hint retest. Dallin Harris Oaks matches the supplied source captions and was incorrectly framed as a name error earlier; retain it. Five audio/control cases per ASR variant were measured; all owned engines cleaned up. See evaluation/reports/quality-diagnostics-2026-10-25/diagnostic-report.md and raw evidence. Next gate: source-selected terminology, broader semantic tests, and longer-context/larger-model ASR comparison before promotion.

## 2026-10-25 source-selected terms and stronger-context experiment

28 source-selected glossary requests (12 failures +16 controls) avoided original Hindi token exhaustion but failed plates/narrative/number controls; do not ship it. Base long-context recognition and Whisper small.en (16 requests with/without hints) did not reliably improve names; hinted small.en hallucinated added ordinary-speech words. Model remains in workspace only. Defaults unchanged, owned engines cleaned up. Report/raw evidence/limited automatic quality gates: evaluation/reports/quality-context-followup-2026-10-25. Next evaluate a stronger text model and another recognition candidate against the fixed controls before promotion.

## 2026-10-25 stronger-model comparison

Qwen3-8B Q4_K_M: 56/56 direct requests stopped normally with no introduced Cyrillic; 46/46 actual Rust translation-plus-synthesis cases produced audio. 70-second eight-stream replay handled 18 clauses per route, 136 audio deliveries +8 shared music-symbol markers, all nine SRTs aligned, drain22.30s, maximum delivered delay23.89s. Only short shared-load performance measured. Meaning errors remain (organ/name/doctrinal/dinner-plates controls); no accuracy promotion or default switch. Candidate model/config remain in workspace.

Whisper medium.en was rejected as a default candidate: name regressions, repeated marker output on one context clip and a prompted-silence hallucination. Keep base recognition and no global hint. Model revisions/hashes and raw evidence: evaluation/reports/stronger-models-2026-10-25. All owned engines cleaned up. Next candidate gate: targeted meaning checks and 15-minute eight-route Qwen3-8B validation, with native review still pending.

## 2026-10-25 Qwen3-8B sustained result and scoped Arabic fix

The 900-second eight-route 8B test completed with gaps: 2,398 audio deliveries +10 untranslated cues across 2,408 opportunities, all nine SRTs301 aligned, drain77.83s, no cutoff/retry/crash. Eight cues were shared music-only markers; two further Arabic requests introduced Cyrillic; no extra Hindi length failures. Compared with4B's18 markers and63.01s drain, 8B reduces mechanical rejection but does not uniformly reduce latency. Arabic median/p95/max70.40/91.91/96.64s (4B57.09/77.71/84.96). Sampled8B GPU memory peak8259MiB; whole-system counters do not isolate a bottleneck. Defaults unchanged; native accuracy unapproved. Report/plots/SRTs: evaluation/reports/qwen8b-sustained-2026-10-25.

After this run, implemented the official Arabic General Handbook equivalent for doctrinal foundation, appended only for matching whole-word English phrases, Arabic target and LDS domain. Three positive real translation/speech cases passed on both4B/8B. Negative controls unchanged; 8B6/6 mechanical,4B5/6 with one pre-existing mixed-script rejection on unrelated mythological wording. Preserve it.52Rust tests +Clippyalltargets-Dwarnings and builds pass. Longer run predates prompt fix; do not merge their validation claims. Official2026ArabicGong page was title-only and Hindi retrieval failed, recorded as unverified pairs. Next quality gate: source-selected ordinary/religious senses (plates, organ, account), Arabic names and held-out native review; do not blindly promote8B on fewer markers.

## Validation policy — user clarification, 2026-10-26

New/additional languages are tested individually. Correct errors where practical and move past blocked voices or quality issues. Concurrent multi-language load tests are separate capacity validation, not a prerequisite for every language. Run the target-language meaning controls and its actual speech adapter first; preserve unreviewed/failed cases and do not promote mechanical completion as accuracy.

Arabic and Hindi individual sense experiments completed108 direct requests in six separate language/model runs. Candidate hints improved some words but retained/introduced semantic/number errors, so defaults/prompts were not changed. Evidence: evaluation/reports/individual-senses-2026-10-26. Keep their experimental issues recorded; next available-language quality/speech pass is French individually.

## 2026-10-26 pretested models, help and custom adapters

Settings now offers the4B speed baseline,8B best-tested translation candidate (experimental), and small.en/medium.en recognition candidates (experimental/not recommended). Candidate weights are installed in standard local paths on this development machine. Selection saves a new config, preserves voices/ports/domain, resets stock adapter overrides and activates through Apply & restart engines. Defaults are not silently changed. Custom JSON editing saves new files; managed launcher persists the selected path and preserves custom executable choices/argument arrays. Explicit custom-adapter flags allow descriptor files without disabling stock GGML/GGUF checks. Models help includes measured results, per-language recommendations, component caveats and full STT/translation/TTS adapter contracts, configuration steps, preflight and testing/rollback instructions in MODEL_ENGINE_GUIDE.md.

UI reviewed: settings labels, installed8B selection, and scrollable help. Direct-exe config discovery corrected to find project offline.json through executable ancestors. Tests cover preserving voices/ports/original files, missing model rejection, custom launch arguments/ports/model selection and stock-preset restart weights. Custom model architecture support requires a compatible local adapter; no claim that arbitrary checkpoints automatically run. Broader workflow-family adapters and native accuracy promotion remain separate.

## 2026-10-27 named tested workflows
Settings Pretested Models now uses one dropdown with a three-stage model summary and Use this tested configuration action. Stock presets stage tested CPU/GPU, 4 threads, 99 layers, 1 slot and 4096 context settings together, preserving routing/installed voices. Dropdown alone previews; Apply & restart engines activates. UI inspected and tests/build/Clippy pass (54 Rust, 18 Python; one Python skip). Custom voices retain separate validation status.

## Future feature: versioned plug-in modules (user-approved 2026-10-27)

Backlog only; do not begin implementation in place of the current individual-language rollout and quality corrections. Extend the deferred interchangeable AI workflow profiles with a documented, versioned module interface so models and frameworks can be upgraded independently of the application.

Planned scope:

- A manifest declaring module name/version, interface version, processing stage, supported languages, streaming capabilities, CPU/GPU requirements and license/attribution.
- Explicit input/output contracts for audio, text, language codes, source timestamps, clause/request identifiers and structured errors. Define sample rates, channels, encoding and timing semantics.
- Declared model files, dictionaries/glossaries, pronunciation rules, dependencies and configuration schema; validate missing or incompatible requirements before activation.
- Local out-of-process modules supporting Python, Rust or other implementations, with bounded communication, startup/health checks, cancellation, finish/drain, shutdown and crash isolation. Process separation is not a security sandbox; do not imply arbitrary code is safe.
- App-owned capture, routing, retained-source buffers, aligned transcripts and delay/resource telemetry. Structured translation failure must preserve the quoted original English Untranslated cue and plot gaps; do not invent audio or silently discard pending speech.
- Settings discovery/selection, compatibility and installation status, explicit activation, saved workflow profiles, rollback, CPU/GPU defaults and restore-default controls.
- A developer guide, schema/specification, working example module, conformance tests and reproducible per-language quality/performance benchmarks. New modules remain untested/experimental until their evidence supports a stronger label.

Acceptance: demonstrate replacing one stage without changing the other stages or losing session behavior; reject incompatible interface versions and incomplete dependencies; verify cancellation, ordering, bounded queues, failure handling, SRT alignment and telemetry under a failed or stalled module. Keep held-out language review separate from development examples. Third-party distribution terms remain component-specific.

Purpose: let users define their own STT, text-translation, TTS and terminology modules, making PolyStream customizable and easier to upgrade as models/frameworks add capabilities or improve measured performance. Existing compatible adapter configuration is the foundation, not completion of this full plug-in system.
