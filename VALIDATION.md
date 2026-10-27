# Windows meeting buffer validation — 2026-10-23

- Windows Rust all-target suite: 33 passed, run serially.
- Clippy all targets with warnings denied: passed.
- Updated Windows desktop executable: built successfully.
- Real-time CUDA replay of the supplied 100.702-second Title Page recording:
  Spanish and German each completed all 38 accepted/recognized/translated clauses,
  drained the backlog, and left zero pending audio. Playback finished at 106.73 s
  (Spanish) and 106.46 s (German).
- The desktop Start, music pause, resume, and Finish controls were exercised with a
  silent physical input. Finish includes a one-second output-device drain allowance.
- Added tests cover 30 minutes of capacity with another 30 minutes headroom, source
  retention during inference, old audio without deadline expiry, capacity pause/drain,
  music-boundary flushing, recovery export, and pending audio after a failed engine.

A parallel suite run on a loaded host had two existing provider timing tests fail;
all tests passed when rerun serially. A full 30-minute live meeting, acoustic latency,
and native-speaker accuracy have not been validated by this update. Details and
retention behavior are in **MEETING_BUFFER.md**; summaries are in
`evaluation/reports/windows-2026-10-23/buffered-es.json` and `buffered-de.json`.

The release notes below describe prior validation, before live meeting buffering.

---

# v0.5.0 General Conference validation — 2026-10-19

Current checks on Linux x86-64 / Rust 1.99.1:

| Check | Result |
|---|---|
| `cargo test --all-targets --locked` | 25 passed |
| `cargo clippy --all-targets --locked -- -D warnings` | Passed |
| Rust formatting | Passed |
| Linux debug benchmark example | Built and executed with actual local model |
| Python tests, including compiled Rust adapter integration | 30 passed |
| Python helper syntax / Bash launcher syntax | Passed |
| Full eligible text-page cache verification | 2,151/2,151 pages, 20 conferences, zero missing eligible pages |
| General-profile real Qwen pilot | 78/80 completed; two translation deadlines |
| Final religious-profile targeted check | 8/8 completed; one terminology review flag remains |
| Ordering and exact-cache probes | 24 cases completed, including three intentional application cache hits |

The real-model runs used pinned Qwen3-4B Q4_K_M weights with verified SHA-256,
llama.cpp b11739 and four CPU threads. The 80-case baseline used the development
benchmark before the final version bump; the final domain/probe executables use v0.5.
The table deliberately separates automated software checks, corpus collection, successful
inference and failed inference. A passing software test is not evidence of religious
translation accuracy. No model training/fine-tuning occurred.

See **evaluation/reports/RESULTS.md** and its JSON reports for exact counts, failures,
chrF settings, timing limits, domain diagnostics, corpus inventory and source hashes.
Published reports contain no full talk texts or full generated translations.

The full 84,688-case inference job, semantic alignment review, native-speaker review,
NVIDIA execution, target-hardware real-time behavior, live audio/ASR/TTS quality and
current Windows execution/build checks remain unverified. The default source voices
still do not reproduce the input speaker's timbre or expression.

## Prior release evidence (historical)

# v0.4.0 religious evaluation validation — 2026-10-19

This release adds the Rust benchmark example and Python corpus/preflight helpers.

| Current check | Result |
|---|---|
| Rust all-target tests | 22 passed, including three new benchmark tests |
| Rust all-target Clippy, warnings denied | Passed |
| Rust formatting | Passed |
| Linux debug benchmark example | Built and executed |
| Python tests | 18 passed, including the black-box Rust runner integration |
| Python helper syntax / Bash launcher syntax | Passed |
| Religious corpus generation | 52 cases, four language directions, aligned source/reference fields |
| Official excerpt integrity | 15 language excerpts, each with URL and SHA-256, extracted from official pages |
| Preflight on missing installation | Correctly reports missing components and returns nonzero |

The black-box test launches two **fake** local HTTP engines and a fake Piper process,
then runs the actual compiled Rust benchmark executable. It checks text input, recorded
WAV input, real adapter/protocol routing, first PCM reporting, valid output WAV files,
truncated-translation rejection, continuing/reporting failures, nonzero failure exit,
and refusal to overwrite results. It also sends all 52 generated religious cases through
the Rust runner. The fake translation text is deliberately fixed; successful plumbing
is **not evidence of correct translation**. Generated mock WAVs/results are temporary
and excluded from the source delivery.

The three benchmark unit tests cover bounded WAV decoding/format rejection, nearest-rank
percentiles and invalid/ambiguous case rejection including unsafe/colliding output IDs.
Python tests cover corpus alignment/hash integrity, recording requirements, installation
layout/errors and CPU/GPU selection policy.

No actual Whisper/Qwen model inference, NVIDIA execution, live mic/line audio, acoustic
latency or bilingual accuracy review was performed in this release. Current Windows
runtime and the new benchmark's Windows target were not tested here. Prior checks below
are retained as historical evidence only. The source archive contains no native engines,
models or compiled binaries. Standard Piper voices still do not preserve source timbre.

## Prior release evidence

# v0.3.1 hardware configuration validation — 2026-10-19

This update changes launcher policy, configuration and documentation. Rust source files
are byte-identical to v0.3.0. Seven Python hardware-policy tests passed, covering local
driver parsing/failure, free-VRAM selection, visibility restrictions, explicit overrides,
CPU disabling both GPU backends and loopback-only engine commands. Python syntax,
Bash launcher syntax, and auto/CPU/CUDA command plans passed. Auto mode correctly chose
CPU in this environment without NVIDIA driver tools. Windows PowerShell forwarding
was reviewed but not executed here.

No Rust compiler, native model engines or NVIDIA GPU are available in this update's
environment; the previous Rust checks below were not rerun. GPU execution, actual
VRAM consumption and target-hardware latency remain unverified. CUDA detection does
not establish that the installed engine binaries contain CUDA support.

## Prior v0.3.0 checks (retained evidence, not rerun)

# v0.3 offline validation — 2026-10-19

Host: Linux x86_64, Ubuntu 24.04.3 / glibc 2.39. Rust 1.99.1.
This delivery contains source, not model weights, native engines or application binaries.

## Completed checks

| Check | Result |
|---|---|
| `cargo fmt --check` | Passed |
| `cargo check --all-targets --locked` | Passed |
| `cargo test --locked` | 19 passed, 0 failures |
| `cargo clippy --all-targets --locked -- -D warnings` | Passed |
| `cargo build --locked` | Linux debug desktop executable linked |
| `cargo check --all-targets --target x86_64-pc-windows-gnu --locked` | Passed on the Windows target, cross-compiled from Linux |
| Setup helpers | Python syntax, offline download-plan output and local command-plan output passed |
| Shell launcher | `bash -n` passed |
| Spanish real-model Piper check | 2.88 s of mono audio generated locally in 0.44 s, 22,050 Hz |
| German real-model Piper check | 2.86 s of mono audio generated locally in 0.41 s, 22,050 Hz |

The TTS checks used the actual native Piper 2023.11.14-2 executable and downloaded
SHARVARD/Thorsten ONNX voices through the Rust `local_voice_check` example. They include
process/model startup and PCM collection in this container. They are synthesis smoke
checks, not translation accuracy, perceptual listening quality or end-to-end latency
benchmarks. No inference service account or cloud speech API was used.

## Automated coverage

Tests exercise the local whisper.cpp multipart/WAV request contract, local llama.cpp
Spanish/German request routing, no authorization header, rejection of truncated model
responses, request cancellation while waiting for a local server, incremental local
PCM decoding before EOF, odd sample boundaries, cancellation of incomplete PCM reads,
stateful resampling, alias rejection, VAD boundaries, language gates, persistent first
errors, UI-independent stop/silence, session isolation, bounded playback and stall detection.

HTTP tests use loopback mock servers. They do not run Whisper or a translation model.
Endpoint construction is fixed to 127.0.0.1 from configured ports; the client disables
proxies and redirects. Source review confirms no Azure SDK, account/key settings or
cloud fallback in runtime code. The setup downloader is separate and explicitly invoked.
The default model plan contains eight pinned files totalling 2,785,177,029 bytes.

## Still required

- Model-backed Whisper recognition and Qwen translation tests on Spanish/German recordings.
- Complete mic → local recognition → translation → speech sessions and native-speaker review.
- Accuracy on names, numbers, negation, specialized terms, accents, noise and context splits.
- Sustained latency/throughput benchmarks on the target CPU/GPU. No real-time guarantee yet.
- Physical audio devices, channel isolation and GUI operation on Linux and Windows.
- Windows native Piper/runtime/library tests; target compilation alone does not validate them.
- Full model-pack download/verify and launcher startup with actual ASR/translation servers.
- Release builds, installers and execution of the supplied remote CI workflow.

The first offline implementation uses short clauses (about 360 ms silence or four-second
maximum), not the retired cloud SDK's incremental recognition callbacks. Hard maximum
cuts can split a word; context-aware boundaries and local voice identity/prosody matching
remain future work. This version prioritizes local-only operation and clear queue limits.
