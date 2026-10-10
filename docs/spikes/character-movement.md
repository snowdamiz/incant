# Character movement — in progress

A read-only `api.computeCharacterMotion` capability now computes collision-aware
movement for a primitive collider attached to a velocity-kinematic body. It uses
Rapier’s controller for wall sliding, slope limits, optional stair stepping,
ground detection and optional snap-down. The calling TypeScript behavior supplies
gravity/jumping and applies the returned displacement divided by the fixed `dt`
through an ordinary Velocity command. There is no separate mutation interface.

Queries exclude the character and sensors, honor its authored collision masks,
and return stable collision IDs. Scene-root/unit-scale physics restrictions still
apply; rotating character bodies are rejected. The world’s up direction is +Y.
No mesh terrain, animated character, ragdoll or native play mode is claimed.
Moving-platform behavior and representative game performance still need review.

An unsimulated author edit uses a separate collider/BVH snapshot, preserving the
live solver’s modification flags. Stepped worlds borrow their existing query
pipeline. Each movement query costs 16 units in the shared 256-unit physics-query
budget. Native query return and script commit both check the script deadline;
catching a query error cannot commit an expired tick’s state or logs.

Five physics cases pass: wall slide/ground/read-only behavior; low-step versus
tall-obstacle traversal; slope limits and snap-down; mask/sensor/immediate edits;
and invalid queries without consuming live solver changes. Two additional
script integration cases pass for movement through commands, hot reload, author
isolation and the shared query budget. An injected native-query deadline test
verifies that an expired tick commits neither state nor logs; it is not a live
provider or device gate. Claude handoff 0023 owns actual rendered motion.

## Corrections from real motion review

Claude's first five-lane course exposed one-tick stalls on flat ground and during
wall sliding. A real PlaySession regression reproduced the floor stall at tick 6:
the controller returned no forward movement, only a 0.1 mm upward nudge. GJK had
returned an approximate floor normal such as `(0, 0.99999994, 0)`. Its slope
decomposition treated the tiny numerical downward tangent as prohibited slipping
and discarded forward travel. Increasing the nudge did not fix the underlying
problem and is not part of the final change.

The character-only query dispatcher normalizes cast normals and recovers exact
box-face normals when the witness lies within 0.1 mm of one face and strictly
inside its edges. Edge/corner and curved contacts retain their computed normals.
This does not change solver contacts or public raycasts. A regression executes
300 ticks for each combination of capsule/sphere/box, flat floor/wall slide and
autostep enabled/disabled: all twelve retain at least 95% of expected tangential
speed on every tick, stable support height and wall separation.

`sliding_down_slope` now requires downward travel beyond 10 micrometers and a final
support surface steeper than `min_slope_slide_angle`. The extra downward shape
cast includes support reached by snap-down; side-wall contact noise cannot classify
the floor as a slope. Tests distinguish uphill, downhill and flat travel.

The public CLI probe creates a scene in one atomic RPC transaction, compiles real
TypeScript under strict checking, walks into a wall, jumps, reverses and lands.
Two 360-tick runs produce exact final simulation/script states, and project/journal
hashes remain unchanged. Source and desktop CI now run this probe; all six platform
probes exercise an actual grounded character query.

## Integrated verification

The corrected tree passes 163 ordinary Rust tests, 40 real GPU checks, full
workspace Clippy with warnings denied, 315 UI tests/build, five Python tests and
generated schema/SDK/bridge/convention checks. Strict TypeScript checks include
both the public CLI example and the rendered course. The native editor release
build with embedded UI was packaged after the GPU suite.

Actual macOS, browser/WASM and iOS simulator execution passes the character query.
The browser and native reports both resolve a 0.1 m desired horizontal step while
grounded and correctly report no downhill sliding. The iOS probe invokes the same
runtime assertion and returns success. These are execution checks, not physical
device performance gates.

Claude Opus 5.5 over ACP accepted the corrected course in handoff 0023, after
fresh 300-tick runs from three cameras. All 303 frames and logs match an independent
repeat; all ten retained screenshots match the real engine originals. Per-tick
logs have no unexpected walking stalls with autostep enabled or disabled. The
five-character script/physics p95 is 0.54–0.56 ms on this M5 Pro in three runs;
the measurement is neither a cross-host comparison nor a game/device budget.

The 0.20 m step still follows the rounded capsule's edge trajectory: 18 ticks to
climb, roughly 0.13 s slower than covering that distance on flat ground. One tick
rises 2.7 cm. A two-tick downhill flag occurs while rolling down the sharp edge,
consistent with the support-normal definition. Constant-speed stairs, smoothing
and debounced animation transitions belong to later character/game-feel tuning;
the current API does not promise them. Moving platforms, mesh terrain, large-world
precision and representative physical-device performance remain unverified.

Hosted checks and merge are pending. See the [machine-readable evidence](evidence/character-movement-2026-10-09.json)
and [Claude's motion review](../../handoffs/0023-character-lookdev/result.md).
