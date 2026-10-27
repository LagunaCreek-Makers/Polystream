# Stream capacity calibration

Run: completed. Source: 100.702 seconds of title-page speech. Single source pass per case, 4 threads/engine, 1 GPU translation slot.

| Streams | Language | Completed | Median / p95 / max delay (s) |
|---|---|---:|---|
| 1 | es | 38 | 5.34 / 8.64 / 8.66 |

Case 1: all 38 clauses per output; aligned SRTs; final drain 6.20s.

| 2 | es | 38 | 5.72 / 9.31 / 9.34 |
| 2 | de | 38 | 5.15 / 6.37 / 6.51 |

Case 2: all 38 clauses per output; aligned SRTs; final drain 7.12s.

| 3 | es | 38 | 6.01 / 9.25 / 9.28 |
| 3 | de | 38 | 5.29 / 6.36 / 6.46 |
| 3 | fr | 38 | 5.87 / 7.68 / 7.76 |

Case 3: all 38 clauses per output; aligned SRTs; final drain 7.37s.

| 4 | es | 38 | 6.68 / 10.81 / 11.10 |
| 4 | de | 38 | 5.73 / 6.65 / 6.66 |
| 4 | fr | 38 | 5.97 / 8.35 / 8.73 |
| 4 | zh-Hans | 38 | 6.27 / 7.34 / 7.52 |

Case 4: all 38 clauses per output; aligned SRTs; final drain 9.54s.


## Unvalidated count-only envelope

{
  "5": {
    "low": 7.3440594,
    "high": 12.366343899999999,
    "validated": false
  },
  "6": {
    "low": 8.0094569,
    "high": 13.922796599999998,
    "validated": false
  },
  "7": {
    "low": 8.674854400000001,
    "high": 15.479249299999998,
    "validated": false
  },
  "8": {
    "low": 9.3402519,
    "high": 17.035701999999997,
    "validated": false
  }
}

These projections assume similar speech and voice costs. They use the largest positive observed median/p95 increment per added stream; they do not bound latency, model steady-state queue stability, include acoustic latency, or establish eight-language support on this hardware. Test the exact combination over a sustained meeting before relying on it.

The earlier 45-minute Spanish/German test reached 125.45/115.46 seconds maximum despite short-test medians near six seconds. Short calibration is not a sustained-session guarantee. CPU/GPU usage alone does not determine delay. Whole-system resource measurements are included in the raw run. Hardware upgrade recommendations require stage-specific evidence.

State: completed. Incomplete/failed cases have no invented results. Native-speaker review and physical output devices are not established by this replay.