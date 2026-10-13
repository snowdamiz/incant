# Three-dimensional physics failure isolation

A late scene failure could leave the standalone 3D physics runtime exposing a
mixture of already advanced bodies and stale state or trigger observations.
A caller catching an unwind after advancement could also retry that solver.
The native ECS owner already refused failed sessions; this change closes the
standalone runtime boundary as well.

After validating the time step, the runtime becomes terminal until every scene,
trigger event and overlap observation has been published successfully. A late
error or unwind leaves it terminal. State and event readers expose no incomplete
frame, and synchronization, installation and removal cannot revive the instance.
Invalid time steps still preserve healthy observations and remain retryable.

The new regressions exercise a real late-scene range error and a caller-provided
ID clone panic during event collection after solver advancement. Both fail on
the unchanged base implementation; the invalid-time-step control passes there.
The repair passes all eight physics-runtime cases in each of the default and
fast8 modes, strict Clippy in both modes, sixteen native consumer cases and a
WebAssembly library compile. Formatting, conventions and whitespace checks pass.
Independent source review found no issues. Raw successful and expected-failing
logs are retained in the [evidence record](evidence/physics-failure-isolation.json).

This is terminal isolation, not rollback. Structural transactions and general
pre-step panic safety are outside this change. Physics saves and complete-world
restoration remain unsupported; no device or performance gate is claimed.

The repair is composed on main239. All 3,281 frozen build inputs equal the
reviewed producer except main239's explicit renderer diagnostic test/workflow,
which are absent from the selected physics test graph. Fresh current-main
qualification passes all eight default runtime cases, strict Clippy and four
scoped graph/format/convention/whitespace checks. Fast8, native consumers and
WebAssembly evidence remain producer-scoped; no duplicate execution is claimed.
The [current composition](evidence/physics-failure-main239.json) retains the
six commands and source comparison, alongside the unchanged original archive
and independent source/final reviews. Only the STATUS conflict needed resolution.
