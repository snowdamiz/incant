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
All nine pass locally; core/script/headless compilation and focused Clippy pass.
Schema regeneration, strict SDK and the public TypeScript probe now pass. The
public probe changes routes from cost 6828 to 9828, closes both passages, saves
while blocked, reopens a passage and arrives. Three blocked ticks and the final
runtime/script states match uninterrupted and repeated playback exactly; authored
files are unchanged. Durable shared Undo/Redo and invalid-grid rollback pass.
All 285 Rust tests, 315 UI tests/build and five tool tests pass. Full Clippy also passes.
GPU/native checks and
Claude's rendered review remain in progress. No phase gate is approved.

A separate public script measurement fills the 65536-cell bound, with a walkable
but isolated destination forcing search of 65532 reachable cells. Three runs of
30 queries completed: p95 whole-script-tick times 10.268, 9.726 and 9.799 ms;
maxima 10.373, 9.796 and 9.941 ms. These include host synchronization and ran on
this Mac during concurrent compilation. They exclude rendering, are not a
controlled benchmark or device gate, and do not guarantee a frame deadline.
The repeatable helper is tools/probes/navigation-grid-scale.py.
