# Compound colliders — in progress

The authored Collider union now includes one-level compounds of 1–64 local
primitive parts. Parts retain stable ULIDs and their own translation/quaternion,
while the parent entity supplies one body, material, sensor state and collision
masks. Primitive volumes contribute additively to mass/inertia, so overlapping
parts are not a boolean volume union. Nested compounds fail explicitly. Existing primitive documents remain
compatible. All authoring still uses the shared atomic command bus.

Preparation sorts parts by stable ID without changing authored presentation order.
Reordering parts therefore preserves live handles, contact state and sleep rather
than rebuilding the body. Runtime raycasts and sensor events identify the parent
entity. Character bodies must remain primitive; compound obstacles are supported.
The character-only box-face normal correction also resolves faces inside compound
parts, preserving continuous motion across their planar surfaces.

Initial checks pass 168 Rust tests. New behavior tests cover rotated parts and
hollow openings, dynamic contact and exact future-state equality while reordering,
one enter/exit event across overlapping sensor parts, masks, explicit capability
failure and atomic command rejection with Undo/Redo/journal recovery. The expanded
walking regression covers 36 combinations, 300 ticks each, including travel over
adjacent compound-floor seams and equivalent geometry authored in rotated local
frames. Review found a 32% movement loss at tick 77 with quarter-turned parts;
normalizing the recovered face after its quaternion transform fixes that loss.
Every configuration now retains at least 95% tangential progress on every tick. The public CLI probe creates an arch and dynamic
dumbbell through atomic commands, rejects duplicate part IDs without changing
revision, and verifies part edits through Undo/Redo and journal reopening. Two
360-tick strict-TypeScript runs retain an open arch, return the parent hit ID and
settle the prop at 0.29993 m with exactly equal final state; author files stay
unchanged. The probe is included in macOS, Windows and Linux CI.

Actual macOS, browser/WASM and iOS-simulator execution uses two adjacent floor
parts, a sphere settling on their seam, and a primitive character sweep. All three
pass; macOS and browser return the same movement and settling values. These are
platform execution checks, not physical-device performance or a universal
determinism claim. Final native appearance and hosted verification remain open. Machine-readable results are in
[evidence/compound-colliders-2026-10-09.json](evidence/compound-colliders-2026-10-09.json).

Claude handoff 0024 owns the object-array Inspector and real rendered look-dev.
Mesh terrain, hierarchical body ownership, local material/mask overrides, nested
compounds and physical-device game performance remain outside this increment.


## Local simulation cost

`python3 tools/probes/compound-cost.py artifacts/compound-cost` creates every
project through the public command bus. Thirty-two awake dynamic bodies each
contain 1, 4, 16 or 64 parts tiling the same unit-cube extent. Three 180-tick runs
include the solver, ECS, document validation/synchronization and no-op JavaScript;
CCD is enabled and rendering is excluded. Median p95 costs on this M5 Pro are
0.754, 1.158, 3.396 and 11.578 ms respectively. The largest observed tick is
15.128 ms at 64 parts. All samples are retained, including development-load
variation; repeated playback final states match and author files stay unchanged.
This small synthetic workload shows that high part counts are costly, even when
the visible extent is identical. Prefer a simple primitive when it describes the
shape. These measurements do not satisfy a complete-game or target-device gate.


## Integrated checks and paused native review

The corrected tree passes 168 Rust tests, 40 explicit GPU checks, 329 UI tests,
Clippy, strict TypeScript, generated schema/SDK/bridge checks and the native
release build. Public primitive, character and compound CLI probes pass again.
Claude's object-list Inspector is integrated; one further correctness fix follows
the selected part's stable ID and preserves row focus through reordered snapshots.
The new regression first failed on index-based selection, then passed.

Before the director paused all computer use and capture on 2026-10-09, native CUA
checks confirmed the three-part list, End/Tab navigation, exact nested paths, full
stable ID and saved-account restoration without prompts. Final wide/minimum native
appearance and 64-part review are pending; no acceptance is claimed for them.
Code, numerical replay, tests and CI continue while capture is paused.

## Claude replay and numerical investigation

Claude completed 28 browser-fixture states with zero axe violations, clipping or
horizontal overflow. The original runtime produced 228 repeated engine frames
with byte-for-byte equality. After capture was paused, two corrected-runtime
replays have exactly equal state/logs; only two characters differ from the original
runtime, by at most 0.18 mm at the final position. Props and arch motion are equal.
These numerical results do not substitute for the pending native appearance review.

A suspected stool instability was investigated with 301 numerical states and an
independent additive mass/inertia calculation. The original report mistook a
near-upright moving body for rest and overestimated the tipping-energy barrier.
Release energy is 80.4162 J, never exceeded by the sampled simulation; the final
value is 14.0716 J. A simple point-foot barrier estimate is 2.4283 J above upright,
below the release's 9.3252 J excess. Contact steps are not strictly energy-monotonic
(the largest local increase is 0.8868 J), so this is a fixture-specific rejection
of the original diagnosis, not a general energy-conservation guarantee. No solver
patch was made. A fresh project built through public commands repeats the trace
exactly without captures. The helper supports `--no-captures`; the reproducible
[energy probe](../../tools/probes/compound-stool-energy.py) and
[measurements](evidence/compound-stool-energy-2026-10-09.json) retain the method.
