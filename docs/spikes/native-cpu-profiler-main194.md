# CPU profiler integration on main194

The accepted default-off CPU profiler is composed with actual main194. The current
union passes 495 affected CPU tests in 62 groups, 19 default-player tests, all-target
Clippy, default/summary wasm compilation, focused exporter tests, dependency guards,
formatting and generated conventions. Independent integration review is pending.

The tested source is `8cc9a729869b5bf290bdf38641b70ef5b63efb6c`; the ancestry join at
`5dfbc826324b18e40bdd512402cfcd9344305d7e` retains actual main194
`071cdf1b7c235702948e27f15d36cda43755577f` without changing compiled inputs. Nine
exact raw-log attributes preserve accepted producer bytes and qualify only terminal
blank-line checks for those evidence files. Original dependency-guard invocation,
raw-log whitespace and linked temporary-fixture failures remain retained separately.

All three selected normal player graphs (default, summary and Tracy) match the
producer's exact dependency versions and features after checkout-path normalization.
The 659 source inputs in those local package closures match except the already
reviewed quota-budget test, which is excluded from ordinary player builds by
`cfg(all(test, feature = "compiler"))`. The sole lock delta is the renderer's existing
rustix edge, outside all three graphs. The desktop exporter, profiler helper and
profile implementation inputs are unchanged. Build-host changes are mobile-only
static-Wasm admission; the desktop TypeScript path is unchanged. Five old/new
selector controls verify identical admission/refusal without invoking a build.

The original two fresh shipping packages, five 3,600-tick player processes, five
precise refusals and partial localhost Tracy capture remain **producer-scoped**.
The integration collector rehashes all 353 retained/original records, 28 package
files and three external binaries, and checks exact logical-report/PCM equality
and profile counters in the five recorded processes. No new current-main archive,
shipping process or capture is claimed. This source-relevance boundary avoids
repeating unchanged shipping builds while making the tested identities explicit.

[Manifest](evidence/native-cpu-profiler-main194.json),
[read-only verification](evidence/native-cpu-profiler-main194/collect.py),
[source relevance](evidence/native-cpu-profiler-main194/source-relevance.json) and
[graph relevance](evidence/native-cpu-profiler-main194/graph-relevance.json) retain
the exact current checks and qualified reuse. The feature remains opt-in; no GUI,
GPU, native-window, phone, performance gain or full PLAN profiler gate is approved.
