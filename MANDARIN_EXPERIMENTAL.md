# Mandarin Chinese — experimental local evaluation

English-to-Mandarin translation, local synthesis, original-timeline SRT export and
per-language delay measurement are implemented for evaluation. Mandarin is an output
language only: Mandarin speech recognition has not been enabled. Cantonese remains
a distinct, unavailable route; it is never substituted with Mandarin.

The existing Qwen3 translation model is asked to use Simplified Chinese. A Windows
installed CJK font (Microsoft YaHei or SimSun) is used as a fallback when available;
Linux may use an installed Noto Sans CJK font. Operating-system fonts are not bundled.
The full UI layout still awaits the user's mockups and interactive visual validation.

## Separate optional voice pack

The local evaluation uses native Piper's compatible Huayan medium voice, pinned to
`rhasspy/piper-voices` revision `c10ece1aade47bb51c153c893d14e5bf8e5b7117`. The model,
configuration and model card are separately checksummed in
`models-mandarin-experimental.json`. This is not part of the default model pack.

The upstream Huayan model card lists the dataset license as **Unknown**. Do not treat
this as a cleared redistributable production voice. The newer Chaowen model card lists
a CC0 dataset, but its `pinyin` phonemizer is incompatible with the installed native
Piper version; it cannot safely be loaded as an ordinary eSpeak voice. A production
Mandarin pack needs a compatible backend/voice with resolved licensing and quality.

Primary model metadata:

- https://huggingface.co/rhasspy/piper-voices/blob/c10ece1aade47bb51c153c893d14e5bf8e5b7117/zh/zh_CN/huayan/medium/MODEL_CARD
- https://huggingface.co/rhasspy/piper-voices/blob/5041a7a49ae5b3a0f30e6837769eb03fdbb231bd/zh/zh_CN/chaowen/medium/MODEL_CARD
- https://huggingface.co/rhasspy/piper-voices/blob/5041a7a49ae5b3a0f30e6837769eb03fdbb231bd/zh/zh_CN/chaowen/medium/zh_CN-chaowen-medium.onnx.json

On this development machine the optional files are installed and verified. The
separate `offline.mandarin.experimental.json` preserves the ordinary configuration.
For an explicit fresh evaluation setup, run `Install-Mandarin-Experimental.cmd`.

Launch:

```text
Start-Translation-GPU.cmd --config offline.mandarin.experimental.json
```

The launcher now passes the selected configuration to the GUI, which previously
defaulted independently to offline.json. Choose Mandarin Chinese (experimental) as
an output and keep English as the input. A Mandarin-plus-Spanish/German pair uses
the same two-output architecture, but that particular pair has not yet been tested.

## Initial mechanical replay result

The 100.702-second title-page recording completed all 38 accepted phrases, with 38
English cues and 38 Mandarin cues. Both SRT files parsed with FFmpeg and had identical
source timestamps. Initial median/p95/maximum source-to-first-queued-PCM delay was
4.74/5.66/5.66 seconds; playback drained at 106.018 seconds. This was single-target
Mandarin, not simultaneous Spanish/German/Mandarin or a physical audio-device test.

Evidence:

`C:\Users\Paul Lott\Documents\New project\dual-development\mandarin-results-20261024-094414\replay`

This is a pipeline pass, **not a full semantic-quality pass**. The initial text contained incorrect religious names and retained the English word "Plates". The follow-up correction has now been applied and tested, using the Church's Simplified Chinese title-page terms while prohibiting completion of quotations from memory: https://www.churchofjesuschrist.org/study/scriptures/bofm/bofm-title?lang=zhs

## Post-correction replay

All 38 phrases completed again. English and Mandarin SRT files each contain 38 source-aligned cues and parse with FFmpeg. The first title now contains 摩尔门经; the author cue uses 摩尔门 without mistranslating him as a modern member; the next cue uses 尼腓 and 页片 without retaining "Plates". These targeted checks passed. Other recognition, translation and pronunciation errors may remain; native-speaker review is still pending.

Post-correction median/p95/maximum first-queued-PCM delay: 4.99/6.11/6.49 seconds. Wall time 105.133 seconds for the 100.702-second input. Evidence: `C:\Users\Paul Lott\Documents\New project\dual-development\mandarin-results-20261024-101653\replay` (including subtitle-validation.json and mandarin-sample.wav). Do not confuse these results with the earlier pre-correction timings.

The language registry also records the remaining requested languages in priority
order. They remain disabled pending their own voice, translation and validation work.

For the expanded user-directed official Church reference policy and source-linked seed glossary, see evaluation/CHURCH_TERMINOLOGY.md and evaluation/church-terminology.json. Apply this policy when continuing the staged Mandarin correction and later language work.

## October 25 UI integration

Mandarin can now be selected from the ordinary Add Language Stream dialog. The app
loads the already-installed optional Mandarin voice on demand into the current
session configuration without requiring a manual configuration switch. It does
not change active engines/models/ports or download anything. A missing pack gives
installation guidance; keep the voice experimental due to the unresolved license
and pending native review. Other requested languages are visible as planned choices
with a Not implemented yet notice and cannot be applied as active streams.
