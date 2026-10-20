# Offline setup — Windows/Linux x64

No paid service or account is needed. Install the engine programs and model files once;
normal operation can then run with the internet disconnected. The Rust application is
the UI/audio/pipeline controller; native programs perform inference on the same PC.

## 1. Install native engines

Use the engines' official sources/releases and keep their accompanying libraries/data.
Set their exact paths in `offline.json` if your layout differs.

| Engine | Default executable location | Source |
|---|---|---|
| whisper.cpp `whisper-server` | `engines/whisper/whisper-server[.exe]` | https://github.com/ggml-org/whisper.cpp |
| llama.cpp `llama-server` | `engines/llama/llama-server[.exe]` | https://github.com/ggml-org/llama.cpp |
| Native Piper CLI | `engines/piper/piper[.exe]` | https://github.com/rhasspy/piper/releases/tag/2023.11.14-2 |

The Piper adapter uses the native CLI's `--model MODEL --output_raw` contract. It was
smoke-tested on the legacy 2023.11.14-2 release. Retain its ONNX runtime libraries,
phonemizer and espeak-ng data directory. The maintained Piper successor is
https://github.com/OHF-Voice/piper1-gpl ; its licensing/build/CLI differ, so evaluate that
native distribution before substituting it. The app does not invoke a Python TTS server.

For whisper.cpp or llama.cpp, official build instructions use CMake. Build the server
target on the intended machine or use a compatible release package. If building static
versions, `-DBUILD_SHARED_LIBS=OFF` simplifies copying; otherwise retain all required
shared libraries beside the executable. GPU acceleration is optional and requires an
engine built for that backend. The Rust controller itself does not implement CUDA.

## 2. Install the pinned model pack

The supplied `models-manifest.json` pins source revisions, file sizes and SHA-256 values.
The helper's default invocation only prints the plan:

```bash
python3 scripts/fetch_models.py
```

Explicit one-time download (approximately 2.79 GB):

```bash
python3 scripts/fetch_models.py --download
```

On Windows replace `python3` with `python`. No authentication token is used. The helper
refuses to overwrite an existing file with a different hash. It verifies downloads before
moving them into place. Retain the downloaded voice MODEL_CARD files and attribution.

The default models are multilingual Whisper base, Qwen3-4B Q4_K_M, Piper Spanish SHARVARD
medium and German Thorsten medium. The Spanish voice is from Spain; source recognition
is selected independently of voice accent. Other local models/voices can be configured,
but their quality, sample rates, licenses and inference compatibility need evaluation.
A `.onnx` voice requires its matching `.onnx.json`; the app reads its sample rate rather
than assuming 24 kHz. Spanish SHARVARD uses its default speaker ID 0.

The translation server must support the model's chat template and
`chat_template_kwargs: {"enable_thinking": false}`. Qwen3 is used in non-thinking mode;
reasoning or incomplete outputs are rejected instead of being spoken. LLM translation
still needs native-speaker accuracy evaluation, particularly for names, numbers,
negation, specialized terms and short context fragments.

## 3. Fully air-gapped deployment

On another computer, prepare this source/application, engine directories, native runtime
libraries and the model pack. Copy them by USB or other offline media. On the target:

```bash
python3 scripts/fetch_models.py --verify
```

Verification makes no network requests. `start_offline.py` and the Rust app do not download
anything. Build the Rust app beforehand or transfer an appropriate compiled binary if
the target is also disconnected from Rust's package registry. No credentials expire.

## 4. Start the local engines

Review paths, ports and CPU thread count in `offline.json`. Check the full installation
structure with `python3 scripts/check_offline.py` before launching:

```bash
python3 scripts/start_offline.py --print-commands
python3 scripts/start_offline.py
```

The printed commands show the exact argv arrays. The helper starts both engines with
`--host 127.0.0.1`; it refuses to reuse occupied ports and stops its child processes when
it exits. It needs only Python's standard library. Keep this terminal open while the app
runs. Startup waits for listening sockets; engine console output is useful while models
are loading. A listening socket alone does not validate model inference.

The launcher defaults to automatic GPU selection for the supplied model pack. It uses
CUDA when the local driver reports sufficient free VRAM, otherwise explicitly disables
GPU inference in both engines. CUDA-enabled engine builds are required. The configured
`gpu_layers` is 99; CPU mode overrides it to zero. See **HARDWARE.md** for the target CPUs,
RTX 4060 baseline, visibility rules, manual overrides and portability requirements.
Use `--device cpu` if CUDA startup fails; there is no automatic session restart.

The app's local URLs are constructed internally from port numbers. They cannot be
configured as remote hosts. HTTP proxies and redirects are disabled. No API headers,
keys or cloud fallback are used. Local inference servers use HTTP on loopback only.

## 5. Run and evaluate

Build/start the Rust app as in README.md. English → Spanish and English → German are the
first evaluation pairs; Spanish ↔ German uses the same local path. Select the intended
microphone or interface line-input channel and headphones/output channel.

You can check an installed voice without an audio device or either HTTP server:

```bash
cargo run --locked --example local_voice_check -- offline.json es spanish-test.wav
cargo run --locked --example local_voice_check -- offline.json de german-test.wav
```

These commands create new WAV files and refuse to overwrite existing output. They test
local TTS only. Measure sustained interpretation delay and review translations with
native speakers before using the app at an event.

## Subsequent development

Replace energy gating/hard clause cuts with speech-aware boundaries, evaluate models on
Spanish/German recordings, tune inference on the target GPU/CPU, then add an offline
speaker-preserving engine. Model management can later be integrated into the Rust UI;
the present helper setup does not claim a complete installation wizard.

See **evaluation/README.md** for religious speech tests, aligned scripture references,
recording conventions and the Rust benchmark runner.

The optional `translation_domain` configuration field supports `general` and
`latter_day_saint`. The supplied config selects the religious profile; older configs
without the field retain general mode. General Conference collection/evaluation helpers
are described in **evaluation/GENERAL_CONFERENCE.md**. Normal translation still uses
local engines with no subscription, API key or external fallback.
