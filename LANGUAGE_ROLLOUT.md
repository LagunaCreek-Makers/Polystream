# Language rollout — 2026-10-25

Status is separate for voice compatibility, mechanical replay and linguistic quality. A completed replay is not a quality pass. No physical speaker paths or native-speaker review are established by these tests.

| Target | Current status |
|---|---|
| Spanish / German | Existing working routes; prior sustained tests retained |
| Mandarin | Optional experimental route; targeted name test passed; Huayan dataset license Unknown and native review open |
| French | Optional speech route mechanically passed; broad translation/pronunciation review open |
| Cantonese | Deferred: distinct spoken language; no compatible installed voice backend |
| Japanese | Optional experimental Piper 1.8 route; 38/38 shared-input replay and targeted name/speech cases passed mechanically. Hi-Fi-Captain dataset CC BY-NC-SA 4.0; native/broad quality review pending |
| Russian | Denis voice works; optional experimental route; 38/38 corrected replay and 6/6 targeted name/speech regressions passed; broad/native quality review pending |
| Arabic | Optional local experimental Kareem / Piper 1.8 route: 38/38 mechanical replay and six final targeted name/number/speech checks passed. Dataset provenance, RTL UI, sustained and native review pending |
| Hindi | Optional experimental Pratham / persistent Piper 1.8 route. 38/38 speech replay and six targeted name/speech cases passed mechanically; native/semantic/sustained review pending. CC BY-NC-SA 4.0 dataset notice retained |
| Basque | Antton voice works; current Qwen3 4B translations fail quality (mixed-script/nonsensical output). UI remains unavailable |
| Italian | Optional experimental Serena / Piper 1.8 route; 38/38 shared-input replay and targeted speech cases passed mechanically. Native/broad quality review pending |
| Tibetan / Afrikaans / Hawaiian / Maori | No compatible voice in the reviewed pinned Piper catalog; revisit voice/backend options |

Pinned catalog: rhasspy/piper-voices revision 5041a7a49ae5b3a0f30e6837769eb03fdbb231bd. Keep upstream model cards beside weights. Russian Denis dataset card says CC0; Basque Antton and Italian Serena dataset cards say CC BY 4.0. These are dataset declarations, not an independent complete licensing audit.

New optional packs import only the selected voice, preserving active ASR/translation models, ports, threads and GPU choice. Existing user configurations are never silently replaced. Russian installer: Install-Russian.cmd. Basque weights and manifests are retained for development and its launcher explains the quality blocker. Italian now uses Install-Italian.cmd; Japanese uses Install-Japanese-Experimental.cmd, both with an isolated Piper 1.8 environment.

## Evidence and source policy

Workspace language-rollout/russian-basque-italian preserves the failed initial attempt. Italian direct probe: process exit 3221226505, zero PCM; stderr reports a multi-codepoint phoneme key. Russian and Basque direct probes produced PCM successfully. The subsequent russian-basque-six-stream run measures pipeline behavior even where Basque content fails quality; it must not be described as six validated languages.

Official Russian terminology: https://www.churchofjesuschrist.org/study/scriptures/bofm/bofm-title?lang=rus (Книга Мормона, Нефий / Нефия, Мормон, листы). Italian references: https://www.churchofjesuschrist.org/study/scriptures/bofm?lang=ita (Libro di Mormon, Nefi, tavole). Basque Church-specific equivalents have not been verified; do not invent a glossary.

Title-page input is development material, not held-out evaluation. Preserve failed SRTs and timing gaps. Native reviewers and broader matched official text/audio remain necessary, following evaluation/CHURCH_TERMINOLOGY.md. The future interchangeable-engine phase remains deferred; these findings inform it without silently starting a backend migration.

## Completed checkpoint

Corrected Russian replay: 38/38, aligned SRTs, median/p95/max 5.58/9.13/9.25s, drain 6.84s. Text-only targeted regression 6/6 produced speech and passed limited name checks. Six-stream diagnostic FAILED on incomplete translation at about 77s observed input; no completed drain and remaining output unmeasured. Preserve this failure; do not label it hardware capacity or language-quality success. All 45 Rust tests, Clippy all targets with warnings denied, and desktop build passed. See evaluation/reports/language-rollout-2026-10-25.

## Italian/Japanese checkpoint — later 2026-10-25

The older native Piper failure is preserved; it is resolved for these optional routes by separate Piper 1.8 Python workers. Default Spanish/German/French/Russian/Mandarin native runtime is unchanged. The initial modern per-phrase process baseline completed 38/38 each but built 145.53s Italian / 136.14s Japanese maximum lag and drained 144.58s after input ended.

The final path reuses one model/phonemizer per language, uses explicit four-thread ONNX CPU sessions, framed bounded PCM responses, UTF-8, cancellation/reset and worker process cleanup on engine drop. The same 100.702s shared-input replay completed 38/38 each with aligned SRTs: Italian median/p95/max 3.40/5.47/6.85s; Japanese 4.71/11.44/11.46s. Overall drain 11.50s; no cutoff/retry or missing input in these completed attempts. Both persistence and thread settings changed, so this does not isolate one causal factor. Physical acoustic latency, sustained operation and native quality remain untested.

Some Japanese names ignored prompt guidance. Final source-scoped corrections normalize only observed Nephi/Lamanite variants when explicitly present in English input under Church context, using official Japanese title-page names. They do not complete missing text. Eight final-adapter short text/speech cases passed. The full speech replay predates that last name-normalization edit; keep that distinction. See evaluation/reports/italian-japanese-2026-10-25.

Installers pin runtime dependencies and voice hashes. Python with venv/pip is required (tested interpreter: 3.12); installer --python can select it. Modern runtime is GPL-3.0; package notices remain with dependencies. Japanese noncommercial dataset voice stays optional/experimental. No production/native-language certification implied. The generic future AI workflow profiles phase remains deferred.

## User distribution intent — 2026-10-25

The user clarified that PolyStream is entirely noncommercial, open source and shared. Treat noncommercial voice/model candidates as eligible for evaluation under this intended use; an NC label alone is not a reason to skip a language. Review the exact checkpoint, dataset and runtime terms before redistribution, preserve upstream attribution/license notices and identify component-specific conditions separately. Do not assume a dataset label alone establishes the weights' complete licensing status. Unknown licensing remains an unresolved question, not an automatic permission grant.

The application currently declares MIT in Cargo.toml. This clarification records intended use; it does not change the source license or relicense third-party models/runtime components. Do not apply a blanket noncommercial restriction to the source code without an explicit licensing decision.

Revisit Hindi Pratham next: its noncommercial dataset label no longer blocks evaluation based solely on project intent. Japanese remains experimental because sustained/native quality review is pending, not merely because its dataset is noncommercial. Keep optional packs and their notices identifiable while preparing reproducible installation/sharing.

## Hindi and eight-stream checkpoint — 2026-10-25

Hindi installer, UI/native label/India flag, Devanagari installed-font fallback and optional voice import are integrated. The native-process baseline completed 38/38 but median/p95/max delay was 20.82/43.57/43.85s, drain 42.91s. Persistent Piper 1.8 completed 38/38, aligned English/Hindi SRT timestamps, median/p95/max 5.31/9.28/9.60s and drain 4.05s. Six targeted text/speech cases passed limited name assertions with nonempty audio. Cold worker startup is included in those text tests. This is mechanical validation, not general translation approval: omitted Nephi in one full-replay clause and account-as-financial-account wording require semantic review. No physical acoustic latency or sustained Hindi validation. Font coverage is added; full Devanagari shaping/visual review is not claimed.

The first actual eight-language attempt used es,de,fr,zh-Hans,ru,it,ja,hi and FAILED at approximately 48s observed input when a translation reached 1024 output tokens without a normal stop. Only 12–14 first-PCM clauses per language were measured; source starts extend to 29.76–33.96s. No completed drain measurement; do not claim eight-language capacity or extrapolate through the missing rest. Existing messages did not identify the failing language; new errors now include target and finish_reason. Dashboard calibration records the failed set instead of presenting an eight-stream prediction as measured. Next: isolate the incomplete-generation request before further sustained eight-stream testing; preserve the failed evidence. Do not infer resource exhaustion or expand token limits blindly.

Reports: evaluation/reports/hindi-eight-2026-10-25. Existing 48 Rust tests, Clippy and desktop build passed before the diagnostic edit; targeted error regression and final Clippy/build verify the diagnostic change. User noncommercial sharing intent is recorded; retain model/dataset licenses and attribution. Install-Hindi.cmd uses the separate pinned modern runtime, leaving existing native voices unchanged.

## 2026-10-25 clause-local translation recovery

An incomplete translation is a model response, not necessarily a dead server. Previously `?` propagated it through the output worker to the shared session failure handler, stopping all routes. Now any translation-request failure is clause-local unless cancellation is already active. It writes a quoted `"Untranslated: <original source text>"` target SRT/live-caption cue, a durable `failures.jsonl` record (language, clause, source interval, reason, null delay), increments untranslated counts and completes only that route's handling of the clause. No fallback audio is synthesized and no PCM timing is invented. Other routes and later clauses continue; buffered source is released only after every route handles it. This intentionally treats the durable untranslated marker as a handled clause, rather than retaining failed audio indefinitely.

Plot curves break at failed source intervals; isolated successful points remain visible, including beside gaps. Trailing gaps remain on the time axis. Untranslated clauses are excluded from delay statistics. Cancellation, ASR failure, transcript/metrics storage failure, synthesis/playback failure and panics still stop safely; this change does not claim fault tolerance for every subsystem. There is no unbounded retry loop or blind quotation completion.

The repeated eight-language 100.702s development replay completed with gaps: 303 audio deliveries out of 304 language-clause opportunities, plus one Hindi untranslated marker at clause 13 (source 33.96–38.16s). Later Hindi clauses resumed; all other seven languages delivered 38/38. English and all eight target SRT cue timestamps/counts aligned at 38 each. Final drain 26.18s. Delivered output maximum delay 28.22s; timing excludes acoustic/device latency. Native-language accuracy and sustained eight-stream capacity remain unvalidated. Both the previous aborted attempt and this completed-with-gaps attempt are preserved separately.

49 Rust tests, Clippy all targets with warnings denied, and the desktop build passed. An injected two-language test verifies consecutive length/HTTP failures, unread notification channels, durable English markers, zero fallback audio and source-buffer cleanup. Reports and blank-gap plot: evaluation/reports/fault-tolerance-2026-10-25.

## Arabic checkpoint — 2026-10-25

Arabic local optional route and installer are integrated. Mechanical replay 38/38, aligned SRTs; median/p95/max 10.64/23.70/23.98s, drain 25.04s. Final targeted tests correct narrative financial-account ambiguity and a mixed-script number; the full replay predates those edits. Six final Arabic name/number/speech checks and six related Hindi narrative checks passed. Mixed-script Arabic errors now use the untranslated fallback instead of being spoken. Native/RTL UI and dataset provenance remain unresolved. See ARABIC_EXPERIMENTAL.md and evaluation/reports/arabic-2026-10-25.

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
