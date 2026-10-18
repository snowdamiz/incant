# Native query stages inside ScriptTick

This default-off diagnostic observes the existing TypeScript query path. It does
not optimize it or change the 2 ms target. Scoped CPU qualification and one
subsequent same-source ordinary/query shipping pair passed exact full-Core
gameplay and offline PCM parity. The instrumented query breakdown below is a
diagnostic observation, not a measured optimization or performance-gate result.

`diagnostic-script-queries` forwards from the native player through `incant_script`
to `incant_profile`, and implies the existing ScriptTick phase observer. Ordinary
builds omit the module, fields and hook calls. A selected build records only while
the existing explicit owner-thread `Capture` and ScriptTick callback are active.
It never starts a listener or captures the display.

The six fixed categories are character movement, trigger events, navigation path,
locale, localization and **other**. Other retains every remaining typed query
kind; it is not a guessed Core category or an omitted cost. Query arguments,
results, project contents and new source identities are not stored. A source-slot
bitmask joins the report's existing source table and does not imply per-source
query timings when more than one slot contributes.

The hooks in `TickQueries::expose_methods` observe four nested intervals:

- **call:** a guard created at the closure's first statement observes through its
  final drop, including ordinary early refusals. Its first timestamp follows
  token admission/context setup; the final timestamp precedes drop aggregation.
  Those bookkeeping intervals and bridge argument collection before entry are
  excluded. This is not the complete bridge/closure duration.
- **parse:** only the unchanged `query_values::parse` call, including guest getters.
- **execute:** only the unchanged typed `NativeQueries` dispatch.
- **materialize:** only `query_values::materialize`, including result allocation.

Admission, argument/work charging, both deadline checks, output charging and error
mapping retain their original ordering. Their durations stay in call residual.
The observation neither alters limits nor caches or removes enforcement. A null
navigation result is a successful query, although the separate navigation
observer counts it as an absent path. Stage errors and pre-parse admission errors
remain distinguishable. Unwinds, arithmetic/clock anomalies, missing boundaries
and forgotten guards invalidate coverage; they never become valid zero samples.

Recursive getters can invoke another query while parsing. Their counts, failures
and inclusive times remain visible, but a nonzero recursive count invalidates the
row's exclusive coverage. Parent and child durations must not be added. No new
observer RefCell borrow spans guest code. Generation-bound guards cannot publish
into a replacement capture; non-Send guards and owner-thread rows retain the
existing collection scope.

Rows contain fixed arrays only. With this feature, both native row types must be
at most 2,048 bytes and the existing 4,096-row collector is bounded to 8 MiB.
Without it the original 1,024-byte/4 MiB ceiling remains. Snapshot/JSON allocations
are additional. The subsequent diagnostic execution bounded final JSON to 32 MiB;
there is no per-tick JSON serialization or per-query allocation in these hooks.
Counters use checked accumulation. Time is monotonic wall duration, not exclusive
thread CPU cost; observer work remains included.

The outer report version remains diagnostic-only version 4. The nested schema is
`1 + navigation + 2*budget + 4*queries`: existing feature combinations remain
schemas 1–4; query combinations use 5–8. This preserves existing feature-off JSON.
The old private analyzer rejects the new schema instead of silently accepting it.
The explicit query-aware private adapter requires the selected exact schema,
category/region arrays, bounds and complete raw coverage; unknown schemas/features
refuse. Nothing here changes public exporter admission.

## CPU qualification

Collector controls cover every category/stage and early error, recursive getters,
missing/reordered/overlapping/leaked scopes, duration/count overflow and backward
clock observations, no recording, foreign-thread exclusion, retired capture
lifetimes and caught unwind. Real ScriptHost controls exercise native character
movement, trigger events, localized text/missing keys, number formatting, path
hit/null, parse/native/materialization failures with unchanged typed components/solver state,
recursive getter behavior, expired previous-tick capabilities, and the unchanged
64-call localization limit.
The same actual query workload also runs without a recording and compares typed entity/physics/locale observations and the following solver step.
Legacy world saves correctly refuse these physics-bearing fixtures; they are not
used as a substitute checkpoint. Existing phase/navigation/budget controls retain schema and storage
assertions for their exact feature combinations.

The original source `a8de928d` was not executed: source review found that the
new fixtures attempted legacy saves of physics-bearing worlds, which correctly
refuse. Test-only successor `555498e3` used the supported state/next-step checks.
Actual supervisor `20459` then passed 20 collector controls (query/navigation/
budget), 3 prior-schema controls, 6 real ScriptHost controls and 7 ordinary query
controls. Its following strict lint failed on duplicate shared fixture modules
and two unused helpers; the complete failed run and its four copied executables
remain evidence. It was not reported as a passing qualification.

Successor `d6ef9bc4` reuses the existing localization/navigation fixture modules;
production code and all behavior assertions are unchanged. Actual supervisor
`93482` passed the six real ScriptHost controls again and both diagnostic/ordinary
strict all-target Clippy selections for profile, script and player. Thus there
are **36 feature-qualified controls, 42 passing executions, and two passing final
strict lint modes**. The 33 unique test names include three existing collector
controls exercised in both prior-schema and query-enabled modes. The 13 new controls consist of seven collector and six real
VM tests. Existing schema1 output and query-field absence are checked separately;
normal summary selection omits the new query feature. Formatting and convention
checks passed. These CPU checks do not run an ignored external-input fixture,
player window, GPU, audio device, capture helper or Tracy listener.

The two native attempts retained their original 900-second aggregate allowance,
12 GiB target ceiling and 6 GiB free floor. Actual combined supervisor time was
99.299201 seconds, recorded only for resource accounting. All observed child
processes/groups and both supervisors were freshly absent after terminal return.
Five executed test binaries were copied independently; all prior target history,
peaks and protected artifact identities were retained. No cache deletion or
fingerprint retirement occurred. Root independently replayed the native records
and subsequently passed all ten development checks at integrated source
`179eff78` (actual `27263`). Its raw logs and corrected data-only process-record
inspection are retained separately from the native producer results. The first run refreshed 1,911 bound local input
mtimes before Cargo to avoid stale cross-worktree metadata; the repair refreshed
only its two changed test files and rehashed unchanged contents.

The [portable packet](evidence/script-query-attribution/manifest.json) retains raw
Cargo/test/lint output, all source generations, first refusal/failure, resource
plans/closeouts and independent source reviews. Its data-only verifier runs with
`python3 -B docs/spikes/evidence/script-query-attribution/verify.py`. It does not
execute an archived runner or executable. Optional `--originals` rechecks retained
external raw records; `--binaries` verifies the five copied executable hashes;
`--source-root PATH` rehashes all 5,113 current producer compiler/source inputs;
`--git-source PATH` checks the recorded changed code bytes at all three historical
commits. Missing external artifacts or refs explicitly refuse those optional
checks. The archive carries identities, not executable binaries or a shipping
reproduction environment.

Independent [source review](evidence/native-script-query-attribution-source-20261018.json)
and [retained native-result review](evidence/script-query-attribution-native-20261018.json)
found no blocking issues. The integrated fifteen affected Rust/manifests match
qualified producer `d6ef9bc4` byte-for-byte; newer main changes are outside those
paths. The integration replay also rechecks all 88 archived records, originals,
five binaries and historical source identities without launching native code.


## Completed same-source shipping observation

The private adapter at `ddf97dac` adds an explicit query mode while preserving
older modes and public exporter admission. Producer `c81e82e2` joins the separately
qualified process-cleanup fix from `8a705821`; its Rust/runtime/compiler recipe is
unchanged from `ddf97dac`. The 4,887 source inputs, actual Node 22.23.3 executable,
193 installed compiler-package files, Cargo target feature recipients and actual
shipping code generation are bound. The query graph adds diagnostic features to
exactly four target crates. Both players retain the original fast physics recipe,
QuickJS bytecode, opt3/fat LTO/abort/one-codegen-unit settings, source-free packages
and no runtime authoring dependencies.

All **60 runtime input files** match the historical 74-file authored manifest.
The other 14 are authoring journal/recovery-history files, not workload content.
The normal empty-recovery epoch is used; no fake history or bypass was added.
The full 10,000-scripted-entity Core workload is unchanged. Compiled bytecode,
hardening bytes, source map, script symbols and all runtime package data match
between the fresh ordinary and query builds.

The first build attempt remains **failed**: exec session `18993` stopped after
231.688855 seconds, before a package or player child existed. Sampled owned RSS
8,620,343,296 bytes exceeded its 8 GiB cap. Immediate cleanup observation then
reported `Owned PID/group remains`, masking the initiating resource error. Later
fresh checks found every owned identity absent. The lifecycle repair preserves
operation and cleanup errors separately and allows bounded post-cleanup absence
observation without signalling retired IDs. No failed result was relabeled.

Reviewed successor exec session `70032` passed both fresh builds, the ordinary one-tick offline
startup, all package/compiler comparisons and cleanup in 573.056497 seconds.
Its 12 GiB RSS allowance was the sole resource ceiling increase; actual peak was
9,235,316,736 bytes. Both private build trees were removed; all 28 managed children
exited zero and the supervisor plus child PID/groups were freshly absent. Original
and successor charges total 804.745514 seconds, within the original 5,400-second
aggregate. The copied ordinary/query players are `d35be725` / `24458b9e`.

A separate quiet grant ran exactly ordinary then query, once each, at actual
exec session `74590` (supervisor PID 49207). Both completed all **2,156 steps, eight observation checks,
17 observed entities and every non-profile report field**, matching the historical
ordinary result. Both complete **13,798,400-byte PCM** streams match the historical
SHA-256 `ccabf2c68b6388ac554ac3efb5609ddf6b4a45d1a3f316891d5f344094e99ee3`.
There were no retries, omitted frames or warmup removal. The 45-second child /
120-second stage limits, 128 MiB output bound, 8 GiB RSS and 6 GiB free floor were
retained. Actual stage time was 12.782800 seconds; all 18 managed metadata/player
children and the supervisor were freshly absent. No window, GPU, device audio,
browser, computer-use or capture API was invoked.

The unchanged closeout helper carries the first failed build's 231.688855-second
charge into its pair closeout, so that raw file's `cumulative` value is **not** the
whole experiment total. The separate outer receipt explicitly records this
limitation and the correct all-build-and-pair sum, **817.528314 seconds**. Raw
records remain unchanged. Separately sampled monotonic accounting fields can
include a small amount of receipt construction; they are resource accounting,
not benchmark durations.

### Inclusive attribution

Machine: **Mac17,9 / Apple M5 Pro, 48 GiB, macOS 27.0.1 build 26A434, AC power**.
The ordinary ScriptTick p95 histogram interval was **2.100001–2.110000 ms**;
the instrumented interval was **2.140001–2.150000 ms**. One fixed-order pair does
not isolate instrumentation overhead or establish repeatability, savings or the
2 ms gate. The query report has all 2,156 valid successful rows, no open rows,
query failures or recursion; its 1,288-byte row reserves 5,275,648 bytes at the
4,096-row capacity, excluding final snapshots/JSON.

The tail below is the same **108 slow whole-script frames**, selected before
summing any query stage. Times are instrumented wall milliseconds, not exclusive
CPU time. Stages are inside call; calls are inside callbacks. Never add a parent
and its children.

| Observation | All 2,156 frames | Same 108-frame tail |
| --- | ---: | ---: |
| Whole ScriptTick | 3,265.764458 | 272.544834 |
| Both source callbacks | 2,507.986884 | 213.192085 |
| Query call | 219.988450 | 87.753706 |
| Query parse | 2.620122 | 0.218432 |
| Query execute | 207.447155 | 86.605200 |
| Query materialize | 4.641541 | 0.612250 |
| Call residual | 5.279632 | 0.317824 |
| Callback residual after query calls | 2,287.998434 | 125.438379 |

All **12,911 queries** belong to gameplay source slot 0: 8,260 character-motion,
2,066 trigger, 414 navigation, 2,156 locale and 15 localization queries. No `other`
query was omitted; its count is zero. The stress callback in slot 1 makes no
native queries and contributes 2,024.478553 ms overall / 105.261377 ms in the tail.
Its residual is not proven interpreter-only time: VM dispatch, bridge work and
observer bookkeeping remain included.

Parsing plus materialization accounts for **0.222%** of total whole-tick time
and **0.305%** of the same-frame tail. This run does not support pursuing query
marshaling as a substantial residual-cost optimization. Native navigation
execution dominates the query tail: 83.938711 ms, inside 84.458549 ms of navigation
calls. This does not revive the already rejected navigation-cache or repeated
start-scan experiments. Numeric preparation/validation remains separately visible
at 428.217131 / 428.336958 ms overall; no new optimization follows from that fact.

Next useful trigger: a specific callback/VM/typed-view validation implementation
change, or a distinct concrete hypothesis supported by retained data. Reassess
periodically during active development; do not repeat the unchanged PGO,
navigation-cache, deadline-check or query-marshaling experiment. A candidate still
needs its own full win/loss/restart gameplay and complete PCM controls and a fixed
matched quiet shipping comparison. The 2 ms requirement remains unmet.

### Portable replay and limits

The [shipping packet](evidence/script-query-shipping/manifest.json) contains raw
ordinary/query profiles and analyses, full gameplay anchor, plans, compiler/source
identities, selected build logs, both outcomes and closure records. Its verifier
recomputes the histogram, same-frame tail, query containment/totals and residuals
without executing archived code or a player:

```sh
python3 -B docs/spikes/evidence/script-query-shipping/verify.py
```

`--originals` checks all archived bytes against external originals; `--binaries`
checks both copied players; `--pcm` hashes the three complete external PCM streams;
`--compiler-inputs` checks all 193 installed compiler files; `--source-root PATH`
checks all 4,887 producing source inputs; `--build-logs` checks the three large
external Cargo records. Missing or changed optional inputs fail explicitly.
Default replay proves **recorded PCM identity**, not possession of omitted PCM
bytes. Executables, debug bundles, large Cargo streams, actual compiler packages
and PCM remain external; their identities are retained. This is data replay, not
shipping reproduction or a fresh measurement. The earlier CPU packet is unchanged.

Root independently reviewed the query adapter and replay verifier, rechecked all
74 portable records and optional external binaries/PCM/compiler/source/build-log
identities, and passed all 27 adapter controls on integration source `fa98d825`.
All ten local development checks passed at that source, including 1,040 UI tests.
The [integration receipt](evidence/script-query-integration-review-20261018.json)
binds retained raw logs and fresh absence of the wrapper and ten owned processes.
These checks do not rerun a player or change the qualified producer identity.
