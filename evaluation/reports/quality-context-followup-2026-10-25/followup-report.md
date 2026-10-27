# Source-selected glossary and longer-context recognition follow-up

Development experiment, October 25, 2026. Application defaults and production source are unchanged. All experiment-owned engines stopped normally. Previous successful timing tests are not accuracy certification.

## Translation: 28 requests

Twelve original Arabic/Hindi failed speech clauses plus eight development controls per language. Only explicit Book of Mormon, Mormon and Nephi name entries were injected when present in the source. Ordinary fragments received concise translation instructions without unrelated glossary entries. This avoided the three original Hindi output-limit failures in this sample, but failed semantic controls. Arabic still introduced Cyrillic on five of the original nine Arabic clauses and on the fifteen control. Retain the existing rejection/fallback behavior.

Removing too much context changed Hindi writing plates to leaves and the historical account to a financial account; Arabic also used the financial sense of account. Number controls failed in both languages. A response can complete in the right script yet be wrong. Do not adopt this candidate based on token completion or script checks alone.

quality-gates.json records limited name, narrative, number and negation surface-form checks. These assertions intentionally expose observed regressions; they are not a general translator or native-speaker certification. Unknown/unchecked criteria remain null rather than being counted as passed. These are development controls, not held-out evaluation.

## Recognition: original audio, longer context, larger model

Base-model context comparison: 9 seconds around Oaks, 11 seconds around the name list, 9 seconds of ordinary speech, the earlier 4-second ordinary control, and generated silence. Longer context alone did not fix the target names. The name hint improved Kearon/Soares but still misspelled Oaks and changed other names in the list.

Downloaded public ggml-small.en.bin into the separate test workspace using hf download. No installation/default model change. Model bytes and SHA-256 are recorded in model-evidence.json. Eight requests without hints and eight with hints compared the three short name clips, two longer name contexts, two ordinary controls and silence.

Whisper small.en without hints yielded Alan Harris-Oakes, Patrick Kieran and Ulysses Suarez on the short clips. With hints it produced Dalin Harris Oaks, Patrick Kearon and Ulisses Soares, but degraded the longer name list and added “food, and food” to the ordinary short control. Silence remained BLANK_AUDIO. These results do not justify replacing the base model or enabling a name hint globally. Neither model/context setting is promoted on these development cases. Request times are engine-only warm-run samples, not full live translation latency or capacity.

## Decision and remaining implementation

Keep the existing models and fault-tolerant untranslated markers. Do not patch recognized names by matching a known meeting transcript. Next compare a stronger multilingual text model on these fixed controls and the existing correct glossary/name cases; for ASR, test another recognition candidate with the same controls before expanding to held-out talks. The configurable workflow architecture remains a later task; this experiment uses the existing Whisper/llama stack.
