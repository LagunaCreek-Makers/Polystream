# Offline evaluation for religious speech

The target setting is religious talks and meetings. Tests cover literal scripture,
original conversational sentences, English input and Spanish/German input/output.
This is an evaluation corpus, not a fine-tuning dataset or a claim of Church endorsement.

## Bundled material

`religious-parallel.json` contains 13 aligned passages in English, Spanish and German:

- Five short official excerpts: 2 Nephi 2:25, Moroni 10:5, Doctrine and Covenants 6:36,
  John 11:35, and Articles of Faith 1:2. Each language entry records the official page
  URL, verse anchor and SHA-256 of the extracted UTF-8 text; the file records retrieval
  time. Verse numbers and footnote markers were removed; punctuation was retained and
  whitespace normalized. Each excerpt is at most 25 words per source page.
- Eight original meeting sentences with **draft**, non-official Spanish/German references.
  These cover sacrament, covenants, Atonement, Holy Ghost, priesthood, bishop, ward,
  stake conference, scripture citations, time/numbers and nested negation. Bilingual
  reviewers should approve or improve the draft references before scoring models.

Official texts belong to their respective rights holders and are excluded from the
application's MIT license. The application does not redistribute full books, talks,
recordings or a scripture audio library. The bundled short excerpt set works offline.
The URLs are provenance, not runtime dependencies. Do not label original test sentences
as official Church wording.

The four evaluated directions are en→es, en→de, es→de and de→es: **52 text cases**.
The previous general-purpose eight-case set remains `spanish-german.json`.

## Run the checks

After installing the local engines and model pack, from the project directory:

```bash
python3 scripts/check_offline.py
python3 scripts/fetch_models.py --verify
python3 scripts/build_evaluation.py --output evaluation/religious-cases.local.json
python3 scripts/start_offline.py --device auto
```

In another terminal, with the engines running:

```bash
cargo run --locked --example offline_benchmark -- offline.json evaluation/religious-cases.local.json results-religious-01
```

Use `python` on Windows. Output directories must be new; their parent must exist. The
runner refuses existing directories and writes per-case results incrementally. Individual
case failures are recorded and the remaining cases continue; any failure gives a nonzero
exit code after writing the summary. A configuration/corpus/output-path error can stop
before a report is created. No network connection is needed beyond local loopback.

The report contains recognized/input text, translated text, references, review focus,
reference URLs, local WAV filenames, stage timings and errors. Summary p50/p95 values
use successful cases only and include the first case. Review failure counts together
with timing: excluding failures can make a poor model appear deceptively fast.

The preflight checks file presence, executable permissions, model header signatures,
voice sample-rate metadata and configuration shape. They do **not** run models,
validate ONNX graphs, check dynamic libraries, prove CUDA support or verify hashes.
`fetch_models.py --verify` performs the separate full pinned-pack hash check. The
launcher now performs structural preflight before spawning its inference engines.

## Recognition and spoken delivery

Text cases exercise translation and TTS. They do not exercise ASR. To test recognition,
have speakers record the source passages in all three languages as PCM16 mono 16 kHz
WAVs. Name files `PASSAGE_ID-en.wav`, `PASSAGE_ID-es.wav`, `PASSAGE_ID-de.wav`, using IDs
from the parallel file. There are 39 recordings for the supplied set; English audio is
reused across its two target languages. Keep each recording between zero and 60 seconds
(exclusive of zero); the runner also caps input WAV files at 2 MiB. Add natural pauses,
regional accents and a separate noise/reverberation test set. Do not use synthetic speech
as the only ASR test: it misses the behavior of real speakers in the room.

```bash
python3 scripts/build_evaluation.py --recordings /absolute/path/to/recordings --output evaluation/religious-audio.local.json
cargo run --locked --example offline_benchmark -- offline.json evaluation/religious-audio.local.json results-religious-audio-01
```

The runner strictly rejects other formats instead of silently misreading audio. It sends
whole recordings as clauses, rather than exercising the live gate, physical audio devices
or segmentation. Measure real mic/line sessions separately, including passages longer
than four seconds, variable speaking rates, sentence breaks and 30-minute continuous use.

## Interpret the results

`first_pcm_ms` measures submission of an already collected clause/text to the first
PCM received from Piper. It includes recognition when audio is supplied, translation,
Piper process/model startup and initial synthesis. It excludes collecting the input,
engine startup before the run, physical playback buffers and acoustic travel. It is
**not** microphone-to-speaker latency. `processing_ms` includes complete synthesis but
excludes report/WAV disk writes. `processing_to_input_ratio` compares total processing
time with input-recording duration; it is not a live pipeline throughput guarantee.

The references support human evaluation rather than exact-string grading. Official
scripture may have established language, historical style and translator choices;
more than one rendering can faithfully convey the meaning. Do not automatically replace
spoken content with memorized scripture: a speaker may paraphrase, qualify, misquote or
say a different verse. Keep scripture quotations and original/paraphrased speech as
separate evaluation categories. This initial five-excerpt set is a smoke set and is far
too small for accuracy estimates or deployment certification.

For each case, review the source, reference, actual translation and output WAV. Rate:

| Dimension | Review question |
|---|---|
| Meaning | Were all claims, conditions and relationships preserved? |
| Doctrine/terminology | Are Church-specific terms appropriate in context? |
| Negation | Did "not", "never", exceptions or qualifications change? |
| Entities | Were names, chapter/verse citations, dates and numbers preserved? |
| Speech | Is pronunciation intelligible, with suitable pacing and sentence emphasis? |

Use a 0–4 scale (0 wrong/unintelligible, 1 major errors, 2 partly adequate, 3 minor issues,
4 faithful/clear). Mark changed doctrine, reversed negation, invented quotations or
changed scripture references as critical errors regardless of average score. Have
independent bilingual reviewers resolve disagreements. Current standard Piper voices
do not preserve the source speaker's timbre or expression.

Record CPU/GPU/RAM, driver/engine versions, model hashes, CPU thread settings, device
mode, build profile and whether this is a first or repeated run alongside each report.
Compare like-for-like builds; debug timings should not be presented as release benchmarks.
No model-backed accuracy or target-hardware performance results are bundled yet.

## Extend the set

Add aligned entries to the parallel file with a unique ASCII ID, category, reference,
three language texts and review-focus notes. For external material retain authoritative
URLs and version/retrieval metadata. General conference talks, prayers and longer Book
of Mormon passages should be added from text/audio you are permitted to use, aligned by
meaning rather than blindly pairing paragraph numbers. Maintain a held-out set of modern
religious speech that is not used to tune prompts or choose glossary entries; familiar
scripture alone can overestimate performance. Corpus changes invalidate the old checksum
unless it is deliberately recomputed and provenance updated. Builder output never
replaces existing files.

## Ten-year General Conference extension

See **GENERAL_CONFERENCE.md** for the full 2017–2026 text collection, the published
80-case real-model pilot, domain terminology checks and resumable exhaustive evaluation.
The original 52-case scripture/meeting set remains a small offline regression corpus.
