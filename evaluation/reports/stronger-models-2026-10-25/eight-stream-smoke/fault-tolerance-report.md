# Fault-tolerant 8-language replay

Run status: completed_with_gaps. Input 70.000s; one shared recognition pass. All subtitle cue timestamps align, including untranslated English placeholders.

| Language | Delivered | Untranslated | Median / p95 / max delivered delay (s) |
|---|---:|---:|---|
| es | 17 | 1 | 6.70 / 10.21 / 10.21 |

es clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| de | 17 | 1 | 9.08 / 16.43 / 16.43 |

de clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| zh-Hans | 17 | 1 | 6.36 / 7.67 / 7.67 |

zh-Hans clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| fr | 17 | 1 | 8.36 / 12.73 / 12.73 |

fr clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| ja | 17 | 1 | 7.73 / 9.77 / 9.77 |

ja clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| ru | 17 | 1 | 8.00 / 14.05 / 14.05 |

ru clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| ar | 17 | 1 | 17.85 / 23.89 / 23.89 |

ar clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| hi | 17 | 1 | 8.51 / 11.95 / 11.95 |

hi clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis

Final drain: 22.30s. Successful output timings exclude untranslated clauses. Failed clauses have no PCM delay; the plot uses NaN breaks rather than drawing through them or assigning zero delay.

Markers are saved as quoted Untranslated: followed by original source text in each affected target SRT and shown in live captions. They are not spoken through a target-language voice. Failure records are durable in failures.jsonl; delivered PCM timing remains in delays.jsonl. Other language routes continue independently. English transcript remains available.

Recoverable translation request/HTTP/model-output failures are clause-local. Cancellation, source-recognition failures, storage failures, synthesis/playback failures and unexpected panics remain fatal. A synthesis failure after partial audio is not silently labeled untranslated. No blind retry loop or completed-quotation fallback was added.

Source phrase start to first queued PCM; physical acoustic latency not measured. This 1.17-minute file-paced development replay measures this exact configuration and soundtrack only; it is not native-speaker quality approval or a guarantee for longer meetings. Older failed attempts remain preserved.