# Stronger model development comparison

October 25, 2026. Downloaded and hash-verified Qwen3-8B Q4_K_M and Whisper medium.en into the separate workspace. Revisions, sizes and SHA-256 are in model-evidence.json. Application defaults, source code and installed model pack are unchanged. This is the existing engine stack with alternative weights, not implementation of the deferred configurable-workflow architecture.

## Translation: Qwen3-8B

All 56 direct requests completed with finish_reason=stop and no introduced Cyrillic: 28 current-prompt cases and the same 28 with selected terminology. Each set contains the 12 actual Arabic/Hindi failures and 16 name/ordinary/narrative/number/negation controls. The 4B current-prompt comparison had 11 rejected requests among the 12 original failure cases (nine Arabic Cyrillic, two Hindi length). These are development cases already inspected, not held-out evaluation.

The actual Rust offline_benchmark then exercised the unchanged current prompt, validation and Piper adapters with translation cache disabled: 46/46 produced non-empty audio. This includes 28 Arabic/Hindi development cases plus three smoke cases each for Spanish, German, Mandarin, French, Japanese and Russian. Sequential text-input first PCM median/p95 were 0.442/4.122 seconds, including the first case. These numbers exclude recognition/input buffering, concurrent scheduling, physical playback and acoustic latency; do not compare them directly to live source-to-output delays.

The larger model visibly improves script compliance and several scripture/narrative/number cases. It is still not semantically reliable on all controls. Examples under the current prompt: organ became a lute; Patrick Kiran became Peter Kiran; a doctrinal foundation became mythological in one fragment; dinner plates were translated as writing boards in Arabic and Hindi. The selected-terms candidate fixed some ordinary-sense errors but introduced others (Arabic bought became wrote; Hindi historical account reverted to financial account). Therefore fewer rejections is progress, not accuracy approval. Native-language review remains required. No default replacement is made on this evidence.

A sampled GPU-memory observation during the runtime test was 8,020 MiB with both engines loaded on the 16,311 MiB GPU. That is a point sample, not a measured peak or sustained-capacity guarantee. Eight simultaneous route testing is recorded separately below when complete.

## Recognition: Whisper medium.en

Sixteen requests: eight original short/context/control clips without hints, then eight with the same Church-name hint. Without hints the longer Oaks clip correctly recognized Dallin Harris Oaks, but the short clip gave Allen Harris Oaks, the name list returned repeated [P] markers, Kearon/Soares remained wrong, and ordinary short speech gained extra words. With hints Kearon/Soares improved, but Oaks remained misspelled and the silence control hallucinated a sentence about the Book of Mormon. This is a clear regression for the tested hinted configuration. No recognition-model or hint change is promoted.

## Evidence and next gate

Raw source text/audio offsets, exact prompts, responses, finish reasons and timings are preserved. The optional Qwen3-8B config is language-rollout/qwen8b-runtime/config.json in the workspace and uses absolute existing engine/voice paths plus the separate candidate model. Its presence does not make it the application default. Recognition controls make clear why source transcripts must not be silently rewritten from expected names.

Next: targeted meaning controls with official paired Church references, followed by a longer eight-route candidate run before any model-pack/default decision. Native accuracy, voice pronunciation and sustained hardware estimates remain separate gates. Existing clause-local untranslated markers and gap plots stay enabled.


## Completed eight-stream smoke test

A 70-second original-soundtrack replay completed with gaps using Whisper base plus Qwen3-8B, four CPU threads per engine and one translation slot. All eight languages handled all 18 clauses: 136 audio deliveries and eight untranslated cues (one shared music-symbol-only clause). All nine SRT files aligned. Backlog drained in 22.30 seconds; largest delivered first-PCM delay was 23.89s (Arabic). No cutoff, retry or crash. Physical/acoustic latency was not measured.

This is a short shared-load test, not the 15-minute candidate validation. Earlier Qwen3-4B 70-second results remain separate; this run alone does not establish faster translation or better sustained delay. Plot and per-language median/p95/max: eight-stream-smoke/fault-tolerance-report.md and fault-tolerant-delay.png.
