# Per-call matrix basis cache: retained failed shipping experiment

The isolated matrix-basis cache preserves the reviewed correctness controls, but
this experiment does **not** accept it for production. All three candidate
script p95 intervals exceed the unchanged 2 ms budget. Pair 0 also has a definite
frame p95 regression above 5%. There were no retries, excluded frames, or workload
changes. This is a failed performance candidate with useful correctness evidence.

The source is `66c5d14761caf61f5d2e3be90313bc5dc27bf715`, with correctness packet
`8baa450446ab81f2c6b57123d23b93cfa5efc133`, over baseline
`03bc4b987f3e553249846c4cfce991793d4d2c48`. Only four Rust source paths differ:
the private cache and its tests, the module declaration, and the existing numeric
validation seam. The prior cursor-loop and persistence-inspector candidates are
absent. No sample script, VM budget, validation order, profiler threshold, or
ordinary compiler configuration changed.

The one-entry cache is local to each checked numeric publication. It keys the
quaternion and scale by their seven exact f64 bit patterns, reuses only the three
translation-independent basis columns, and builds the current translation column.
Misses call the original matrix function, which is also the parity oracle. Every
row still receives the original component validation. Parent/root multiplication,
finite checks, error order, and publication remain in their previous positions.
See `numeric-basis-cache.md` for the focused correctness boundary and its source
review; no shipping benefit follows from those CPU tests.

## Source and execution binding

The ordinary public exporter ran from the clean packet tip, using Rust 1.99.0,
locked/offline dependencies, explicit `aarch64-apple-darwin`, shipping profile,
summary profiling, two Cargo jobs, and two Rayon threads. It created fresh private
build storage and removed it before the source-free startup check. The compile
command took 280.597 seconds. All seven exporter commands returned zero. After
export, 20,591 tracked source files, all 74 authoring files, and the pinned Node /
TypeScript tool files matched the pre-build freeze. No target reuse or claim of
compile-speed improvement is involved.

| Player | SHA-256 |
| --- | --- |
| Retained baseline 03bc | `a2794edcc1e7b21db3291eebb76cb2cf923290cc18005f91b023fe4de27fa361` |
| Basis candidate 8baa | `ae65066bec7d686a35386f5b43bc2f34a229a2185b51be5aaeeffa48369a3b28` |

All 74 authoring files are byte-identical to the baseline. The exact Core project,
input recording, observation plan, gameplay and stress sources, cooked runtime
data, codegen flags, fast SIMD8 physics features, and profile configuration match.
The population is 10,045 entities with 10,000 Script attachments across two
sources: one Core controller and 9,999 stress rows. Every script row retains its
three writes and actual checksum readbacks.

## Functional controls

Supervisor 96663 ran 41 new unprofiled candidate processes, all successful:

- One full 2,156-tick Core win matched the complete retained baseline JSON report
  and PCM bytes, including the existing gameplay observation oracle.
- Forty 65-tick batches checked all 9,999 final row vectors with finite-value and
  numerical checks, plus sentinels at ticks 1, 63, 64, and 65 and entity population.
  Each complete report and PCM matched its corresponding historical 03bc baseline.

The forty baseline row runs were explicitly reused from the earlier combined
experiment's baseline half. Their actual player identity, command receipts,
inputs, observation plans, and raw results were checked. They are not new runs
and do not include the failed cursor/inspector candidate. These controls observe
every final row; they do not claim to inspect every intermediate write.

## Predeclared quiet comparison

Plan SHA `7b53506a538b72b606253b8eee31b11a651bfeb9519f69714ca3028103eabf5d`
and runner SHA `1c8ec19778d4fbaed3695b7607439923676d30738203076e7a1dc2a683fa4e5a`
were reviewed before execution. The quiet receipt retains root's explicit grant
and acknowledgments from root, publisher, runtime reviewer, and this lane.

Supervisor 37856 executed B0/C0/C1/B1/B2/C2 once. Each process completed exactly
2,156 frames with empty stderr. Both source callbacks had 2,156 successful calls.
All raw reports apart from profile data, and all PCM bytes, matched the unprofiled
control. Every frame counted; no warmup was excluded. The unmodified
`mac_performance.comparison` required every workload/machine/configuration match
field equal and calculated the original 2 ms and 5% interval verdicts.

The intervals below are milliseconds. Each lower endpoint excludes the previous
10 microsecond bin; these are histogram intervals, not exact percentile values.

| Pair | Baseline script p95 | Candidate script p95 | Baseline frame p95 | Candidate frame p95 | Result |
| --- | --- | --- | --- | --- | --- |
| 0 | [2.390001, 2.40] | [2.370001, 2.38] | [3.260001, 3.27] | [3.460001, 3.47] | 2 ms fail; frame >5% regression |
| 1 | [2.370001, 2.38] | [2.370001, 2.38] | [3.220001, 3.23] | [3.350001, 3.36] | 2 ms fail |
| 2 | [2.410001, 2.42] | [2.340001, 2.35] | [3.310001, 3.32] | [3.210001, 3.22] | 2 ms fail |

Requested Rust allocation peaks were 76,881,326 bytes in every baseline, and
76,881,225 / 76,881,193 / 76,881,225 bytes for the candidates. All allocation and
script 5% comparisons passed, but that does not erase the failed absolute script
budget or pair 0 frame regression. Requested Rust allocation is not RSS or GPU
memory. Three pairs are not a statistical confidence interval.

Both build/functional supervisors and the timing supervisor are terminal and
reaped. No native player remains. The quiet hold was explicitly released. This
is current-Mac CPU simulation evidence; there is no rendered FPS, GPU, mobile,
other-device, or phase-gate approval claim.

## Replay without executing native code

The packet at `evidence/numeric-basis-shipping/manifest.json` retains raw reports,
commands, freezes, plans, source copies, original correctness packet, and reviewed
source receipt. Its independent verifier recomputes nearest-rank p95 intervals
from all raw histogram bins, strict comparison verdicts, functional scalars and
report identities. It never invokes a compiler, player, or timing workload.

```sh
python3 docs/spikes/evidence/numeric-basis-shipping/verify.py
python3 docs/spikes/evidence/numeric-basis-shipping/verify.py --external --originals --git-source
```

Assets, authoring history, PCM, executable/symbol files, and logs larger than 8 MiB
remain hash-bound external evidence under `~/.codex/incant-evidence/`. Portable
replay verifies the recorded identities and calculations; external replay also
checks those exact bytes. It does not manufacture unavailable original files.
The packet is retained for audit, not as approval to integrate the candidate.
