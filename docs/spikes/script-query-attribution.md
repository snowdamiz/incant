# Native query stages inside ScriptTick

This default-off diagnostic observes the existing TypeScript query path. It does
not optimize it or change the 2 ms target. Source and controls are prepared; no
native qualification, full Core run or performance result is claimed here.

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

- **call:** from the closure's first statement through its return, including
  ordinary early refusals. It excludes bridge argument collection before entry.
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
are additional. Future diagnostic execution must bound the final JSON to 32 MiB;
there is no per-tick JSON serialization or per-query allocation in these hooks.
Counters use checked accumulation. Time is monotonic wall duration, not exclusive
thread CPU cost; observer work remains included.

The outer report version remains diagnostic-only version 4. The nested schema is
`1 + navigation + 2*budget + 4*queries`: existing feature combinations remain
schemas 1–4; query combinations use 5–8. This preserves existing feature-off JSON.
The old private analyzer rejects the new schema instead of silently accepting it.
A subsequent explicit query-aware adapter must require the selected exact schema,
category/region arrays, bounds and complete raw coverage; unknown schemas/features
must refuse. Nothing here changes public exporter admission.

## Prepared CPU controls

Collector controls cover every category/stage and early error, recursive getters,
missing/reordered/overlapping/leaked scopes, duration/count overflow and backward
clock observations, no recording, foreign-thread exclusion, retired capture
lifetimes and caught unwind. Real ScriptHost controls exercise native character
movement, trigger events, localized text/missing keys, number formatting, path
hit/null, parse/native/materialization failures with unchanged world bytes,
recursive getter behavior, expired previous-tick capabilities, and the unchanged
64-call localization limit.
The same actual query workload also runs without a recording and compares saved
world bytes. Existing phase/navigation/budget controls retain schema and storage
assertions for their exact feature combinations.

## Proposed full Core diagnostic protocol

After CPU qualification and separate resource review, extend the existing private
Core attribution adapter with one explicit query mode; no public exporter or
project capability gains diagnostic authority. Freeze and audit its exact target
feature graph and compiler recipients before building. From one source identity,
produce an ordinary summary-profile player and a query-diagnostic player with the
same original 74 authored files, 10,000 scripted entities, closed ordinary physics
recipe, Rust opt3/fat LTO/abort/one codegen unit and unchanged QuickJS C recipe.
Keep source-free program/package and data identities explicit for both builds.

Run the full 2,156-tick route once per player after source/build storage removal.
Require identical complete gameplay observations, checkpoints/events and all
13,798,400 bytes of 48 kHz stereo f32 PCM. Retain rejected/partial runs and stop on
first failure; no retry, warmup removal, workload simplification or timing verdict.
Use fixed 60-second player / 150-second combined bounds, 32 MiB per report and
96 MiB combined outputs, plus the ordinary process ownership and storage guard.
Final executable/plan bindings and any adjusted resource proposal precede launch.

The query analyzer must correlate all rows to actual whole ticks and source
slots, recompute coverage, and select the same slow whole frames before reporting
stage totals. Reject incomplete rows, recursion, stage counts exceeding calls,
unknown categories, nonfinite/overflowed values and mismatched features. Preserve
all failures; do not silently discard invalid rows. Report non-Core `other` use
explicitly before interpretation. Query call times belong to callbacks and stage
times belong to calls. The remaining callback time still includes VM dispatch,
argument setup, uninstrumented host work and observer overhead; it is not proven
pure interpreter cost. No marginal-p95 subtraction or predicted saving follows.

Only a distinct, substantial measured subpath would justify a later small change.
That later change needs its own full win/held-out loss/restart gameplay and PCM
parity and fixed balanced quiet shipping comparison, including PlayerFrame and
memory. This diagnostic alone cannot adopt an optimization or pass the 2 ms gate.
