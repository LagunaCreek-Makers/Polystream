# Fault-tolerant eight-language replay

Run status: completed_with_gaps. Input 100.702s; one shared recognition pass. All subtitle cue timestamps align, including untranslated English placeholders.

| Language | Delivered | Untranslated | Median / p95 / max delivered delay (s) |
|---|---:|---:|---|
| es | 38 | 0 | 17.96 / 24.49 / 24.87 |
| de | 38 | 0 | 15.90 / 18.28 / 18.59 |
| fr | 38 | 0 | 12.84 / 18.56 / 19.03 |
| zh-Hans | 38 | 0 | 17.03 / 19.95 / 20.13 |
| ru | 38 | 0 | 19.88 / 27.94 / 28.22 |
| it | 38 | 0 | 4.78 / 12.99 / 14.62 |
| ja | 38 | 0 | 13.34 / 19.03 / 19.05 |
| hi | 37 | 1 | 11.13 / 15.07 / 15.32 |

hi clause 13 at 33.96s: Hindi text translation was incomplete (finish_reason=length); stopped instead of speaking a truncated result

Final drain: 26.18s. Successful output timings exclude untranslated clauses. Failed clauses have no PCM delay; the plot uses NaN breaks rather than drawing through them or assigning zero delay.

Markers are saved as quoted Untranslated: followed by original source text in each affected target SRT and shown in live captions. They are not spoken through a target-language voice. Failure records are durable in failures.jsonl; delivered PCM timing remains in delays.jsonl. Other language routes continue independently. English transcript remains available.

Recoverable translation request/HTTP/model-output failures are clause-local. Cancellation, source-recognition failures, storage failures, synthesis/playback failures and unexpected panics remain fatal. A synthesis failure after partial audio is not silently labeled untranslated. No blind retry loop or completed-quotation fallback was added.

Source phrase start to first queued PCM; physical acoustic latency not measured. This is a short development replay, not sustained eight-language capacity or native-speaker quality approval. Older failed attempts remain preserved.