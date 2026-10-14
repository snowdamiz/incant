# QuickJS store diagnostic: no adoption

The diagnostic-only numeric Float64 store candidate did not show a consistent
benefit. It was slower in two of three mean comparisons and faster in one.
No production dependency patch, shipping rebuild or optimization intake is
justified by these results.

The predeclared plan `55702a6a` and runner `58d667b2` ran once in the acknowledged
quiet window: six fresh processes in B0/C0/C1/B1/B2/C2 order. Warmup was zero;
all 500 samples per process, including first initialization, were retained.
Every run matched its functional reference, including 58 semantic records,
three negative controls, all 9,999 rows/sentinels/padding at every callback tick,
and three final layout controls. No errors, retries or exclusions occurred.

Milliseconds below are actual standalone callback intervals, not engine frame
or shipping-profile measurements:

| Pair | Original mean | Candidate mean | Original median | Candidate median | Original p95 | Candidate p95 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 1.156390 | 1.177176 | 1.142396 | 1.178521 | 1.270084 | 1.266916 |
| 1 | 1.093275 | 1.152095 | 1.080750 | 1.097980 | 1.144500 | 1.289167 |
| 2 | 1.154163 | 1.121970 | 1.144667 | 1.074375 | 1.256083 | 1.250875 |

The candidate/original mean changes were +1.80%, +5.38% and −2.79%.
These are descriptive paired results, not statistical significance or a shipping
regression gate. The p95 uses nearest rank, sorted sample 475 of 500; median
averages sorted samples 250 and 251. Verification is outside the measured
`Function.call` update interval. This does not include Incant's host lifecycle,
native loans, Core controller, physics, rendering or audio.

Source `831c89a8` and the three isolated CI executables are bound by the preceding
[functional diagnostic](quickjs-f64-store-diagnostic.md), packet `3a6552e0`.
Original/candidate binaries remain `58bb741d…` / `0b75ca98…`; the deliberately
wrong-store binary was not timed. All compiler/C/lock/callback identities and
historical failures remain unchanged. The normal engine dependency is untouched.

The [portable timing manifest](evidence/quickjs-f64-store-timing/manifest.json)
retains 27 records and 21 deduplicated payloads, including all 3,000 raw intervals,
reference outputs, plan, runner, truthful four-lane acknowledgments and outcome
receipts. Source and executable bindings are externally hash-verifiable:

```sh
python3 docs/spikes/evidence/quickjs-f64-store-timing/verify.py
python3 docs/spikes/evidence/quickjs-f64-store-timing/verify.py --external
```

Both passed with all summaries and paired ratios recomputed. The sequence took
19.725 seconds wall time. Supervisor 2003 exited zero and was reaped; PID 48063
and all players were absent before the quiet window was released. No timing
sample was retried. The 2 ms Core shipping gate remains unmet.
