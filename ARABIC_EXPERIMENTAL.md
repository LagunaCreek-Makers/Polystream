# Arabic optional language validation — 2026-10-25

Arabic Kareem medium voice, pinned Piper voice revision 5041a7a49ae5b3a0f30e6837769eb03fdbb231bd, persistent Piper 1.8 CPU worker with bundled Tashkeel diacritization; shared Whisper Base recognition and Qwen3 4B Q4_K_M translation. Source: the 100.702-second title-page development clip. Four threads per inference engine and per voice ONNX session; Tashkeel has its own runtime session and is not included in the voice-session thread cap.

Full replay: 38/38 delivered audio clauses, zero untranslated markers, 38 aligned English/Arabic SRT cues. Median/p95/max first-queued-PCM delay: 10.64/23.70/23.98 seconds. Final drain: 25.04 seconds. No cutoff/retry or unmeasured input in this completed attempt. Physical audio devices/acoustic latency were not tested. This is not sustained Arabic capacity or native-speaker accuracy validation.

## Quality correction evidence

The full replay predates final narrative-wording and mixed-script corrections. Inspection found a financial-account word in narrative context and an initial targeted test omitted the expected Nephi spelling. A later short test emitted a Cyrillic Russian number in an otherwise Arabic sentence. Preserve all three targeted attempts instead of treating their earlier name-only checks as semantic approval.

Final guidance distinguishes historical narrative (سجل / رواية) from financial حساب, retains actual financial senses, and specifies Arabic script and numeric values. Final adapter also rejects introduced Cyrillic characters in Arabic translations unless they were explicitly present in source text; a rejected clause goes through the existing Untranslated English fallback. This narrow check does not certify language or detect every mistranslation. It is regression-tested against the observed bad output and preserves explicitly quoted source Cyrillic.

Six final targeted text/speech cases completed with nonempty PCM and passed name/number assertions. Examples include نافي, كتاب مورمون, the meeting time التاسعة, and لا تغيّر العدد خمسة عشر. A separate six-case Hindi text/speech retest passed after the related narrative-wording change. No claim is made that the entire speech replay was repeated with those final prompt/guard changes.

Official Arabic terminology reference: https://www.churchofjesuschrist.org/study/scriptures/bofm/bofm-title?lang=ara . Title-page material is development data, not held-out evaluation. Broad translation, pronunciation and native-speaker review remain pending.

## Installation and remaining limits

Install-Arabic-Experimental.cmd uses the existing isolated Piper 1.8 environment and pinned/checksummed optional models-ar.json. Other voice runtimes are preserved; Arabic remains a choice among a maximum of eight outputs, not a ninth simultaneous route. Its voice-region badge represents the Jordanian Kareem voice, not an assertion that Arabic belongs to one country. Right-to-left text shaping and native UI visual review remain pending; SRTs retain logical-order UTF-8 Arabic.

Pinned repository README metadata declares MIT, while the Kareem model card says dataset license See URL and links AliMokhammad/arabicttstrain. That training repository did not establish clear dataset terms in this review. Keep this provenance unresolved; do not infer model/dataset redistribution clearance solely from repository metadata or the user's noncommercial intent. Optional local evaluation is separate from a cleared default voice distribution pack.

Full raw evidence: workspace language-rollout/arabic-first-replay. All targeted failures and final checks are retained alongside this report. Native speakers, held-out Church passages, RTL UI review and a longer multi-output rehearsal remain next validation work.
