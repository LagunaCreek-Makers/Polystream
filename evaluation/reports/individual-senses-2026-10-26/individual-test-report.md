# Individual Arabic/Hindi sense tests

User direction, October 26, 2026: evaluate additional languages individually, correct errors where possible, and bypass blocked languages. Multi-stream tests are for separate capacity work, not required for each new language. All tests here used one language at a time; owned engines were started and cleaned up sequentially. No application code, default model or prompt was changed by this experiment.

108 direct text requests: nine fixed development cases, two prompt variants, six separate runs (Arabic/Hindi each with 4B and 8B; then Arabic/Hindi with a context-selected glossary under4B). These are not held-out tests and do not measure recognition, audio quality or live delay. HTTP/output completion is not semantic correctness.

## Cases

Musical organ fragments, playing an organ, bodily organ, fifteen dinner plates, Nephi's writing plates, Mormon's historical account, financial bank account, negation, and meeting time. Raw English source, prompt and translation are preserved in each results.jsonl.

## Findings

Adding disambiguating instructions at the end of the full religious glossary improved the instrument noun in some cases, but other results introduced absent names or retained the wrong ordinary sense. A selected glossary removing unrelated writing/name entries improved dinner plates to dishes in Arabic, but the4B model changed fifteen to seventeen. Hindi's selected glossary improved the dish noun but still changed dinner to all-day and introduced a new bodily-organ error. Other Hindi results mishandled the subject/verb of playing an instrument.

The8B context-hint tests also left dinner plates as writing boards in both languages. More completed responses and fewer script errors are not enough to ship these prompts. The narrative/financial distinction and negation often survived, but that does not establish all-case or native grammatical accuracy. Keep the existing experimental status and fallback behavior; do not replace the current prompt with these candidates.

## Official terminology provenance

The musical instrument term was checked against the same handbook chapter in English, Arabic and Hindi:

- https://www.churchofjesuschrist.org/study/manual/general-handbook-selections/19-music?lang=eng
- https://www.churchofjesuschrist.org/study/manual/general-handbook-selections/19-music?lang=ara
- https://www.churchofjesuschrist.org/study/manual/general-handbook-selections/19-music?lang=hin

Arabic الأرغن and Hindi ऑर्गन refer to the keyboard instrument in that context. This verifies the noun, not the surrounding generated grammar. Ordinary dish terms and control sentences are development test material, not claims of official paired quotation or native review. Existing scriptural plates/name terminology remains a separate religious sense.

## Next rollout work

Keep Arabic and Hindi available experimentally, with these semantic issues recorded. Do not spend the next language rollout cycle repeatedly stress-testing eight streams. Continue individual quality/speech validation for the other available languages, beginning with French, while leaving unsupported voices/backend gaps explicitly deferred. Any future capacity benchmark remains a separately labeled task. Native review is still required before calling a language production-accurate.
