# Windows automatic setup

Double-click `Install-Windows.cmd` in the project folder. It installs the dependencies
for the existing compiled application on Windows 10/11 x64:

- Python 3.12 through winget if Python 3.9 or newer is not available.
- Microsoft's signed Visual C++ x64 runtime (Windows may request administrator approval).
- Official whisper.cpp and llama.cpp Windows CPU packages and native Piper 2023.11.14-2.
- The existing pinned Whisper, Qwen3, Spanish voice, German voice and attribution files.

Allow about 6 GB of free disk space and an internet connection for setup. The models
alone download about 2.79 GB. No account or API key is needed. Keep the project folder
in its installed location; rerun setup after moving it to refresh the Python launcher.

After setup, double-click `Start-Translation.cmd`. This starts the engines and compiled
application together in CPU mode. Keep the console open. CPU translation can be slow;
GPU builds and driver installation are not included in this installer. Run the existing
voice checks and native-speaker translation evaluation before event use.

Setup selects the newest matching official CPU release among the first 50 releases,
because upstream stable tags may contain no binaries. You can select explicit versions:

```powershell
.\scripts\install-windows.ps1 -WhisperTag YOUR_TAG -LlamaTag YOUR_TAG
```

Engine packages are checked against the GitHub asset size and SHA-256 digest when
published. An observed hash, source URL and release are saved in each engine folder's
`installation.json`; an observed hash alone is not independent integrity verification.
The model pack always requires the repository's pinned hashes and sizes.

Rerunning setup reuses existing engine directories and verified models. It does not
upgrade engines. Move an engine directory aside to reinstall it. A conflicting model
hash stops setup; review and move that file aside before retrying. Interrupted model
downloads restart the affected file. The original configuration is retained once as
`offline.json.before-installer`; setup updates only the three engine paths.

Preview without changing anything:

```powershell
.\scripts\install-windows.ps1 -Plan
```

Use `-SkipRuntime` only when a suitable Visual C++ runtime is already installed.
If winget is unavailable, install Python 3.9+ manually and rerun. No Rust compiler,
Visual Studio build tools, CUDA toolkit or cloud services are required for setup of
an already compiled application. Existing Linux setup remains in `OFFLINE_SETUP.md`.

## NVIDIA GPU setup

Run `Install-GPU.cmd` after CPU setup. This installs official CUDA 12.4 engine
packages in separate `engines/whisper-cuda` and `engines/llama-cuda` directories,
including the matching llama runtime DLL package. It writes `offline.cuda.json`
and `Start-Translation-GPU.cmd`, preserving the CPU configuration and launcher.
An NVIDIA driver must already be installed. This does not install or upgrade drivers.
Actual inference is required to verify compatibility, especially on newer GPUs.
The GPU launcher warms up both local inference engines before launching the GUI.
First-use CUDA kernel initialization may take several minutes; warm-up has a
five-minute request limit, separate from the app's 30-second translation deadline.

Run `Start-Translation-GPU.cmd` for GPU inference; `Start-Translation.cmd` retains
CPU inference. If the GUI configuration is changed, keep ports, model paths and voices
consistent with the configuration passed to the engine launcher.

## Actual Windows diagnostics

Build the existing diagnostic executables once with `cargo build --locked --examples`.
Then run `Diagnose-Windows.cmd` or `Diagnose-GPU.cmd` with the application and other
engine launchers stopped. Each diagnostic creates a new results directory, synthesizes
an English input fixture using Windows' English voice, tests both Piper voices, then
tests typed and audio English input through the app's Rust adapters into Spanish and
German. It saves WAVs, engine logs, timings and `diagnostic.json`. No microphone is
recorded, and a passed test does not establish listening quality or semantic accuracy.

For an extended uncached check, pass `--repeat 5`. For a human recording, pass
`--audio "C:\path\recording.wav"` (PCM16, mono, 16 kHz). Microphone capture, playback
routing and human listening remain separate manual checks in the desktop app.
