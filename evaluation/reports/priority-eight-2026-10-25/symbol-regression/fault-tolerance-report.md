# Fault-tolerant 8-language replay

Run status: completed_with_gaps. Input 70.000s; one shared recognition pass. All subtitle cue timestamps align, including untranslated English placeholders.

| Language | Delivered | Untranslated | Median / p95 / max delivered delay (s) |
|---|---:|---:|---|
| es | 17 | 1 | 6.35 / 10.68 / 10.68 |

es clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| de | 17 | 1 | 7.56 / 11.00 / 11.00 |

de clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| zh-Hans | 17 | 1 | 6.96 / 9.21 / 9.21 |

zh-Hans clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| fr | 17 | 1 | 7.26 / 9.36 / 9.36 |

fr clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| ja | 17 | 1 | 4.97 / 9.28 / 9.28 |

ja clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| ru | 17 | 1 | 6.47 / 9.08 / 9.08 |

ru clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| ar | 16 | 2 | 16.58 / 18.76 / 18.76 |

ar clause 8 at 33.78s: Arabic translation introduced unexpected Cyrillic script; translation rejected

ar clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| hi | 17 | 1 | 6.00 / 11.30 / 11.30 |

hi clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis

Final drain: 16.85s. Successful output timings exclude untranslated clauses. Failed clauses have no PCM delay; the plot uses NaN breaks rather than drawing through them or assigning zero delay.

Markers are saved as quoted Untranslated: followed by original source text in each affected target SRT and shown in live captions. They are not spoken through a target-language voice. Failure records are durable in failures.jsonl; delivered PCM timing remains in delays.jsonl. Other language routes continue independently. English transcript remains available.

Recoverable translation request/HTTP/model-output failures are clause-local. Cancellation, source-recognition failures, storage failures, synthesis/playback failures and unexpected panics remain fatal. A synthesis failure after partial audio is not silently labeled untranslated. No blind retry loop or completed-quotation fallback was added.

Source phrase start to first queued PCM; physical acoustic latency not measured. This is a short development replay, not sustained multilingual capacity or native-speaker quality approval. Older failed attempts remain preserved.