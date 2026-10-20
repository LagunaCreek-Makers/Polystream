# Ten-year General Conference evaluation

The text scope is April 2017 through October 2026: the 20 conferences in the last ten
conference years as of October 19, 2026. Source: the official Church archive at
https://www.churchofjesuschrist.org/study/general-conference?lang=eng .

## Evidence and limits

The completed collection contains 2,151 language pages for 717 conference text entries
in English, Spanish and German. This includes administrative addresses linked in the
conference indexes. One additional indexed entry, the October 2020 video presentation
at `/2020/10/33video`, has no page transcript and is explicitly excluded. It is not
silently counted as a successful text test.

There are 21,172 candidate aligned paragraph triplets, giving 84,688 directional cases
for en→es, en→de, es→de and de→es. Collection completeness means the eligible pages were
retrieved, parsed and hash-checked. It does not mean all paragraphs were semantically
aligned or translated by the model. These are separate stages.

The current real-model pilot uses one preselected talk from each conference and one
short paragraph from that talk, in all four directions. See the completed numerical
results under `reports/`. The pilot uses the actual Qwen3-4B Q4_K_M weights and local
llama.cpp b11739 on a CPU-only development host with four inference threads. It is
not an RTX 4060, Xeon E5, i7 or Ryzen hardware certification. Full-corpus inference,
bilingual accuracy review, ASR, voice quality and live acoustic latency remain separate
validation work. A small paragraph sample cannot establish accuracy over ten years.

## Prepare the corpus once

From the project directory, while connected to the internet:

```bash
python3 scripts/conference_corpus.py --start-year 2017 --end-year 2026 --output conference-corpus --download
```

Use `python` on Windows. The default is four download workers; `--workers 1` through
`--workers 8` are supported. The collector fetches only the official host, validates
language identity and document type, records provenance/hashes, and reuses verified
cached pages when resumed. It accepts both numbered and modern alphanumeric publisher
paragraph IDs. Figures, bylines and footnotes are excluded from the prose.

After preparation, rebuilding and checking the cache requires no internet:

```bash
python3 scripts/conference_corpus.py --start-year 2017 --end-year 2026 --output conference-corpus
```

A missing page, changed cached content, language fallback or unsupported layout produces
an explicit failure and a nonzero exit code. Coverage is recorded in `coverage.json`,
with partial progress in `coverage.partial.json`. An output lock prevents two new
collectors from writing the same directory at once. Ctrl+C stops further downloads;
in-flight requests can take their timeout to finish. Verified cache entries remain
available for resume. A stale lock should only be removed after its collector has ended.

The supplied source archive contains public provenance, hashes and aggregate results.
It does not bundle full talk texts, full translations, recordings or model weights.
Retain the official links and rights notices in any local copies. The app itself still
runs offline without an account or subscription. Corpus download and optional metric
installation are preparation steps and can occur on another computer for USB transfer.

## Alignment and dataset partitions

`aligned.jsonl` uses shared publisher paragraph IDs across all three languages. It
preserves source URLs, document hashes and an `alignment` field explicitly marking
these as unreviewed candidates. Matching IDs are useful evidence, but do not guarantee
that every translated paragraph has identical semantic scope. Translations may split,
combine, omit or revise content. The collector never invents missing references or
pairs paragraphs merely because their ordinal positions match.

Candidates require three distinct, nonempty language strings, each 15–2,000 characters
long. Paragraphs outside this range and unmatched IDs are outside the candidate set.
Longer passages need a reviewed alignment/segmentation process; truncating each language
at a character count would create unreliable references.

The partitions are whole conference years: 2017–2023 `train` (15,190 candidates), 2024
`dev` (2,046), 2025–2026 `test` (3,936). These labels reserve separate material for
future tuning and evaluation. **No model training or fine-tuning has occurred.** Public talks may have appeared in model pretraining; a year partition alone does not
prove independence from pretraining. Keep
future prompt/glossary selection on train/dev material and preserve a held-out evaluation.

## Run translation evaluations

Install the native engines/models and run the local inference engines as described in
`OFFLINE_SETUP.md`. Build the Rust runner:

```bash
cargo build --locked --example offline_benchmark
```

On Linux the binary is `target/debug/examples/offline_benchmark`; on Windows it is
`target/debug/examples/offline_benchmark.exe`. For performance comparisons, use the same
build profile in every arm; a release build is preferable on the target hardware.

Plan the complete candidate run without performing inference:

```bash
python3 scripts/run_conference.py --corpus conference-corpus --config offline.json --binary target/debug/examples/offline_benchmark --output conference-results
```

Append `--run` to execute. Omitting a paragraph limit selects every candidate paragraph.
The standard launcher now explicitly requests one translation slot and a 4,096-token
context, matching the single speech worker and the measured engine setup. Results are
processed in batches of 100 cases; there are 847 batches for the current
full corpus. Each batch reuses the long-running local model server. The runner skips
Piper and does not require installed voices in translation-only mode. It does not test
speech recognition or synthesis. Start only the translation server manually if you
want to evaluate without installing the other engines; the standard app launcher
checks the complete live installation.

Resume by running the identical command again. Successful case IDs are skipped, failed
cases are retried, and a partial trailing result line is ignored and retried. Changing
the corpus, model file, config, Rust binary, selection or timing-related options requires
a new output directory. Model/config/binary/corpus hashes are recorded in run identity.
The configured model must match the model actually loaded in the local server. Keep
hardware, engine arguments, driver, thread count and device mode constant when resuming;
the fingerprint cannot attest to unobserved changes in an independently started server.
Use a new output directory when those settings change. A batch
without successes stops the run so a broken engine does not generate hundreds of empty
attempts. The report retains failed attempts and unique completion counts.

The published pilot used the general profile. Copy `offline.json` to
`general-profile.json` and set `translation_domain` to `general` before reproducing
that pilot against the same corpus snapshot:

```bash
python3 scripts/run_conference.py --corpus conference-corpus --config general-profile.json --binary target/debug/examples/offline_benchmark --output conference-pilot --paragraph-ids evaluation/reports/pilot-paragraph-selection.json --max-chars 300 --run
```

Alternatively, `--limit-paragraphs 20` selects a deterministic conference-balanced pilot
from the full candidate corpus. That generic selection is different from the published
one-talk-per-conference pilot. `--split dev` or `--split test` filters the partition.
The 30-second production translation deadline is retained. Slow CPU runs can time out;
these are real failures, not successful translations. The full run can take substantial
compute time even on a GPU and has not been completed in this development environment.

## Score and review

Optional scoring dependencies are separate from the Rust app:

```bash
python3 -m pip install -r evaluation/requirements.txt
python3 scripts/score_conference.py --results conference-results --output conference-scores.json
```

For air-gapped scoring, download wheels on a matching connected system with
`python3 -m pip download -r evaluation/requirements.txt -d evaluation-wheels`, transfer
them and install with `--no-index --find-links evaluation-wheels`.

The report records SacreBLEU chrF2, its exact signature, case counts, successful-case
median/p95 translation times, cache hits and cases requiring review. chrF measures
character overlap with a published reference, **not a percentage of correct doctrine
or translation accuracy**. Legitimate interpretation can differ from the official
written translation. A sentence can reverse meaning while preserving most words.

Review flags highlight reference terminology missing in the output, digit differences,
possible omitted negation and large expansions. They are diagnostics rather than
error verdicts: inflection, paraphrase, number formatting and implicit negation can
produce false positives or negatives. Review the actual source, reference and output
with bilingual speakers, using the rubric in `README.md`. Investigate critical errors
regardless of aggregate chrF. Failure counts must accompany all latency results.

## Performance probes

`compare_conference_performance.py` runs two short 2024 development paragraphs through
fresh local engines in interleaved and language-grouped orders. It also compares four
exact repeats with the in-memory translation cache disabled/enabled. It assigns a free
loopback port to each owned child engine, cleans up its child engines and writes a numerical report.

```bash
python3 scripts/compare_conference_performance.py --corpus conference-corpus --config general-profile.json --binary target/debug/examples/offline_benchmark --output conference-performance-probes --threads 4
```

The published timing probe used the general profile. This defaults to CPU (`--gpu-layers 0`). To request GPU offload on an appropriately
configured CUDA engine, explicitly pass `--gpu-layers 99`; inspect the engine log to
verify actual offload. A single short probe is not a stable throughput benchmark.
Compare engine versions, model hashes and hardware, and repeat measurements under a
controlled load before drawing hardware conclusions.

The app now caches up to 64 exact completed translations per engine/session, shared
across its worker clones. Keys include source language, target language, exact text,
local model path and port. Only successful, complete results are cached. Stop remains
effective on a cache hit. New sessions start fresh; no cache is written to disk. Restart
sessions when changing models/prompts manually. Different wording, punctuation or
negation produces a different key. Audio is synthesized for each request.

The corpus simulation found fewer than 1% of source paragraphs repeating within a
64-entry cache. This is a useful optimization for exact repeats and rehearsals; it
cannot make novel CPU inference real-time. Bulk evaluation defaults to cache disabled
so repeated text does not inflate reported inference speed. `--cache` is available for
an explicitly labelled cache-enabled experiment. See `reports/` for the measurements
and limitations actually established in this release.

## Religious terminology profile

The supplied `offline.json` selects `translation_domain: "latter_day_saint"`. This adds
Latter-day Saint context and a small directional glossary. It preserves the existing
instructions about names, numbers, negation and treating input as content, and asks the
model to avoid adding doctrine or completing quotations from memory. The glossary is
conditional on the Church sense, rather than forcing a religious reading of ordinary
literal uses. `general` retains the original prompt; legacy configs without the field
also use general mode. Cache keys include the selected profile.

The general-profile pilot surfaced terms requiring review, including Spanish sacrament
and stake-conference wording. A targeted terminology check compares an observed 2022
training-set passage and a short 2024 development passage with their saved baseline
outputs. This is a regression check, not independent held-out evidence of improved
accuracy across ten years. See `reports/conference-domain-summary.json` for completed
results and failures. Bilingual reviewers still need to judge meaning and doctrine.

To reproduce that targeted comparison after running the general-profile pilot, use a
config copy with `translation_domain` set to `general`, then:

```bash
python3 scripts/compare_conference_domain.py --baseline-results conference-pilot --config general-profile.json --binary target/debug/examples/offline_benchmark --paragraph-ids evaluation/reports/domain-paragraph-selection.json --output conference-domain-check
```

The check starts and stops its own native translation engines. It uses CPU by default;
`--gpu-layers 99` requests offload on an appropriate CUDA build. Its saved baseline and
updated-profile timings were collected separately and must not be treated as a controlled
speed comparison. Any model selection or future prompt tuning should retain separate
held-out material and be followed by live speech/audio validation.
