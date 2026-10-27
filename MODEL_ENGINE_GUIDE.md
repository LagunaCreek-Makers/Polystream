# Models, test results and custom engines

## Choosing a pretested configuration

Settings > Pretested Models offers a dropdown of named tested workflows. Choose one, review the speech-to-text / translation / speech summary, then click Use this tested configuration. This selects all model paths and the tested CPU/GPU parameters together: CPU + NVIDIA GPU, four threads, one translation slot, 4096 context tokens and 99 GPU layers. Apply & restart engines activates it. Choosing a dropdown item alone is only a preview. Finish or stop the current meeting first. Selecting a preset preserves installed voices, local ports and the terminology domain, and resets custom adapter overrides to stock engines. Generated files never overwrite your original configuration. CPU + NVIDIA GPU remains the default; Restore advanced defaults resets four threads, one translation slot and 4096 context tokens.

Fastest tested baseline: Whisper base, Qwen3-4B Q4_K_M, Piper voices. Recommended starting point for Spanish/German and French. This is a tested speed-oriented baseline, not a guarantee that it is fastest for every language or sentence.

Best tested translation candidate: Whisper base, Qwen3-8B Q4_K_M, existing Piper voices. Experimental. Fewer rejected Arabic/Hindi clauses and improved development controls, but remaining meaning errors. Consider it for Arabic/Hindi evaluation if the added delay is acceptable. No model here is certified universally best or production-accurate.

Whisper small.en and medium.en: experimental, not recommended as default. Name recognition did not reliably improve; hinted tests introduced extra speech, including a silence hallucination with medium.en. The presets do not enable name hints. These recognition models accept English only.

Mandarin/Japanese/Russian/Italian: existing experimental/validation status remains unchanged by choosing a larger translator. Mandarin voice dataset provenance and other optional voice terms remain separate from model quality. Cantonese, Tibetan, Afrikaans, Hawaiian and Maori voice gaps remain open; Basque translation quality is blocked. Choosing a model does not silently enable unsupported languages.

## What the tests measured

Same 15-minute recording, eight simultaneous outputs, Whisper base, four threads per engine, one translation slot. 301 source clauses per language; all nine SRT files aligned. Qwen3-4B: 2390 audio deliveries and 18 untranslated cues; final drain 63.01s. Qwen3-8B: 2398 deliveries and 10 untranslated cues; drain 77.83s. Eight cues in each run were the same music-only marker. No engine crash or 600-second cutoff occurred.

Language: 4B median/p95/max seconds -> 8B median/p95/max seconds
Spanish: 11.60/29.81/41.43 -> 9.71/31.00/33.52
German: 13.19/29.24/37.90 -> 12.22/28.37/35.14
Mandarin: 7.46/24.97/37.69 -> 9.98/29.16/37.78
French: 17.67/31.85/39.83 -> 17.37/34.00/36.22
Japanese: 6.69/26.54/40.02 -> 12.67/39.29/45.35
Russian: 23.06/37.17/44.57 -> 27.09/44.40/47.42
Arabic: 57.09/77.71/84.96 -> 70.40/91.91/96.64
Hindi: 5.05/13.58/19.44 -> 5.66/16.57/19.13

These are successful-clause source-start to first-queued-PCM measurements, excluding physical device/acoustic latency. Untranslated clauses are gaps, not zero delay. One run per model is evidence for this machine and soundtrack, not a controlled hardware comparison or a promise on another meeting. GPU memory sampled peaks: 6753 MiB / 8259 MiB. Model size/CPU/GPU changes invalidate previous capacity calibration. Additional-language accuracy checks now run individually; concurrency testing is separate.

Detailed reports and plots: evaluation/reports/qwen8b-sustained-2026-10-25/ and evaluation/reports/stronger-models-2026-10-25/. Official terminology references, native-review limitations and failure evidence are saved there. Arabic doctrinal-foundation correction was tested separately after the sustained run. Current ordinary/religious sense issues are in evaluation/reports/individual-senses-2026-10-26/.

## Installing the tested model files

Presets check installed files before selecting them; no silent network download. Standard paths beside the project configuration:
models/whisper/ggml-base.bin
models/whisper/ggml-small.en.bin
models/whisper/ggml-medium.en.bin
models/translation/Qwen3-4B-Q4_K_M.gguf
models/translation/Qwen3-8B-Q4_K_M.gguf

Use the existing installer for the baseline. Download the exact candidate with hf download Qwen/Qwen3-8B-GGUF Qwen3-8B-Q4_K_M.gguf --local-dir models/translation. Tested revision: 7c41481f57cb95916b40956ab2f0b139b296d974. Use --revision with that value for reproduction. SHA-256: d98cdcbd03e17ce47681435b5150e34c1417f50b5c0019dd560e4882c5745785.
Whisper medium.en tested revision: 5359861c739e955e79d9a303bcbc70fb988958b1 from ggerganov/whisper.cpp. Hash/provenance for all candidate files is recorded in the stronger-model report. Retain upstream model, runtime and voice notices when sharing; application source licensing does not replace component terms.

## Define your own workflow in Settings

Open Custom engine configuration, load the selected JSON, edit it, then Validate & save as new custom configuration. Paths may be absolute or relative to the selected configuration's directory. The saved file uses absolute model/engine/voice paths. Apply & restart engines starts the selected executable programs. Standalone app launches cannot manage engine restarts; use Start-Translation.cmd or Start-Translation-GPU.cmd.

For compatible weights, change whisper_model (Whisper GGML), translation_model (llama.cpp GGUF) and voices (language-code to Piper voice path). Each voice needs a sibling MODEL.json with audio.sample_rate. Keep ports distinct and in 1..65535. threads is 1..128. translation_domain is general or latter_day_saint. Unknown configuration fields are rejected rather than ignored.

Different engine architectures are supported through compatible local adapters, not by pointing a GGUF field at an arbitrary checkpoint. Implement the contracts below. Mark asr_custom_adapter or translation_custom_adapter true to allow a local descriptor file in the corresponding model field instead of enforcing GGML/GGUF magic. The descriptor must exist; its content is interpreted by your adapter. Stock configurations keep format checks enabled. Configure the adapter executable as whisper_server or llama_server. Optional asr_launch_args / translation_launch_args arrays are passed before the standard launch flags, as separate arguments without a shell. For Python, use its executable path and an absolute script path as the first launch argument. Custom configurations preserve these executable choices rather than silently replacing them with stock binaries.

Example fields for a custom recognition adapter:
"whisper_server": "C:/engines/my-venv/Scripts/python.exe"
"asr_launch_args": ["C:/engines/my_whisper_adapter.py"]
"asr_custom_adapter": true
"whisper_model": "C:/engines/my_asr_descriptor.json"

This is a fragment to merge into a complete existing configuration, not a standalone JSON file. Keep the remaining required fields and voices. An adapter is a program you install/write; the app does not automatically implement faster-whisper, NLLB, Qwen-TTS or other APIs.

## Speech-to-text adapter contract

Launcher passes -m MODEL --host 127.0.0.1 --port PORT -t THREADS; CPU mode adds --no-gpu. Accept these options or translate them in your wrapper. Implement GET /health returning HTTP 200 only when the model is ready. Implement POST /inference multipart fields: file is mono PCM16 WAV at 16000 Hz, language is en/es/de, translate=false, temperature=0.0, temperature_inc=0.0, response_format=json. Return HTTP 200 JSON {"text":"recognized supplied speech"}. Empty text means no recognized speech. Do not invent words on silence. Host is fixed to loopback by the app. Recognition timeout is 30s plus 5s to read the response.

## Text translation adapter contract

Launcher passes -m MODEL --host 127.0.0.1 --port PORT -t THREADS -ngl LAYERS -c CONTEXT --parallel SLOTS --alias local-translation --jinja. Accept/translate these options; CPU mode sets layers zero. Provide GET /health and POST /v1/chat/completions. Requests contain model=local-translation, system/user messages, temperature=0, max_tokens=1024, stream=false, chat_template_kwargs.enable_thinking=false. Preserve the user fragment, names, numbers and negation. Return {"choices":[{"finish_reason":"stop","message":{"content":"translation"}}]}. Never return stop for truncated output; use length or an HTTP error so the app saves an untranslated source cue and continues. No streaming or remote endpoint is consumed by the current adapter. Timeout: 30s request, 5s response read.

## Text-to-speech adapter contract

Simplest compatible executable: set piper_binary to your wrapper. It receives --model VOICE --output_raw, reads one UTF-8 text request on stdin and emits mono signed little-endian PCM16 at the configured voice sample rate on stdout. Diagnostics belong on stderr; exit nonzero on failure. Raw output must not contain WAV headers or log text. Maintain an existing piper_binary file even when using persistent workers because preflight checks it.

Persistent Python option for a different TTS library: set piper_python_voices[language-code] to your isolated Python executable and piper_worker to your own worker script. The worker receives -u SCRIPT --model VOICE --threads N. Read UTF-8 JSON lines {"text":"..."}. For each request emit one or more frames: 4-byte unsigned little-endian byte count followed by that many PCM16 bytes; then a zero-length frame to finish. Flush frames promptly. See scripts/piper_worker.py and provider.rs for an executable reference implementation. Reuse the model across requests; never mix requests. VOICE must be an existing file plus VOICE.json containing {"audio":{"sample_rate":22050}} (replace the rate with your actual 8000..96000 rate). It may be an adapter descriptor if your worker handles it; the stock Piper worker requires a real compatible ONNX voice. One global worker script serves the configured Python voices, so a multi-family custom worker must dispatch by descriptor. Frames/timeouts/size limits are checked; partial synthesis failures still stop the session rather than inventing replacement audio.

## Test and troubleshoot your custom engine

Run python scripts/check_offline.py --config YOUR_CONFIG.json for file/schema preflight. Inspect planned launch arguments with python scripts/start_offline.py --config YOUR_CONFIG.json --print-commands. Existing saved launcher preferences override that command; use a clean configuration directory or inspect engine-settings.json when diagnosing. Launch through the app only after health and sample requests pass. Run the Rust offline_benchmark example with a cases JSON containing one target language; include silence/negation/numbers/religious and ordinary senses. Use --no-translation-cache. Audio cases need a mono PCM16 16000-Hz WAV and exactly one of text/audio per case. Save output in a new directory.

A minimal text case: [{"id":"sample-1","source":"en","target":"es","text":"I do not agree."}]
Invocation: target/debug/examples/offline_benchmark.exe YOUR_CONFIG.json CASES.json NEW_RESULTS --no-translation-cache

Then test a short live session, routing and subtitles. Record model revision/hash, hardware, prompts, latency, untranslated counts and native review. Never mark a custom engine pretested solely because files load or audio is produced. Keep the original configuration for rollback. To restore it, load/select its path in Advanced settings and Apply & restart engines; custom paths keep custom executable choices.
