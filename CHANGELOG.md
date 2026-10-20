# v0.5.0 — General Conference validation and religious terminology

- Collects and hash-checks 2017–2026 conference text in English/Spanish/German, with resumable offline caches and explicit coverage/exclusions.
- Supports modern publisher paragraph IDs, bounded alignment candidates and whole-year train/dev/test partitions.
- Adds resumable 100-case translation-only batches with pinned model/config/binary/corpus identity and partial-result recovery.
- Adds chrF scoring and review diagnostics for Church terminology, digits and possible negation changes.
- Adds an optional Latter-day Saint context/directional glossary; the supplied config enables it and older configs retain general mode.
- Adds a bounded exact-translation cache keyed by language/text/model/port/domain, with cache-disabled benchmark controls.
- Includes a real-model CPU pilot and targeted domain/performance probes, without claiming exhaustive inference or validated religious interpretation.
- Keeps all runtime inference local; no account, subscription or cloud fallback.

# v0.4.0 — religious speech evaluation

- Adds a Rust benchmark runner reusing the app's local recognition/translation/TTS adapters.
- Saves incremental per-case results, output WAVs, stage timings and failure-aware summaries.
- Supports bounded PCM16/16kHz/mono recordings and typed-text test cases without audio devices.
- Adds 13 aligned religious passages, generating 52 English/Spanish/German direction cases.
- Retains official scripture provenance, short excerpt hashes, and draft-reference review notes.
- Adds local installation preflight and makes startup refuse incomplete installations.
- Tests policy, corpus integrity, WAV parsing and runner success/failure through local mock engines.
- Does not claim model accuracy, live acoustic latency or source-voice matching.

# v0.3.1 — hardware configuration

- Targets the specified Xeon, modern Intel i7 and AMD Ryzen systems, RTX 4060+ and 32–256 GB RAM.
- Adds local free-VRAM detection, explicit CUDA/CPU modes, thread overrides and wrapper forwarding.
- Forces both inference engines off GPU in CPU mode; preserves existing GPU visibility restrictions.
- Documents portable native builds, allocation policy and outstanding hardware benchmarks.
- Adds seven offline launcher policy tests. Rust inference/audio code is unchanged.

# v0.3.0 — offline architecture

- Replaces Azure recognition/translation/TTS with local whisper.cpp, llama.cpp and native Piper.
- Removes Azure SDK bindings, account/key settings, cloud fallback and cloud setup scripts.
- Restricts HTTP inference to numeric loopback addresses, ignores proxies and disables redirects.
- Adds a local model/engine configuration, pinned 2.79 GB model inventory and optional setup helpers.
- Preserves Windows/Linux Rust UI, microphone/line channel selection and typed speech modes.
- Uses continuous capture with short clauses: approximately 360 ms pause detection and four-second cap.
- Retains bounded queues, persistent error state, cancellation, stale-result protection and playback stall detection.
- Cancels HTTP waits and kills/reaps the active Piper child on Stop/error.
- Reads each voice's actual sample rate and incrementally decodes local raw PCM.
- Adds local-only protocol tests and a real-model voice check example.

This version replaces v0.2's cloud streaming/interim-caption architecture. It does not
claim validated end-to-end real-time latency, source voice matching, installers or a
bundled native/model distribution. v0.2 remains a historical cloud-based prototype.
