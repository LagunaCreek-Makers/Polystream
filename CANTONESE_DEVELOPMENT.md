# Cantonese backend assessment — 2026-10-25

Next priority after the implemented experimental Mandarin route. Cantonese remains
selectable for inspection in the UI but cannot be applied as an active stream.
Language code yue and native name 粵語 are distinct from Mandarin zh-Hans / 普通话.
Traditional Chinese writing alone does not establish Cantonese spoken output.

The official rhasspy/piper-voices catalog fetched via hf download contains 178
voices and no yue or zh_HK entry. Snapshot revision:
5041a7a49ae5b3a0f30e6837769eb03fdbb231bd.
voices.json SHA256: 97ecf06e9f4d7ae7aa06b7b34ef5e1377a36869dd9bf9c1682a5cd695fbea331.
Local snapshot: workspace/language-catalog-research/voices.json.
This finding concerns that catalog only, not every possible Piper-compatible voice.

Primary sources reviewed:
- https://huggingface.co/rhasspy/piper-voices/blob/5041a7a49ae5b3a0f30e6837769eb03fdbb231bd/voices.json
- https://github.com/QwenAudio/CosyVoice (official project; prior FunAudioLLM URL redirects here)
- https://huggingface.co/FunAudioLLM/CosyVoice-300M

CosyVoice is a candidate for a separate local Cantonese speech backend. Its model
documentation advertises Cantonese and a different inference stack from native
Piper. This is an upstream capability claim, not local validation or a selected
production voice. No CosyVoice weights, dependencies, or inference engines were
installed or run during this catalog/UI change.

Implementation gates, in order:
1. Pin a specific model/runtime and evaluate Windows compatibility, licensing,
   reference-voice requirements and phonemization. Use an appropriately licensed
   standard voice rather than assuming arbitrary reference recordings can ship.
2. Add a loopback-only streaming synthesis adapter with cancellation and bounded
   read timeouts. Preserve per-language pacing, mute/volume and source retention.
3. Evaluate genuine written Cantonese translation, not Mandarin text in Traditional
   characters, against official paired Church sources where linguistically suitable.
   Source text and spoken-language provenance must be recorded separately.
4. Run English→Cantonese clause replay, subtitle alignment and delay tests, native
   review and simultaneous-output hardware tests before changing enabled=false.

Following requested priorities remain French, Japanese, Russian, Arabic, Hindi,
Basque, Italian, Tibetan, Afrikaans, Hawaiian and Māori. Catalog presence alone
does not clear voice licensing, compatibility, pronunciation or translation quality.
