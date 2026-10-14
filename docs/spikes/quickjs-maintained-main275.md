# Maintained QuickJS shipping observation result

The complete ScriptTick remains above the unchanged 2 ms budget. All six runs
completed; every within-mode frame/script/requested-memory 5% check passes, while
the composite comparisons fail solely on the budget. M0 is the selected maintained
baseline even though it fails. No phase gate is approved.

Execution source is `43b185ef03e0ac377eac50a1183781aef5ab212c`, based on exact
main275 `0b1f739c6617f0f654380ae7ad7910219e789b0c`. Runtime, vendor and public
exporter inputs remain main275; only the reviewed runner/protocol paths are added.
Later main276 application/test repairs are not relabeled as this execution.
The one fresh public export used Rust 1.99, locked/offline shipping, jobs 2/Rayon 2,
arm64 macOS, summary profiling and the unmodified private-target lifecycle.
Its built-in one-tick startup and all three full 2,156-tick functional controls
passed. All 74 authoring files remain exact; full logical reports after removing
only `profile` and exact PCM match the historical unprofiled anchor. Core win and
stress assertions pass. Old v2 timings remain historical.

The quiet sequence used the same player/package on Mac17,9, Apple M5 Pro,
macOS 27.0.1 (26A434), AC power, 15 physical CPUs and 48 GiB RAM. All six fresh
processes include every frame, have 2,156 valid heap observations and two ordered
sources with 2,156 calls each, zero unwind and zero unavailable CPU samples.
The mode, v3/heap-v1 schema, 32 MiB heap limit and detailed/null estimate rules match.
The fixed order has no retry, warmup exclusion or quantile averaging.

| Run | Whole ScriptTick p95 interval, ns | 2 ms |
| --- | --- | --- |
| D0 | 2,230,001–2,240,000 | fail |
| M0 | 2,090,001–2,100,000 | fail |
| M1 | 2,080,001–2,090,000 | fail |
| D1 | 2,220,001–2,230,000 | fail |
| D2 | 2,200,001–2,210,000 | fail |
| M2 | 2,100,001–2,110,000 | fail |

D0−M0 and D1−M1 have descriptive quantile differences of 130,001–149,999 ns;
D2−M2 has 90,001–109,999 ns. These describe independent distribution quantiles,
not telemetry p95 or a guest-loop improvement. No strict comparison crosses modes.

M0 callback wall/current-thread CPU means are 0.216964723/0.216911876 ms for Core
and 0.944506474/0.944452736 ms for the 9,999-instance bulk source. Inclusive
ScriptTick wall/CPU means are 1.502785814/1.502647570 ms. These contextual means
are not callback p95s or exclusive cost attribution, and none is subtracted from
the whole-tick p95. A bounded next engine hypothesis is Rust-owned undo/ID capacity
reuse in `loan.rs`; allocation cost is not isolated by these profiles, so no saving
or future pass is predicted. Copies, validation, detachment and rollback remain
required. A new candidate needs separate functional and shipping qualification.

The [portable manifest](evidence/quickjs-maintained-main275/manifest.json) binds
raw reports, commands, metrics, compiler log, functional/build/grant receipts and
external PCM/player/assets. Its 1.9 MiB archive supports a read-only replay:

```sh
python3 -B docs/spikes/evidence/quickjs-maintained-main275/verify.py --verify-only
# On the original host, additionally rehash every retained original:
python3 -B docs/spikes/evidence/quickjs-maintained-main275/verify.py --verify-only --external
```

The external replay exited 0. It runs no player or compiler. Large external
payloads are hash-bound rather than embedded. The public exporter deleted its
private Cargo JSON stream before independent retention; published artifact
reports and the complete verbose build log are retained. This is Mac CPU
simulation evidence, with no GPU/frame-presentation or complete phase approval.

Current-main intake is qualified separately on main276. The copied runner paths
are producer-exact. Local metric controls (21), sequence/refusal controls (13) and
native process controls (44 passed, four Windows-only skips) pass, as do the
portable external replay, conventions and whitespace checks. Independent runner
source/evidence review found no actionable defects. No native rerun is relabeled
as part of this integration.
