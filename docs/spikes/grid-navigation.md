# Rectangular grid navigation

`NavigationGrid` stores typed row-major cell costs on an ordinary entity. Zero
blocks a cell; positive integer weights from 1 to 1000 are walkable. Dimensions
are 1..1024 on each axis, with at most eight grids and 65536 cells across the
project. Edits use ordinary shared commands, validation, provenance and history.

`api.findGridPath` queries one grid in one scene. Coordinates are cell indices;
the result lists adjacent cells including both endpoints. Four-way or eight-way
movement is explicit. Diagonal moves require both adjacent orthogonal cells to
be walkable, preventing corner cutting. Actor clearance and world transforms
remain the caller's responsibility; this query does not add sprites, tilemap
rendering, collision generation or 2D physics.

Weighted A* uses an integer planner metric: entering-cell weight multiplied by
1000 for straight moves or 1414 for diagonals. This is a declared approximation
to Euclidean distance, not a reported world-space length. An admissible octile
or Manhattan heuristic uses the smallest walkable weight. Stable row-major ties
make equal-cost results repeatable without random choices. Start-cell cost is
not charged. A blocked endpoint or unreachable destination returns null; malformed
input, exhausted search or a path over 4096 cells fails explicitly. Search is
bounded by a requested 1..65536 expansion count.

Each query costs 64 of the sandbox's 256 shared native-query units. Read-only
queries see the start-of-tick grid; successful component edits become visible
on the next tick. Derived immutable snapshots reuse unchanged components and
publish only after full engine synchronization succeeds. Per-grid generations
identify changed snapshots within a session; they reset on process/save reopen
and should not be used as durable gameplay identity. Authored data and saved
runtime component edits remain the source of truth.

Five library tests cover weighted detours, blocked corners, exact costs against
an independent exhaustive-relaxation oracle, stable ties, invalid input, bounded
search and an overlong corridor. Four integration tests cover shared command
validation/provenance/Undo/Redo, atomic runtime synchronization, next-tick edits,
hot reload, save/reopen, failed-query rollback and project-wide storage limits.
The public strict-TypeScript probe changes route costs from 6828 to 9828, closes
both passages, saves while blocked, reopens a passage and arrives. Three blocked
ticks and the final runtime/script states match uninterrupted and repeated
playback exactly; authored files remain unchanged. Durable shared Undo/Redo,
invalid-grid rollback and the project-wide grid count limit pass.

The integrated branch passes 309 Rust tests, 315 UI tests/build, five tool tests,
full Clippy and generated contracts. After the Windows CPU-budget regression test
correction, all 54 script tests and Clippy pass again. This test-only correction
does not change the reviewed binary's production code. No phase gate is approved.

A separate public script measurement fills the 65536-cell bound, with a walkable
but isolated destination forcing search of 65532 reachable cells. Three runs of
30 queries completed: p95 whole-script-tick times 10.268, 9.726 and 9.799 ms;
maxima 10.373, 9.796 and 9.941 ms. These include host synchronization and ran on
this Mac during concurrent compilation. They exclude rendering, are not a
controlled benchmark or device gate, and do not guarantee a frame deadline.
The repeatable helper is tools/probes/navigation-grid-scale.py.


Integrated source `fd9f46a` includes the final steering correction, off-mesh links,
bounded script diagnostics and executing-thread CPU budgets. All 309 Rust tests
passed at `ac81c4b`; the next change only includes the already-computed grid result
in the portable probe's JSON. Clippy, generated contracts and the native core
probe pass at `fd9f46a`. All three public grid/steering/off-mesh save workflows
pass on its immutable binary. The maximum-size grid measurement repeats with
p95 10.813, 10.921 and 10.971 ms, maxima 10.861, 11.082 and 11.133 ms. The same
local workload limitations apply; this is not a controlled before/after study.

All 40 GPU tests, the final editor build, native development packaging (including
the CPU clock license), and WASM/iOS-simulator core compilation pass. These target
compiles do not establish an interactive device or scripting-VM gate. Claude
handoff 0029 passes its scoped rendered-movement review on the frozen binary.
Two independently authored runs match all 516 frames and logs; their states
match after strict mapping of the 18 generated project/scene/asset IDs. Save at
tick 156 while unreachable reproduces runtime and script state exactly. Reopened
log messages differ only in session-local grid generation. Every route cost
matches the independent Dijkstra oracle; arrival is at tick 476, with 0.2 m
minimum footprint clearance and no corner cutting. Astra verified all 14
committed screenshots against both raw runs, audited shared mutation routing
and the strengthened checker, then reran all 309 Rust tests and full Clippy.

The behavior chooses to finish its current cell step when a route becomes null;
this is not an API requirement. Route overlays see the start-of-tick grid, and
linear turns and small/occluded markers remain documented visual limitations.
Hosted checks remain pending. See the [Claude result](../../handoffs/0029-grid-navigation-lookdev/result.md)
and [exact evidence](evidence/grid-navigation-2026-10-10.json).
