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


## Resumed Claude review, 2026-10-10

The director reauthorized computer use. Handoff 0025 reviewed the corrected
runtime: two runs of 228 frames are byte-identical, with arch/terrace motion and
falling props accepted and no new solver defect. Native CUA captures verified
mixed/64-part lists, End/Tab focus, exact part paths and full IDs, scrolling and
primitive regression. Claude accepted the captured appearance scope. Account
email-bearing originals remain ignored; committed Inspector crops exclude it.
Native images are CUA JPEG at Retina scale; the OS sharing indicator obscures
the unchanged traffic lights. Native reordering and invalid-project display are
not claimed: the Inspector is read-only and invalid projects reject loading.

Claude then reduced the unavailable Agent pane's default height to 200–240 px,
while retaining deliberate user resizing and the ready Agent's normal size.
The browser minimum-size Inspector gains 60 px; all 334 UI tests/build and the
32 browser states pass. The integrated native build at `19c3be9` is packaged,
but final post-change captures wait for the Mac to be unlocked. This is a device
availability issue, not a renewed screen-capture prohibition. PR #25 remains a
draft until Claude confirms the rebuilt native layout and updated CI passes.


Integration with main `68f6004` includes the now-merged audio and input actions.
All 237 Rust tests, 40 explicit GPU checks, 334 UI tests/build, five tool tests,
Clippy, strict TypeScript, generated contracts and the native release build/package
pass. Public compound, audio/timer and input-action probes pass. Updated hosted
CI and post-layout native appearance acceptance remain pending.


## Final native review and navigation integration, 2026-10-10

Claude handoff 0025 now accepts the rebuilt native Inspector at 1440×874 and
1000×650: mixed/64-part lists, exact paths, End→Tab full IDs, primitive regression
and compact unavailable-Agent allocation. The reviewed app is source `5b97ecd`,
SHA `f23c403fceb9bca3418bd0dc317e9e316c808fe665eb9fc8fb99748afabf5224`.
Astra quit the old process and reopened the rebuilt bundle through CUA before
capture. Full account-bearing JPEGs stay ignored; Claude committed account-free
Inspector/Agent crops. Native reorder and invalid-data display remain outside
the read-only/rejection boundary. The OS sharing pill obscures traffic lights.

Integrating current main exposed navigation baking's missing compound-shape
case. Baking now combines each primitive's conservative tessellation after its
local rotation and translation, in stable part-ID order. It preserves openings
instead of substituting the union's bounding box. A real engine regression
bakes an arch with a rotated box pillar, capsule pillar and beam, checks the
straight route through its opening, reverses presentation order, then inserts
a sphere and verifies re-baking forces a route around the closed arch. This
regression passes. At `fc60bcf`, the complete integration passes 315 Rust tests,
Clippy, 334 UI tests/build, five tool tests, generated contracts, native release
build/package and WASM/iOS simulator core compilation. The frozen headless binary
passes the public compound, saved navigation, 100-agent steering, off-mesh links
and weighted-grid workflows, including exact repeats and saved restoration.

The 40 GPU checks passed on `debe4ed`; the renderer is unchanged by the later
navigation/script integration, and PR #40 independently passed the same 40. This
is retained coverage, not a claim of another combined-head GPU run. Native UI
source is unchanged from Claude's accepted `5b97ecd` capture. Updated hosted CI
is required before merge. Machine-readable results are in
[evidence](evidence/compound-navigation-integration-2026-10-10.json). No phase
gate or physical-device gate is claimed.
