# Fault-tolerant 8-language replay

Run status: completed_with_gaps. Input 900.000s; one shared recognition pass. All subtitle cue timestamps align, including untranslated English placeholders.

| Language | Delivered | Untranslated | Median / p95 / max delivered delay (s) |
|---|---:|---:|---|
| es | 300 | 1 | 11.60 / 29.81 / 41.43 |

es clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| de | 300 | 1 | 13.19 / 29.24 / 37.90 |

de clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| zh-Hans | 300 | 1 | 7.46 / 24.97 / 37.69 |

zh-Hans clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| fr | 300 | 1 | 17.67 / 31.85 / 39.83 |

fr clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| ja | 300 | 1 | 6.69 / 26.54 / 40.02 |

ja clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| ru | 300 | 1 | 23.06 / 37.17 / 44.57 |

ru clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| ar | 291 | 10 | 57.09 / 77.71 / 84.96 |

ar clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis

ar clause 41 at 133.68s: Arabic translation introduced unexpected Cyrillic script; translation rejected

ar clause 129 at 402.52s: Arabic translation introduced unexpected Cyrillic script; translation rejected

ar clause 171 at 518.34s: Arabic translation introduced unexpected Cyrillic script; translation rejected

ar clause 198 at 598.38s: Arabic translation introduced unexpected Cyrillic script; translation rejected

ar clause 238 at 722.00s: Arabic translation introduced unexpected Cyrillic script; translation rejected

ar clause 254 at 769.88s: Arabic translation introduced unexpected Cyrillic script; translation rejected

ar clause 266 at 801.30s: Arabic translation introduced unexpected Cyrillic script; translation rejected

ar clause 274 at 825.54s: Arabic translation introduced unexpected Cyrillic script; translation rejected

ar clause 280 at 841.74s: Arabic translation introduced unexpected Cyrillic script; translation rejected
| hi | 297 | 4 | 5.05 / 13.58 / 19.44 |

hi clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis

hi clause 157 at 469.10s: Hindi text translation was incomplete (finish_reason=length); stopped instead of speaking a truncated result

hi clause 221 at 669.70s: Hindi text translation was incomplete (finish_reason=length); stopped instead of speaking a truncated result

hi clause 230 at 699.90s: Hindi text translation was incomplete (finish_reason=length); stopped instead of speaking a truncated result

Final drain: 63.01s. Successful output timings exclude untranslated clauses. Failed clauses have no PCM delay; the plot uses NaN breaks rather than drawing through them or assigning zero delay.

Markers are saved as quoted Untranslated: followed by original source text in each affected target SRT and shown in live captions. They are not spoken through a target-language voice. Failure records are durable in failures.jsonl; delivered PCM timing remains in delays.jsonl. Other language routes continue independently. English transcript remains available.

Recoverable translation request/HTTP/model-output failures are clause-local. Cancellation, source-recognition failures, storage failures, synthesis/playback failures and unexpected panics remain fatal. A synthesis failure after partial audio is not silently labeled untranslated. No blind retry loop or completed-quotation fallback was added.

Source phrase start to first queued PCM; physical acoustic latency not measured. This 15.00-minute file-paced development replay measures this exact configuration and soundtrack only; it is not native-speaker quality approval or a guarantee for longer meetings. Older failed attempts remain preserved.

## Sustained rehearsal observations

All 900 seconds of the original soundtrack were fed at real-time pace, including music. All 301 source clauses were handled by each route: 2,390 audio deliveries and 18 untranslated markers out of 2,408 opportunities. All nine SRTs have 301 identical cue timestamp pairs. Backlog drained 63.01 seconds after input ended. No cutoff, retry or engine crash occurred. This is completion with gaps, not an all-translated accuracy pass.

Eight untranslated cues are the same music-symbol-only source clause (one per language). Nine further Arabic cues were rejected for introduced Cyrillic script. Three further Hindi cues hit the 1,024-token incomplete-output guard. Later clauses resumed in every case. English recognition itself contains apparent proper-name errors (for example, Patrick Kiran and Elder Zora Corset); this test does not establish source transcript or translation accuracy. A targeted source-recognition/translation review is needed before promoting experimental languages.

Maximum sampled oldest retained-source age: 84.98s. Sampling is every few seconds; this is not a continuous peak measurement.

Whole-system CPU mean/max: 40.76% / 86.72%. Whole-system RAM mean/max: 48.83 / 52.20 GiB. GPU utilization mean/max: 30.97% / 92%; sampled GPU memory peak 6,753 MiB. These aggregate counters do not identify the bottleneck or measure application-only memory. Arabic delay is materially higher than other routes, but this run alone does not isolate translation scheduling, text length, CPU phonemization, synthesis or queued audio as the cause.


Follow-up correction: Dallin Harris Oaks is correct in the supplied captions; it is not an ASR error. See the quality-diagnostics report for original-audio comparisons, including the domain hint regression.
