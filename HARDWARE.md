# Offline hardware target

Design target supplied by the user, not a list of benchmarked/certified systems:

| Component | Target |
|---|---|
| CPU | Intel Xeon E5-2600 v3 family; Intel Core i7 12th generation or later; AMD Ryzen 5500 or later |
| GPU | NVIDIA RTX 4060 or later, with compatible local driver and CUDA-enabled inference engines |
| System RAM | 32–256 GB, DDR4 or later as supported by the motherboard |
| OS / architecture | Windows or Linux, x86-64 |

## Baseline allocation

Keep the pinned multilingual Whisper base and Qwen3-4B Q4_K_M models for the initial
Spanish/German evaluation. Run recognition and translation on CUDA; keep native Piper,
audio capture/playback and UI on the CPU. The RTX 4060 has 8 GB VRAM. The initial design
therefore targets that capacity, even when system RAM is 256 GB. System RAM does not
increase dedicated GPU memory. Larger models require separate memory and latency tests.

The launcher defaults to `--device auto`. It queries the local `nvidia-smi` driver tool
with a three-second timeout, selects the GPU with the most free VRAM among those reporting
at least 6144 MiB free, and restricts both child engines to its UUID. This threshold is
an initial policy for the supplied models, not a measurement or reservation. Other
processes can consume VRAM after selection. Detection does not prove that engine binaries
contain CUDA support or that model inference will succeed; check their startup logs.

If no eligible GPU is reported, auto mode starts both engines in CPU mode. An existing
`CUDA_VISIBLE_DEVICES` setting causes auto mode to choose CPU rather than overriding a
user/administrator restriction. To use that configured environment, request CUDA explicitly.

```bash
python3 scripts/start_offline.py --device auto --print-commands
python3 scripts/start_offline.py --device cuda
python3 scripts/start_offline.py --device cpu
python3 scripts/start_offline.py --device cuda --threads 6
```

Use `python` on Windows. Both launch wrappers forward these options. Explicit CUDA mode
keeps the inherited GPU visibility settings. `gpu_layers: 99` in `offline.json` requests
full offload for the supplied 4B model; it is adjustable for other models. Whisper uses
its engine's GPU default. CPU mode forces Whisper `--no-gpu` and llama `-ngl 0` regardless
of the configured layer count. CPU mode is a compatibility path and may not keep up with
live conversation, particularly on older Xeons.

Start with four CPU threads **per engine**, leaving resources for Piper, capture and UI.
Compare four, six and eight threads on each machine; do not assume all logical cores or
a dual-socket Xeon will reduce delay. A server startup failure is reported visibly;
there is no automatic restart or replay of a partially translated live session. For
CUDA load failures, stop and restart explicitly with `--device cpu`.

## Native engine builds

The Rust controller stays portable; CUDA lives in whisper.cpp and llama.cpp. Install
CUDA-enabled native engines and their required libraries, plus a compatible NVIDIA
driver. CUDA Toolkit/build tools are needed when compiling those engines. They require
no service account or subscription. Follow the upstream build documentation:

- https://github.com/ggml-org/whisper.cpp#nvidia-gpu-support
- https://github.com/ggml-org/llama.cpp/blob/master/docs/build.md#cuda

Both upstream projects support `-DGGML_CUDA=ON` CMake builds. When distributing across
these CPU families, disable host-specific compilation (`-DGGML_NATIVE=OFF`), avoid
`-march=native`/Rust `target-cpu=native`, and check the complete instruction-set flags
and dependencies of each chosen native build. Test on the older Xeon as well as the
modern Intel/AMD machines; a successful build on one host does not establish portability.
An RTX 4060 build alone also does not establish compatibility with every later GPU.

NVIDIA specifications: https://www.nvidia.com/en-us/geforce/graphics-cards/40-series/rtx-4060-4060ti/

## Acceptance measurements

For each CPU family, Windows/Linux and GPU VRAM size, record driver/engine versions,
model hashes, threads, peak system RAM/VRAM, recognition and translation accuracy,
first audible output delay, and whether queues stay bounded during 30 minutes of
continuous speech. Exercise mic and line input, silence/noise, stop/restart, busy GPU
and explicit CPU operation. Review Spanish/German with native speakers.

The development environment has no NVIDIA GPU; this update tests selection and command
construction only. Real-time latency and CUDA execution are unverified. Preserving the
speaker's timbre and expression still requires another offline synthesis/voice engine
and its own GPU budget; current Piper output uses standard Spanish/German voices.
