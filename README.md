# Live Translate — offline Rust prototype v0.5

Windows/Linux desktop voice translation focused on **Spanish and German**.
English is the default source; Spanish and German can also be sources.

**No Azure account, API key, subscription, paid service or internet connection is
required during use.** The app makes HTTP requests only to numerical loopback addresses
on this computer, ignores proxy settings, disables redirects, and runs Piper locally.
It contains no cloud fallback or automatic model downloads.

## Experimental meeting features

Live voice mode now supports a shared input feeding two independently routed Spanish/German outputs. It automatically saves source and translated SRT transcripts in a unique meeting subfolder of your selected project folder. The UI includes per-language delay traces and system CPU/RAM meters. See [DUAL_OUTPUT_VALIDATION.md](DUAL_OUTPUT_VALIDATION.md) for setup, short-test results, and the sustained/physical validation still required.

For the first new-language evaluation, see [MANDARIN_EXPERIMENTAL.md](MANDARIN_EXPERIMENTAL.md). Mandarin remains experimental and is not in the default model pack.

## Local pipeline

| Stage | Engine | Where it runs |
|---|---|---|
| Microphone/line capture and playback | Rust + CPAL | This PC |
| Speech recognition | Multilingual Whisper through whisper.cpp | Local process |
| Spanish/German translation | Local GGUF model through llama.cpp | Local process |
| Spoken output | Native Piper CLI + ONNX voice | Local child process |
| Desktop controls, captions, queues and cancellation | Rust + egui/Tokio | This PC |

The Rust app uses native inference engines; it does not require a Python inference
service. Python is used only by optional setup/launcher helpers.

## Modes

- **Live voice translation:** continuously captures mic/line audio, recognizes short
  clauses, translates them locally and speaks the result while capture continues.
- **Text → translated speech:** translates typed text into Spanish/German and speaks it.
- **Text → speech:** speaks typed Spanish/German text without translation.
- **Audio check:** tests the input meter/phrase detector locally, with no models needed.

See **OFFLINE_SETUP.md** for engine/model installation, including a USB/air-gapped route.
The source archive does not include model weights, native engines or compiled binaries.
The pinned model pack is about **2.79 GB**, plus engine programs. Downloads are an
explicit setup action and can be performed on another computer. Model verification
runs without network access.

## Build and start

Install current stable Rust. On Linux, install the ALSA/display development packages:

```bash
sudo apt-get install build-essential pkg-config libasound2-dev libx11-dev libxi-dev \
  libxrandr-dev libxcursor-dev libxinerama-dev libegl1-mesa-dev libwayland-dev libxkbcommon-dev
cargo build --locked
```

Windows requires Visual Studio C++ Build Tools, then `cargo build --locked` in a developer
terminal. Start the app with `cargo run --locked`. The default debug binary is in
`target/debug/`. For automatic Windows dependency setup, run `Install-Windows.cmd`; then use
`Start-Translation.cmd`. For NVIDIA acceleration, run `Install-GPU.cmd` and use
`Start-Translation-GPU.cmd`. See **INSTALL_WINDOWS.md** and **WINDOWS_VALIDATION.md**.

For the complete local pipeline after installing the engines/models:

```bash
python3 scripts/start_offline.py
# In another terminal from the project directory:
cargo run --locked
```

On Windows, use `python` instead of `python3`. `scripts/launch.sh` and `scripts/launch.ps1`
combine engine startup with launching an already-built debug executable. The startup
helper defaults to automatic GPU selection with CPU fallback (see HARDWARE.md),
never downloads anything and binds the inference engines only to 127.0.0.1.

Select the input device/channel, output device/channel, languages and local configuration
file (`offline.json` by default). Use headphones or an isolated output feed so translated
speech is not recaptured. Only the selected output channel carries audio.

See **MEETING_BUFFER.md** for speaker transitions, music breaks and finishing pending speech.

## Real-time limits and voice fidelity

This is a local short-clause pipeline, not a verified end-to-end streaming speech model.
It uses an energy gate, about 360 ms pause detection and a four-second maximum clause.
Continuous speech can be split at that maximum, potentially cutting a word or losing
sentence context. The earlier Azure streaming/interim-caption path has been removed.
Captions now appear when a local clause is recognized and translated.

Local model inference and sentence structure add delay. **Sustained real-time performance
and acoustic end-to-end latency have not been measured.** A fast CPU/GPU and appropriate
model sizes determine whether it can keep up. The hardware design target is the specified Intel/AMD CPUs, an RTX 4060 or later,
and 32–256 GB system RAM. See **HARDWARE.md** for GPU selection, CPU fallback and
benchmark requirements. This target has not yet been validated on those machines.

The timing display estimates clause/text submission to first queued audio, excluding
hardware buffering. It is not an acoustic latency measurement. The standard local
Spanish/German voices do **not** match the original speaker's timbre, tone or inflections.
Local voice/prosody preservation remains a later engine integration and performance task.

## Reliability behavior

- Capture, recognition and translated speech use separate workers and bounded queues.
- PCM playback has a strict 800 ms buffer; a stalled output is detected after two seconds.
- Stop cancels local HTTP waits, kills the active Piper child and silences playback without
  depending on delivery of UI events. A local server can finish inference already submitted.
- The first error is retained in session state. Restarted sessions have separate buffers
  and stop tokens, preventing old results from entering a new session.
- Live source speech is buffered in memory (60 minutes by default, adjustable up to two
  hours). A busy speech worker waits for space; older buffered speech does not expire.
  Backlog is visible, and music pause/finish controls allow pending speech to drain.
  Device failures and engine errors still stop processing visibly; automatic retry is
  not implemented. See **MEETING_BUFFER.md** for capacity limits and recovery.
- Captions and the live source backlog stay in memory during normal operation. An
  explicit recovery button can save pending source audio after an error. Stop discards
  pending audio. The local engines have their own console/output behavior.

## Religious speech profile and General Conference

The supplied configuration now uses `"translation_domain": "latter_day_saint"`, adding
context and source/target terminology for sacrament, stake, ward, covenants, priesthood,
Atonement, Holy Ghost, deacon and bishop. It also instructs the model to preserve the
spoken content without adding doctrine or completing quotations. Choose `"general"`
for the original profile. Older configs without this optional field retain general mode.
A profile is guidance to the model; it is not a guarantee of accurate interpretation.

The ten-year conference collection covers April 2017–October 2026: 717 text entries in
three languages, 2,151 pages, and 21,172 candidate paragraph triplets. One video-only
entry is explicitly excluded. See **evaluation/GENERAL_CONFERENCE.md** and **reports/**
inside evaluation for collection coverage, actual model results, resumable full-corpus
runs, domain checks and performance probes. The full 84,688-case inference run has not
been completed; the current real-model pilot attempted 80 cases across all 20 conferences.

An in-memory cache retains up to 64 exact completed translations per engine/session.
Different languages, text, models, ports or domain profiles have distinct keys. Stop
remains effective. The cache is not persisted to disk. Bulk benchmarks disable it by
default. Most conference paragraphs are novel, so this optimization helps exact repeats
without solving CPU-only latency for new material.

## Initial religious-speech evaluation

See **evaluation/README.md** for a 52-case English/Spanish/German test set using short
official Book of Mormon and other scripture excerpts plus original meeting sentences.
The new Rust `offline_benchmark` example uses the actual app adapters, saves translated
WAVs and reports processing timings. It supports typed text and human-recorded WAVs.
The launcher checks local installation structure before starting inference engines;
`python3 scripts/check_offline.py` runs the same checks independently.

## Validation

See **VALIDATION.md**. Current Linux checks passed 25 Rust tests, 30 Python tests,
formatting and Clippy, and built/executed the benchmark with an actual local model.
The General Conference pilot and targeted checks are documented separately in
**evaluation/reports/RESULTS.md**. Previous Windows target compilation and real Piper
Spanish/German synthesis checks are retained as historical evidence. Current GPU,
Windows execution, complete voice-pipeline, listening accuracy and physical-device
validation remain pending.

```bash
cargo fmt --check
cargo check --all-targets --locked
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
```

The automated software tests use local mock HTTP engines and in-memory PCM streams.
They do not download models or certify translation accuracy. Actual model runs are
reported separately. License and attribution details for the
selected third-party engines/models are in **THIRD_PARTY.md**.
