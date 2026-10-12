# Native animation crossfade SDK

The TypeScript gameplay host and Rust no_std Wasm SDK expose the existing native
two-input crossfade through the ordinary tick-local structural command buffer.
No authoring path, model decoding, runtime compiler or additional dependency is
introduced. Core clocks/save rules are documented in
[native-animation-crossfades.md](native-animation-crossfades.md).

`commands.controlAnimation(id, 'crossfade', clip, seconds, duration)` accepts exact
primitive arguments: clip is an integer 0..255 or null for rest; seconds is finite
and nonnegative (rest requires zero); duration is finite 0..60 seconds. Actual
clip existence and duration are checked against the final authored native
animation declaration at atomic commit. Zero selects immediately. A nonzero fade
cannot interrupt an active fade. Select/Seek cancel it. No coercion, hidden pose
interruption, per-entity getter or parallel mutation route is provided.

TypeScript's `animations` retains its five destination arrays and adds six
aligned VM-owned copies: `transitionFlags`, `sourceClips`, `sourceSeconds`,
`sourceSpeeds`, `transitionElapsed`, `transitionDuration`. Transition flags are
active=1, captured outgoing playing=2, looping=4. Inactive rows have zero suffixes;
active source clips use -1 for rest. Destination contribution is elapsed/duration.
Master playing freezes both clocks and the fade. Speed/loop controls change only
destination settings. A fade started while paused captures a still outgoing
pose that remains still after resume. Retaining or mutating a copied array cannot
change playback or any later snapshot; VM allocations remain quota-bound.

## Explicit Wasm compatibility

The generated registry adds `incant_animation_v2` imports at opcodes 26–28:
`snapshot_len`, `read_snapshot`, `control`. Opcodes 0–25, signatures and legacy
records stay unchanged. The v2 snapshot header uses `INCAN002` and an exact
80-byte stride. Its first 40 bytes preserve the destination identity/clip/flags/
time/speed prefix; the suffix is u32 source clip, u32 transition flags, then f64
source seconds/speed/elapsed/duration. Inactive suffixes are entirely positive
zero; active flags cannot coexist with destination completion. Parsing validates
all rows, exact lengths/trailing bytes, flags, ranges and finite fields before
exposing any iterator. Bytes can be unaligned; they are never native pointers.

The 24-byte control frame stays copied. Existing kinds 0–4 retain their semantics;
only the v2 control import accepts kind 5 for crossfade, with target clip/time and
duration in value. The new Rust `AnimationControl::Crossfade` validates local
arguments before invoking that import. `Tick::animations` and `control_animation`
use v2 explicitly. `legacy_animations` and `legacy_control_animation` support
older hosts; legacy length/read both refuse while any transition is active, since
a single old row cannot faithfully describe the blend. The safe new parser
accepts both versions; an old parser refuses the new magic/stride. Unknown
versions, imports and signatures remain explicit failures.

Each snapshot is bounded to 100,000 rows: v1 at most 4,000,016 bytes and v2 at most
8,000,016. Host row/wire scratch retains bounded capacity across updates. When a
legacy frame is representable, both wire versions are staged at tick start;
this is not a total-process heap bound. All copied reads/commands use the existing
32-MiB guest memory, 128-MiB transfer, 65,536-call and 1,024-command ceilings.
Destination range/capacity is checked before copying; short buffers receive no
partial write. Active correct-epoch ingress errors latch rejection across parsing
and call/transfer/memory quotas. Stale/inactive direct calls retain prior semantics;
real imports still trap. An ignored safe-SDK error also prevents publication.
Any failed callback, native retarget validation or late invalid command discards
all staged numeric, structural, animation and log work and fails the host. This
cannot undo simulation already advanced before the callback or guest globals.

## Validation scope

The source freeze and final evidence record the actual local checks. Focused
TypeScript guests exercise distinct source instances, owned aligned snapshots,
read-after-queue invisibility, pause/resume, cancellation, malformed controls and
mixed-work rollback. A real guest starts a fade and ordinary NativeSession v5
fresh/in-place checkpoints continue identically through later controls.

The combined conformance helper compiles actual Rust samples/failure guests and
raw WAT, validates/meters them and links pinned WABT wasm2c code beside Wasmtime.
Tests inspect both clocks and captured paused-source flags, strict wire parsing,
zero/60-second admission, short/out-of-bounds frames, unknown tags/versions,
nonfinite durations, active retarget, quotas, traps and atomic rollback. A real
stateless Rust guest continues from fresh/in-place NativeWorld v5 restores in
fresh hosts using only world-held state. Full static NativeSession module-state
checkpoints remain explicitly unsupported; no Wasm memory/global persistence is
fabricated. The separate existing three-process bulk smoke is not a fresh
source-deleted crossfade shipping-export claim.

These checks are local CPU behavior/compile evidence. No GPU/window, rendered
appearance, physical device performance, general blend graph, IK/root motion,
complete game UI or phase gate acceptance is claimed. Canonical render binding
is a separate integration lane. The final frozen combined run passed 313 tests
and doctests across 54 groups, with no failures or ignored tests; all-target
Clippy, actual wasm32 library compilation, generated/graph checks, 22 strict
TypeScript cook tests and 20 Python tool tests passed. The independent source
review found no actionable defect. Exact source/artifact/log identities and
retained earlier failures are in
[the evidence](evidence/native-animation-crossfade-sdk.json).
