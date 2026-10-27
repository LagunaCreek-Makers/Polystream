# Language rollout replay report

State: failed; input 100.702s. 4 threads/engine, one GPU translation slot.

Basque translation quality FAILED despite voice operation. This is mechanical timing evidence, not six validated languages. Russian terminology retest is separate. No physical device/acoustic latency or native-speaker quality review.

| Case | Language | Clauses | Median | p95 | Maximum |
|---|---|---:|---:|---:|---:|
| 1 | ru | 38 | 6.41 | 11.23 | 11.44 |
| 1 | eu | 38 | 20.33 | 36.77 | 37.88 |

Case 1 drain: 42.50s; SRT cue counts/timestamps aligned.


## Incomplete case 2

Replay exited 1; inspect C:\Users\Paul Lott\Documents\New project\language-rollout\russian-basque-six-stream\02-threads-4-slots-1
Pipeline error: incomplete translation rejected; not a 600-second cutoff. No automatic retry. Final polling observation is approximate, not the exact failure instant.
Last observed source consumption: 76.56 seconds. No completed drain measurement. Unmeasured output remains unvalidated.

es: 22 measured first-PCM clauses; median/p95/max 6.82/7.49/7.64s; last measured source start 57.08s.
de: 23 measured first-PCM clauses; median/p95/max 5.42/6.15/6.36s; last measured source start 60.88s.
fr: 23 measured first-PCM clauses; median/p95/max 5.80/6.67/7.16s; last measured source start 60.88s.
zh-Hans: 23 measured first-PCM clauses; median/p95/max 6.18/7.51/7.57s; last measured source start 60.88s.
ru: 22 measured first-PCM clauses; median/p95/max 8.25/9.81/10.27s; last measured source start 57.08s.
eu: 18 measured first-PCM clauses; median/p95/max 12.26/21.09/21.09s; last measured source start 43.36s.

Delays are source phrase start to first queued PCM. No cutoff or retry was needed for completed cases; see explicit failure evidence for partial cases. Failed initial Italian attempt remains preserved separately; zero PCM caused early failure, so remaining input was not tested in that attempt.

The title-page clip is development material, not held-out evaluation. Short test results do not establish sustained meeting performance. Basque is disabled in the shipping UI; results remain diagnostic evidence only.