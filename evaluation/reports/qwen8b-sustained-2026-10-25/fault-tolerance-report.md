# Fault-tolerant 8-language replay

Run status: completed_with_gaps. Input 900.000s; one shared recognition pass. All subtitle cue timestamps align, including untranslated English placeholders.

| Language | Delivered | Untranslated | Median / p95 / max delivered delay (s) |
|---|---:|---:|---|
| es | 300 | 1 | 9.71 / 31.00 / 33.52 |

es clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| de | 300 | 1 | 12.22 / 28.37 / 35.14 |

de clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| zh-Hans | 300 | 1 | 9.98 / 29.16 / 37.78 |

zh-Hans clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| fr | 300 | 1 | 17.37 / 34.00 / 36.22 |

fr clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| ja | 300 | 1 | 12.67 / 39.29 / 45.35 |

ja clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| ru | 300 | 1 | 27.09 / 44.40 / 47.42 |

ru clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis
| ar | 298 | 3 | 70.40 / 91.91 / 96.64 |

ar clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis

ar clause 71 at 195.72s: Arabic translation introduced unexpected Cyrillic script; translation rejected

ar clause 261 at 787.44s: Arabic translation introduced unexpected Cyrillic script; translation rejected
| hi | 300 | 1 | 5.66 / 16.57 / 19.13 |

hi clause 13 at 52.54s: Translation contains no letters or numbers for speech synthesis

Final drain: 77.83s. Successful output timings exclude untranslated clauses. Failed clauses have no PCM delay; the plot uses NaN breaks rather than drawing through them or assigning zero delay.

Markers are saved as quoted Untranslated: followed by original source text in each affected target SRT and shown in live captions. They are not spoken through a target-language voice. Failure records are durable in failures.jsonl; delivered PCM timing remains in delays.jsonl. Other language routes continue independently. English transcript remains available.

Recoverable translation request/HTTP/model-output failures are clause-local. Cancellation, source-recognition failures, storage failures, synthesis/playback failures and unexpected panics remain fatal. A synthesis failure after partial audio is not silently labeled untranslated. No blind retry loop or completed-quotation fallback was added.

Source phrase start to first queued PCM; physical acoustic latency not measured. This 15.00-minute file-paced development replay measures this exact configuration and soundtrack only; it is not native-speaker quality approval or a guarantee for longer meetings. Older failed attempts remain preserved.