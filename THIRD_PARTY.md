# Third-party engines and models

The Rust application's MIT license does not replace engine/model/voice licenses. This
source package does not redistribute the native engines or model weights.

| Component | Source / license information checked |
|---|---|
| whisper.cpp and Whisper model conversion | https://github.com/ggml-org/whisper.cpp — MIT |
| llama.cpp | https://github.com/ggml-org/llama.cpp — MIT |
| Qwen3-4B-GGUF | https://huggingface.co/Qwen/Qwen3-4B-GGUF — Apache-2.0 |
| Legacy native Piper | https://github.com/rhasspy/piper — MIT project; preserve bundled dependency notices |
| Maintained Piper successor | https://github.com/OHF-Voice/piper1-gpl — GPL-3.0 project, not the same license as legacy Piper |
| Spanish SHARVARD medium voice | MODEL_CARD identifies SHARVARD dataset and CC BY 3.0; attribution required by that license |
| German Thorsten medium voice | MODEL_CARD identifies Thorsten dataset and CC0 |

Voice cards (included by the model download helper):

- https://huggingface.co/rhasspy/piper-voices/blob/main/es/es_ES/sharvard/medium/MODEL_CARD
- https://huggingface.co/rhasspy/piper-voices/blob/main/de/de_DE/thorsten/medium/MODEL_CARD

The repository-wide Piper voices label is not a substitute for each voice's MODEL_CARD
and dataset terms. Retain model cards, source references, license texts and applicable
attribution when preparing a distributable model pack. The pinned inventory is recorded
in `models-manifest.json`; setup uses public downloads without a subscription.

## Evaluation texts

`evaluation/religious-parallel.json` includes five short official scripture excerpts in
three languages from The Church of Jesus Christ of Latter-day Saints. Each entry retains
its authoritative URL and excerpt hash. These third-party texts are not covered by the
application MIT license. The remaining meeting sentences/references are original test
material, not official Church translations; bilingual review remains required.

## General Conference evaluation

The optional collector accesses official General Conference text pages in English,
Spanish and German for local evaluation. Full talks, generated full translations and
recordings are excluded from this source distribution. Public provenance, hashes and
aggregate metrics are included under evaluation/reports. These source texts are not
covered by the application MIT license. SacreBLEU 2.6.0 is an optional evaluation-only
dependency; retain its upstream notices and those of its dependencies when distributing
an offline evaluation environment. It is not required by the Rust runtime.
