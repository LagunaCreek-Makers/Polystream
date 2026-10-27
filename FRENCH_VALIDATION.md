# French language pack — validation in progress

Pinned voice: rhasspy/piper-voices / fr_FR-siwis-medium, revision
5041a7a49ae5b3a0f30e6837769eb03fdbb231bd. Upstream model card identifies the SIWIS
dataset and CC BY 4.0 license. Attribution/model card retained beside the model;
model card URL:
https://huggingface.co/rhasspy/piper-voices/blob/5041a7a49ae5b3a0f30e6837769eb03fdbb231bd/fr/fr_FR/siwis/medium/MODEL_CARD
Dataset: https://datashare.is.ed.ac.uk/handle/10283/2353

The native espeak French frontend is compatible with the installed Piper runtime.
Install-French.cmd downloads only the pinned/checksummed optional pack. Session
configuration adds this installed voice when French is selected; other model and
hardware settings remain unchanged. offline.french.json includes the four current
evaluation voices. Mandarin remains experimental under its separate license caveat.

Initial French Church glossary is sourced from the official French title page:
https://www.churchofjesuschrist.org/study/scriptures/bofm/bofm-title?lang=fra
Livre de Mormon, Néphi, Mormon (ancient author), plaques (writing medium).
This title-page clip is development material, not held-out evaluation. Natural
pronunciation and broad religious-translation quality require further validation.

Pending: isolated 1/2/3/4-stream replay, subtitle counts/timestamps, delay statistics,
inspection of new French output and sustained/native-speaker review. No claim that
all requested languages are production-ready. Incompatible/missing voices are
skipped for later work, as authorized by the user.

## 2026-10-25 mechanical validation completed

French completed 38/38 phrases in both the three- and four-language cases; all English/target SRT cue timestamps aligned. Four-language French median/p95/max: approximately 6.06/8.35/8.73 seconds (summary median convention). Whole run drain: 9.54 seconds. See evaluation/reports/capacity-2026-10-25 for exact recalculated statistics. No cutoff/retry.

Inspection found Livre de Mormon, Mormon, Néphi and plaques in initial cues. Other clauses include questionable phrases (for example cue 12), so mechanical success does not establish accurate translation or pronunciation. Native-speaker review, held-out material and sustained French testing remain pending. The installer now creates the optional configuration if absent and preserves existing configurations.
