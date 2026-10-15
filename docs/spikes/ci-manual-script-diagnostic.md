# Explicit selection for the manual script diagnostic

Checkpoint 43's Linux desktop job reached the workspace ignored-test command,
then failed in `actual_native_stress_regions_128` because
`INCANT_CORE_PHASE_INPUT` was absent. That fixture needs an explicitly prepared,
hash-bound Core stress input directory. Its previous `test + profile + compiler`
condition became reachable through ordinary workspace feature unification.
The later missing gameplay/export artifacts were consequences of this earlier
failure, not separate successful or attempted gameplay runs.

`incant_script/diagnostic-core-stress` now explicitly selects this manual fixture
and all fourteen of its internal timestamp hooks. The feature is default off and
implies the existing compiler and profiler capabilities. It remains test-only;
enabling the feature in a production library cannot add these hooks. The ignored
test still refuses missing or invalid prepared input when deliberately selected.
Its source, assertions, workload and timing interpretation are unchanged.

The [probe instructions](../../tools/probes/core-script-phase/README.md) include
the new selection. No workflow, required context, GPU test, assertion or broad
ignored-test command is removed. An independent source inventory found 127
workspace ignored-test declarations; this fixture was the only normally reachable
one requiring separately prepared external input. Other manual output requirements
are already under explicit features, or are optional evidence destinations.

Local compilation with ordinary profiler/compiler features selects zero ignored
script-library tests, including a direct executable run with the input variable
absent. Deliberate selection passes all 128 native script ticks and the three
omitted-write negative controls. Strict all-target Clippy passes in both feature
modes, along with workspace formatting and generated conventions. This is
functional diagnostic coverage, not shipping performance evidence.

The [compact receipt](evidence/ci-script-diagnostic.json) records the actual
commands and hashes. Corrected Linux/Windows/Mac hosted execution remains pending;
release promotion stays held until the complete checkpoint passes.
