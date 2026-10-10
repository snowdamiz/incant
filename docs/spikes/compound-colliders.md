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
walking regression covers 24 combinations, 300 ticks each, including travel over
adjacent compound-floor seams. The public CLI probe creates an arch and dynamic
dumbbell through atomic commands, rejects duplicate part IDs without changing
revision, and verifies part edits through Undo/Redo and journal reopening. Two
360-tick strict-TypeScript runs retain an open arch, return the parent hit ID and
settle the prop at 0.29993 m with exactly equal final state; author files stay
unchanged. The probe is included in macOS, Windows and Linux CI.

Actual macOS, browser/WASM and iOS-simulator execution uses two adjacent floor
parts, a sphere settling on their seam, and a primitive character sweep. All three
pass; macOS and browser return the same movement and settling values. These are
platform execution checks, not physical-device performance or a universal
determinism claim. Full visual/native, performance and hosted verification remain
open. Machine-readable results are in
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
