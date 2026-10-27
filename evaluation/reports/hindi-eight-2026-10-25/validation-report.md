# Hindi and eight-language speech validation

Input: 100.702-second title-page development clip. Shared Whisper Base recognition, Qwen3 4B Q4_K_M translation, four CPU threads per engine, one GPU translation slot. Persistent Piper 1.8 CPU workers use four threads each where configured. Other voices use existing native Piper.

Timing: source phrase start to first queued PCM; physical audio devices/acoustic latency not measured. Short replay is not a sustained capacity guarantee. All languages still require appropriate native-speaker/semantic review.

| Run | Language | Completed PCM clauses | Median (s) | p95 (s) | Max (s) |
|---|---|---:|---:|---:|---:|
| hindi-first-replay | hi | 38 | 20.82 | 43.57 | 43.85 |

hindi-first-replay: completed; drain 42.91s; SRT timestamps aligned. No cutoff or retry. All accepted clauses measured.

| hindi-persistent-replay | hi | 38 | 5.31 | 9.28 | 9.60 |

hindi-persistent-replay: completed; drain 4.05s; SRT timestamps aligned. No cutoff or retry. All accepted clauses measured.

| eight-language-first-replay | es | 12 | 6.98 | 8.71 | 8.71 |
| eight-language-first-replay | de | 13 | 5.98 | 6.59 | 6.59 |
| eight-language-first-replay | fr | 13 | 5.20 | 6.11 | 6.11 |
| eight-language-first-replay | zh-Hans | 13 | 6.06 | 7.28 | 7.28 |
| eight-language-first-replay | ru | 12 | 6.76 | 9.14 | 9.14 |
| eight-language-first-replay | it | 14 | 4.37 | 8.43 | 8.43 |
| eight-language-first-replay | ja | 14 | 4.55 | 7.17 | 7.17 |
| eight-language-first-replay | hi | 13 | 6.00 | 8.59 | 8.59 |

eight-language-first-replay: failed; drain unmeasured; SRT timestamps partial/unvalidated. Replay exited 1; inspect C:\Users\Paul Lott\Documents\New project\language-rollout\eight-language-first-replay\01-threads-4-slots-1

Incomplete portions are not counted as passing. Plot lines end at each language's last measured source clause; no extrapolation into unmeasured input.


## Quality and distribution limits

Hindi uses the Pratham medium voice. Upstream model card identifies CC BY-NC-SA 4.0 dataset terms; intended use is noncommercial and shared. Keep model/dataset attribution and component-specific notices. This does not change the application source license.

Official Hindi title-page reference: https://www.churchofjesuschrist.org/study/scriptures/bofm/bofm-title?lang=hin . Initial names मॉरमन की पुस्तक and नफी appear, but full-replay inspection found omitted names and questionable wording (for example financial-account wording खाता in narrative context). A name substring pass is not semantic approval. No scripture completion or invented recovery is performed. Native review and held-out passages remain pending.

The six-case Hindi text/speech regression is separate from live-input latency. Initial cold worker load is included in that test; it must not be compared directly to source-start speech delays.

A Windows Nirmala TTC fallback is loaded for Devanagari when installed. Complete Indic shaping and physical UI/audio review have not been established by these automated tests.

Raw evidence is retained in workspace language-rollout under each run name. Existing unsuccessful six-stream/Basque and Italian-old-runtime attempts remain preserved separately.
## Eight-stream failure details

Last observed consumed input was 47.98s; languages had first queued PCM for 12–14 clauses each, ending at source starts 29.76–33.96s. Everything after each line endpoint is unmeasured for that output. The llama log shows a request generating 1024 tokens and hitting the configured output limit. This was not a 600-second delay cutoff or an established RAM/VRAM exhaustion event. The failing target was not identified by the old message, so no target-specific cause is asserted. New diagnostics include target and finish_reason; no blind retry or truncated speech fallback was introduced.
