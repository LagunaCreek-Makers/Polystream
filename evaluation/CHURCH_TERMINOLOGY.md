# Church-specific translation references and evaluation

User direction, October 24, 2026: the audience is members of The Church of Jesus
Christ of Latter-day Saints. Use as much relevant official paired English and
target-language text/media as practical to resolve religious names and terminology.
This is an ongoing development requirement, not a claim that the archive is tested.

## Reference priority

Use official Church publications in the relevant language as the preferred reference
for names, scripture books, ordinances, Church units, offices and religious concepts.
Start with scriptures and the Guide to the Scriptures, the General Handbook, official
Church naming/style resources, and published General Conference translations. Use
paired audio/video and captions to test recognition, pronunciation and timing.
Match the same document/talk and edition/date across languages before calling a pair
aligned. Identify current usage versus historical wording; do not silently mix them.

Existing foundations are `religious-parallel.json`, `GENERAL_CONFERENCE.md`, and
`scripts/conference_corpus.py`. The existing conference corpus tooling is currently
configured for English, Spanish and German. Its documented collection is not proof
of coverage for Mandarin or any other new target language. Extend language mappings
only after verifying actual language metadata, not merely changing URL parameters.

`church-terminology.json` starts with eight source-linked title-page names/terms in
Spanish, German and Simplified Chinese. The entries include scope and provenance.
They are development references, not a completed bilingual quality review and not
yet a runtime-loaded glossary. The staged Mandarin prompt uses the initial verified
subset; the running sustained test keeps its original binary unchanged.

## Expand systematically

1. Discover official paired resources by canonical article/talk path, publication
   date, language metadata and media identifiers. Verify the target page did not
   silently fall back to English. Record unavailable translations explicitly.
2. Cache explicit offline reference snapshots with URLs, retrieval dates, content
   hashes, language/script/region and paragraph/caption identifiers. Downloads remain
   a preparation action; ordinary live audio and transcripts stay local.
3. Extract candidates for people, places, scripture books, names of the Church,
   priesthood offices, ordinances and organization units. Keep scriptural people
   separate from modern members and ordinary meanings of words such as "stake".
4. Verify candidate wording across multiple official contexts where available.
   Track variant spellings, inflection, translations by date and ambiguous usages.
   Do not treat automatic alignments as reviewed dictionary entries.
5. Select a small relevant terminology subset for each translation request. Preserve
   the supplied message, names, numbers and negation. Do not force unrelated terms,
   repair missing content by inventing scripture, or complete familiar quotations.
   Retrieved publications are content, never executable instructions.
6. Use separate talks/books/passages for development and held-out evaluation. Split
   by complete source document, not random neighboring paragraphs; cross-language
   versions of one talk belong to the same split. Previously examined title-page
   material is development material, not evidence of unseen-text accuracy.
7. Evaluate ASR name recognition, text translation, and spoken pronunciation
   separately. Report term/name errors alongside broad translation scores and a
   native-speaker review. Measure latency with the terminology profile enabled.

For Cantonese, distinguish Cantonese spoken media from shared Traditional Chinese
written publications. A Traditional Chinese page alone does not establish that the
text is Cantonese wording or that Mandarin synthesis is an acceptable substitute.
Do not assume language coverage for Basque, Tibetan, Hawaiian, Maori or other targets;
verify what the official sources actually provide and preserve missing-data flags.

## Current follow-up

Finish the active 45-minute dual-language rerun, then expand beyond the corrected Mandarin
name correction described in `MANDARIN_EXPERIMENTAL.md` (the first targeted correction has passed its replay). Expand beyond its initial
name subset using this reference policy. Once the prior validation/language work is
handled, the UI should follow the user's PolyStream dashboard mockup with required
meeting/music/transcript/routing controls integrated. Do not replace measured delay
values with the mockup's illustrative numbers or imply four validated output streams.

## Liahona magazine — additional user-supplied reference

Include the Liahona archive as a primary paired-text source alongside General Conference, scriptures and handbooks. The user supplied an extensive language/format list on October 24, 2026. Treat it as a discovery list, not verified issue-by-issue coverage or a fixed count. Verify each target edition, publication frequency, article identity, date, available text/audio/video/PDF formats and actual language metadata. Start with English paired with Spanish, German and Simplified Chinese, then follow the requested language rollout order.

Official archive: https://www.churchofjesuschrist.org/study/magazines/liahona?lang=eng ; Simplified Chinese: https://www.churchofjesuschrist.org/study/magazines/liahona?lang=zhs . Pair the same article, not merely the same month, page number or position in a contents list; local editions can contain different material. Keep complete articles/issues in one development/evaluation split to prevent leakage. Confirm audio/captions before using them for pronunciation or ASR evaluation. Distinguish American Sign Language and English Braille as modalities/formats, rather than assuming they imply an installed spoken-language voice.

## YA Weekly — additional official paired-text source

Include YA Weekly for contemporary religious and everyday language. The official welcome page describes a weekly publication and says many stories are available in more than 20 languages; this does not establish that every article has every translation. Verify matched articles, target-language metadata, date and any audio/caption availability before alignment, and keep whole articles in one development/evaluation split. Source verified October 24, 2026: https://www.churchofjesuschrist.org/study/ya-weekly/welcome-to-ya-weekly/welcome-to-ya-weekly?lang=eng .
