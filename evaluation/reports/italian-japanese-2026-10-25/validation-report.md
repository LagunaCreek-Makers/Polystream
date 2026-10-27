# Italian / Japanese optional Piper validation

100.702-second shared English title-page development clip; Whisper Base, Qwen3 4B Q4_K_M, one GPU translation slot. Piper 1.8 CPU synthesis. Native-speaker/semantic/pronunciation review remains pending. Timing is source phrase start to first queued PCM, not physical acoustic latency.

| Runtime | Language | Clauses | Median (s) | p95 (s) | Maximum (s) |
|---|---|---:|---:|---:|---:|
| Per-phrase process baseline | it | 38 | 74.87 | 142.34 | 145.53 |
| Per-phrase process baseline | ja | 38 | 67.93 | 132.77 | 136.14 |

Per-phrase process baseline: backlog drain 144.58s; 38 aligned English/Italian/Japanese SRT cues. No cutoff/retry and no unmeasured input in either completed attempt.

| Persistent worker; 4 CPU threads/voice | it | 38 | 3.40 | 5.47 | 6.85 |
| Persistent worker; 4 CPU threads/voice | ja | 38 | 4.71 | 11.44 | 11.46 |

Persistent worker; 4 CPU threads/voice: backlog drain 11.50s; 38 aligned English/Italian/Japanese SRT cues. No cutoff/retry and no unmeasured input in either completed attempt.


The new path both reuses the loaded voice and sets four ONNX CPU threads per voice. This comparison does not isolate their separate contributions and is not a sustained-session guarantee. Existing native Piper voices are unchanged.

The shared speech replay predates the final source-gated Japanese alias correction. Targeted final-adapter name tests are stored separately in italian-japanese-terminology; do not claim the full corrected transcript was replayed unless separately measured. Initial Japanese output contained a Nephi variant; source-gated normalization replaces only observed variants when English input explicitly names the corresponding person/people, in the Church domain. It does not fill missing quotations or establish general accuracy.

Official Japanese reference: https://www.churchofjesuschrist.org/study/scriptures/bofm/bofm-title?lang=jpn . Italian reference: https://www.churchofjesuschrist.org/study/scriptures/bofm?lang=ita . These clips are development material, not held-out evaluation.

Italian Serena dataset card: CC BY 4.0. Japanese Hi-Fi-Captain dataset card: CC BY-NC-SA 4.0; optional experimental pack, separate from default distribution. Modern Piper: OHF-Voice/piper1-gpl v1.8.0; package and third-party notices retained in the isolated environment.
## Final checks

Eight final-adapter Italian/Japanese text-to-speech cases completed with audio and passed targeted name substring checks. Full 38-phrase replay is mechanically complete but predates final Japanese alias normalization; no claim of full corrected-transcript or native review. All 48 Rust tests, 10 launcher tests, Clippy all targets with warnings denied, and desktop build passed. Optional installers verified all model checksums and existing runtime imports; fresh runtime setup was exercised with bundled Python 3.12, not every Python installation.
