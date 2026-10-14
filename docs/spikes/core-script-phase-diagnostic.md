# Native script phase diagnostic

One exact CPU test passed on source `13b178c81998b6b74bb4db2b06b0fc343cea0eb0`,
from the preserved `03bc4b987f3e553249846c4cfce991793d4d2c48` engine. It executed
128 real ScriptHost ticks against the unchanged Core stress script and 9,999
native Transform rows. Every row and the finite sentinel were checked after
every tick; three separate omitted x/y/z-write mutations were rejected.

This is a diagnostic of one source, not the full Core workload: the controller,
46 original entities, assets, UI, audio, playback and physics are absent. It uses
an optimized CI test binary, not a shipping player. The parent reported rendering
CPU/GPU work stopped; a peer was allowed bounded raw CPU analysis. No quiet timing
or 2 ms gate acceptance is claimed. The failed combined cursor/inspector shipping
experiment remains unchanged and those production candidates remain held.

Fourteen markers exist only with `cfg(all(test, feature = "profile", feature =
"compiler"))`. Normal libraries and players contain no marker code. Source bytes,
features, full build-input inventory, preparation, command, raw stdout/stderr and
an immediate executable copy are bound in the evidence. The ordinary 50,000 us
guest deadline and 32 MiB VM/loan budgets were unchanged. No exception, numeric,
UI, physics, hierarchy, lifetime or publication check was removed.

## Observations

All 128 ticks are included, including the first 10.240166 ms outer sample. The
outer mean is 1.682080 ms and the exact nearest-rank p95 order statistic is
1.766917 ms. Each row below reports the same 128 intervals. Means are additive;
percentiles are not. The remaining outer tail averages 0.000575 ms.

| Region ending at marker | Mean ms | Diagnostic p95 ms |
| --- | ---: | ---: |
| enter | 0.000369 | 0.000625 |
| initial_checks | 0.000213 | 0.000333 |
| snapshots | 0.011680 | 0.016042 |
| group_setup | 0.056013 | 0.055291 |
| capabilities | 0.015119 | 0.021000 |
| undo_capture | 0.013814 | 0.015833 |
| view_exposure | 0.015088 | 0.006375 |
| callbacks | 1.134425 | 1.196375 |
| revoke_checks | 0.000189 | 0.000292 |
| numeric_validation | 0.253436 | 0.286583 |
| transaction_stage | 0.000236 | 0.000500 |
| scope_cleanup | 0.075828 | 0.089250 |
| publication | 0.000950 | 0.001625 |
| heap_telemetry | 0.104145 | 0.121708 |

Guest callback execution dominates this fixture. The largest measured host region
is numeric validation, followed by heap telemetry and scope cleanup. Source
inspection identifies `prepare_candidate_globals` as the numeric-validation
path: it reads all applicable components, validates scalar/UI/physics domains,
normalizes the quaternion, builds the local matrix, composes parent-first derived
matrices and checks finiteness. Scope cleanup publishes those validated matrices
and marks changes. Any optimization must preserve that validation, error order,
exact arithmetic and rollback/unwind behavior. This diagnostic has not isolated
which individual operation within those regions dominates.

The transaction-stage region, which includes persistence extraction, averages
0.000236 ms here. This result does not support prioritizing another persistence
parser change for this particular workload. Heap telemetry is the existing
per-tick QuickJS heap walk inside ScriptTick; it remains included.

The existing profile contains exactly 128 script.tick calls, 128 source calls,
128 heap samples, and zero unwinds. There is no player Frame guard in this
script-only test, so player frame histograms intentionally contain zero samples;
they are not represented as shipping frame coverage. All region records are raw
nanosecond durations, not the profiler's binned intervals.

## Reproduction and evidence

The preparation and exact single-test command are in
`tools/probes/core-script-phase/README.md`. The test is intentionally ignored by
ordinary suites and must be selected by its full name with `--exact --ignored`.
The retained command used Rust 1.99.0, explicit aarch64-apple-darwin, `--profile ci`,
locked/offline dependencies, two jobs and Rayon 2, with
`compiler,profile,physics-fast-simd8`. One named test passed; no full suite or
strict Clippy run is claimed for this diagnostic source.

Supervisor 49242 returned zero and reaped its child tree. Its OS PID 4717 was
subsequently absent. The 15.803-second build/test command retained the executable
immediately, before any target reuse: SHA256
`3821ab6019a19453564f6e24484079bf3da34c0b56e04775e88db45432e78194`, mode 0755.
The owned target and builder were released. No native rerun, shipping build or GPU
execution followed. All frozen source identities remained unchanged.

The compact portable packet is `docs/spikes/evidence/core-script-phase/`.
`verify.py` rehashes its lossless payloads and recomputes every mean, sum, p95,
interval/tail equation and coverage count directly from raw stdout. Add
`--external` to rehash original files and the external executable; add
`--git-source` to rehash every build input against the immutable source commit.
Large executable bytes are hash-bound externally; raw logs and frozen owned
sources are portable. The collector never executes native work.

```sh
python3 docs/spikes/evidence/core-script-phase/verify.py --external --git-source
```
