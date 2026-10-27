# Windows pipeline and GPU validation — October 23, 2026

Tested on this Windows x64 machine with an NVIDIA GeForce RTX 5060 Ti (16 GB),
driver 610.88, four inference threads, and the repository's pinned model pack.
All eight model files passed SHA-256 verification.

## Actual inference

- CPU: both standalone Piper voices passed; four app-adapter cases passed
  (typed English and generated English audio into Spanish and German).
- CUDA: a 12-case repeated run passed with the application translation cache disabled.
  Two additional four-case verification runs passed, including explicit GPU-log checks.
- Whisper logs show `using CUDA0 backend`; llama.cpp logs show
  `offloaded 37/37 layers to GPU` with about 2,376 MiB of CUDA model buffers.
- Engines: official whisper.cpp b5454 and llama.cpp b11849 Windows CUDA 12.4 packages,
  matching llama CUDA runtime DLLs, and native Piper 2023.11.14-2.

The input fixture was Windows synthetic English: “Good morning. The meeting starts
at nine.” Its length was 3.755 seconds. Results are a short functional check, not
a sustained conversation or independent translation-quality evaluation.

| Measurement | CPU | Warmed CUDA |
|---|---:|---:|
| First generated PCM for audio input | 4.44–4.48 s | 1.11 s mean across six audio cases |
| Complete processing for audio input | 4.73 s | 1.40 s mean across six audio cases |

These times start after a complete fixture is submitted. They exclude live capture,
phrase segmentation, speaker hardware buffering and audible latency. Different numbers
are expected for actual live speech. This is an illustrative comparison on one PC.

## Physical microphone and speakers

The app opened and stopped capture using Microphone (USB Condenser Microphone),
channel 1, with Speakers (Sound Blaster Z), channel 1. The meter responded and detected
phrases. Both brief live sessions displayed zero dropped capture blocks and zero
clipped samples.

- Live English → Spanish on CPU: the sentence was recognized and translated;
  the user confirmed hearing Spanish. UI submission-to-first-queued-audio: 10.525 s.
- Live English → German on CUDA: the sentence was recognized and translated;
  the user confirmed hearing German. UI submission-to-first-queued-audio: 4.235 s.

These UI measurements are not acoustic measurements, and the two live trials use
different target languages. Audible output does not establish grammatical or semantic
accuracy; the German caption warrants native-speaker grammar review.

## Startup issues found and fixed

An initial CUDA diagnostic encountered translation/recognition deadlines during cold
kernel initialization and was stopped. The launcher now warms up translation and
recognition before exposing the app to its normal short deadlines. One subsequent
warm-up took 24.2 seconds; cached follow-up warm-up took about 0.5 seconds.

The original launcher considered an open socket ready, which caused HTTP 503 while
models were still loading. It now waits for HTTP `/health` success before warm-up.
The corrected actual GPU launcher started successfully and ran the live German test.

CPU and GPU engine folders, configurations and launchers remain separate. No NVIDIA
driver upgrade or system CUDA toolkit installation was required on this machine.

## Software checks

- Windows Rust all-target test suite: 25 passed.
- Existing Python launcher policy tests: 7 passed.
- Existing configuration tests: 6 passed, 1 platform-specific test skipped.
- PowerShell 5.1 archive installation/reuse/checksum regression test: passed.
- Diagnostic Python syntax check: passed.

Use `Start-Translation-GPU.cmd` for normal GPU operation, or `Start-Translation.cmd`
for CPU operation. Stop an existing launcher before starting another. See
`INSTALL_WINDOWS.md` for repeatable diagnostics and prerequisites.
