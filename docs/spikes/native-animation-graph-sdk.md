# Native Wasm animation graph observations and controls

This slice extends the public Rust module SDK and both native Wasm hosts. It
connects the existing bounded authored state-machine/one-dimensional blend
implementation to copied observations and transactional controls. It does not
add graph authoring, two-dimensional or nested blend trees, IK, root motion, full
guest checkpoints, mobile measurements or a phase-gate acceptance.

## Versioned contract

The registry retains every published function at opcodes 0–30. New imports are
`incant_animation_v3::{snapshot_len,read_snapshot}` at 31–32 and
`incant_animation_graph_v1::{snapshot_len,read_snapshot,control}` at 33–35. All
records are explicit little-endian bytes with stable big-endian IDs. Generated
Rust/C dispatch and codecs come from the registry; the runtime never parses it.

A v3 pose row retains the complete 80-byte v2 prefix and adds the actual second
pose, its sample time, its kind and the destination contribution (104 bytes total).
Timed transitions retain both clocks and their elapsed/duration. Parameter blends
have two clip samples and a weight without inventing a timed transition. An
inactive suffix is canonical zero. The v1/v2 layout and refusal behavior remain:
v1 cannot represent timed blends; neither old layout can represent parameter
blends. The SDK default `animations` selects v3; `transition_animations` selects
v2 and `legacy_animations` selects v1. Unsupported hosts reject the new imports.

The graph snapshot has a 24-byte header, sorted 64-byte entity/state rows and
sorted 24-byte parameter rows. Offsets are contiguous with no orphan parameters.
It exposes the explicit master-pause flag separately from natural clip completion,
state clocks and stable authored parameter IDs/values. It transfers no asset
handles, graph definitions, project records or native pointers. Limits are 1,024
graph instances, 16 parameters each and 458,776 copied bytes. Elapsed state time
is at most 3,600 seconds; long clip time is not truncated to that limit.

A 32-byte copied control sets one parameter or selects one authored state. Tags,
reserved fields and finite global bounds validate before queuing; authored ranges,
entity existence and state/parameter membership validate in the native atomic
transaction. Selection resets graph clocks and preserves master pause. Numeric
writes, structural commands, playback controls and primitive logs all commit or
reject together. Ignoring an authorized boundary or SDK error cannot publish the
prefix. A stale epoch cannot poison a different current update.

Snapshots capture committed state at callback start. Queued commands cannot alter
the same callback's readback. The v3/graph byte frames are encoded lazily on first
access and retain bounded allocation capacity between updates. Caller-owned
initialized buffers receive complete copies only; invalid or short destinations
remain unchanged. Existing memory/call/transfer/structural quotas still apply.

## Verification status

Corrected production sources passed independent review. The final actual-host
suite passed 355 tests/doctests across 61 groups, scoped all-target Clippy and
three fresh development processes. The prepared bundle contains 109 modules and
546 independently rehashed files, including twelve new graph guests. Supported
SDK wasm32 compilation, generated contracts, formatting and tooling checks pass.
Python ran 231 tests with four existing skips. The earlier 74-test shared-boundary
suite overlaps the final suite; these are not additive distinct counts.

The independent P3 finding is fixed: a forged parameter blend with a rest
destination now refuses. Its reproducer and regression are retained. An earlier
test setup wrongly reused a terminal aborted capability, one fixture used an
invalid module name, and lint checks found fixed-chunk/test-style issues. Each
original log remains in [the combined receipt](evidence/native-animation-graph-sdk.json).

The added actual Rust guest compares both hosts with a separate native oracle for
parameters, pause/resume, state selection, both blend endpoints and captured pose
values. Failure guests cover ignored buffer/value errors, authored range/key/entity
errors, command quotas, callback failure and traps, preserving the complete world
and withholding logs. Prior v1/v2 WAT controls and transition suites remain active.
