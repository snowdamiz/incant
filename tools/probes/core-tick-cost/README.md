# Core tick diagnostics

These standalone experiments inspect candidate costs without changing the Core
workload or engine. They are not shipping, native host, startup or gate evidence.
Production changes require separate review and matched shipping measurements.

`prepare.mjs DESTINATION` refuses an existing destination and derives six exact
variants from the checked-in `samples/core/stress.ts`: unchanged baseline,
count accumulation outside the loop, cached length, forward cursor, ordinary
structure-of-arrays metadata, and locally bound structure-of-arrays metadata.
All three writes and both actual checksum readbacks remain per owned row. No
Float64Array metadata or unrolling candidate is adopted from the earlier slower
diagnostics. Generated sources, fixture and their hashes are retained.

The generated Rust fixture uses pinned QuickJS 0.14.0 and Bevy 0.19.1. Generate
its lockfile offline and freeze it before compiling with Rust 1.99, the explicit
native target, profile `ci`, two jobs and `RAYON_NUM_THREADS=2`. The isolated
manifest and lockfile are diagnostic inputs, not the engine feature graph. Use a
task-specific target; never share another lane's target. Retain actual argv,
stdout/stderr, tool identities and copied executable immediately.

The JS diagnostic records all 500 calls including initial cache creation and
checks all rows, controller exclusion, padding and sentinel at ticks 1/64/500.
Then untimed controls change the fresh view's stride with identical IDs, reorder
IDs, and replace the attachment snapshot's ordinal mapping. Omitted actual writes
and NaN in either sentinel checksum must fail the independent finite row oracle.
This fixture does
not implement native capability revocation; existing engine tests remain
required for any production adoption. No mean or percentile from this diagnostic
may be presented as the script phase p95.

The Bevy experiment compares current `World::get<Identity>` against safe typed
`QueryState` prepared afresh for each immutable batch. Every token still checks
the world ID, entity generation and live ID. Query preparation/allocation is
included in each batch's recorded interval; no query state survives a mutation.
Actual archetype movement, despawn/ID reuse, wrong ID, foreign world and replacing
the world distinguish the controls. It uses synthetic native rows and reports
the complete raw intervals; it does not exercise the Incant host or authorize
skipping any lifetime check.
