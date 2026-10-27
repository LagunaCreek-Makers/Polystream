# Arabic/Hindi and English ASR development diagnostics

Scope: actual failed source clauses from the 900-second priority-eight rehearsal, examined as development cases. No held-out quality claim, no production prompt/model change, and no automatic promotion of experimental languages. All owned engines were cleaned up.

## Translation comparison

Twelve failed speech clauses: nine Arabic and three Hindi. The original 1,024-token, temperature-zero, non-thinking settings were retained. Requests, prompts, raw text and finish reasons are saved in results.jsonl.

| Prompt | Arabic introduced-Cyrillic rejections / 9 | Hindi incomplete outputs / 3 |
|---|---:|---:|
| Current English instructions and full glossary | 9 | 2 |
| Concise English instructions, no glossary | 5 | 0 |
| Target-language instructions, no glossary | 2 | 1 |

The third original Hindi failure completed under the diagnostic repetition, but its current-prompt output was contaminated by an unrelated narrative/glossary term. Several other outputs that passed mechanical guards still changed meaning. Thus reducing rejection counts does not establish accurate translation. Current Hindi prompts caused long repetition of unrelated glossary phrases on two cases; removing the glossary avoided those token-limit failures in this sample. This supports testing source-selected terminology rather than including every term on every clause, but does not by itself validate that change.

Arabic outputs included Cyrillic embedded within Arabic words, even for short fragments and names. Native Arabic instructions improved the narrow script count, but did not eliminate errors. Do not rewrite characters to make incorrect wording pass the validator, disable the guard, or substitute names guessed from a reference. Existing clause-local English markers remain the correct fallback.

## English recognition comparison

Original PCM clips were compared without text substitution: Oaks 518.34–519.90s, Kearon 598.38–599.74s, Soares 595.50–598.10s. One ordinary-speech clip and four seconds of generated silence served as negative controls. The original supplied caption file is meeting-stress/captions.srt.

| Audio | No hint | Domain-name hint |
|---|---|---|
| Oaks | Dallin Harris Oaks. | Dalin Harris, Oaks. |
| Kearon | Patrick Kiran. | Patrick Kearon. |
| Soares | Ulysses Suarez. | Ulisses Soares. |

The hint corrected two names but worsened the first. Spelling the full spoken name in the hint did not fix that regression. Ordinary speech and the silence marker were unchanged in these two controls. This is too small a sample to establish hallucination safety; do not silently ship the hint as a universal correction.

Correction to earlier development framing: Dallin Harris Oaks matches the supplied source captions and is not an ASR error. Do not shorten it to an initial or rewrite correct recognition. The earlier failed output remains preserved as original evidence.

Official Church reference spellings were checked against https://www.churchofjesuschrist.org/study/general-conference/2024/04?lang=spa and https://news-in.churchofjesuschrist.org/leadership-and-organization . These pages support spellings, not a claim about the meeting recording's date or current office holders. The supplied recording captions establish the examined utterances. whisper.cpp's documented server implementation accepts a multipart prompt field: https://raw.githubusercontent.com/ggml-org/whisper.cpp/master/examples/server/server.cpp . The local installed server behavior was measured directly.

## Next implementation gate

Test source-selected glossary terms on the known name/narrative cases plus untouched ordinary fragments, and compare a better-supported Arabic translation model before promotion. For ASR, evaluate longer context and a larger recognition model on the same original clips plus held-out speech; a list-of-names hint alone has a demonstrated regression. This is a targeted implementation need, not permission to mark unreviewed translations accurate. User-approved configurable engine workflows remain deferred; no new model was downloaded or installed during these diagnostics.
