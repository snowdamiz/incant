# Named gameplay input actions

`settings.input_actions` is typed, validated project data. Authors, agents and
gameplay scripts replace it through `set_input_actions`, using the existing
atomic command bus, provenance, durable history and Undo/Redo. Empty maps are
omitted from canonical JSON, preserving pre-existing empty settings and project
hashes. This adds runtime mapping; the Phase 2 graphical binding editor remains
open and no visual layout is changed.

## Definitions and snapshots

An action has kind `button`, `axis1` or `axis2`, and zero to sixteen bindings.
An empty binding list means unbound and produces a neutral named action. Up to
64 actions are permitted, with names of 1–64 ASCII letters/digits or `_.:-`.
Supported sources are physical keys, mouse buttons, controller buttons, controller
axes or sticks, and tap/long-press/swipe gesture pulses. Controller bindings name
the normalized device ID; automatic player assignment remains open. Gesture
actions last one tick. Raw input remains available alongside named controls.

Scalar sources have a two-component `scale`, default `[1,0]`, with finite values
in [-1,1]. Buttons need nonnegative X and zero Y; axis1 needs zero Y. Whole sticks
require axis2 and can invert Y. Buttons take the maximum contributing value and
activate at a configurable threshold in (0,1], default 0.5. Axis bindings add;
opposite directions cancel. Axis1 clamps magnitude to one and remaps its dead
zone; axis2 uses a radial dead zone and normalizes to a unit disc, preventing
faster diagonal keyboard movement. Dead zones are [0,0.95], default 0.15.

`api.input().actions[name]` returns `{kind,value:[x,y],active,pressed,released}`.
Axis activity means a nonzero value after dead-zone processing. One-axis/button
actions use X only; button values retain their analog magnitude. Snapshots are
copied, so script mutations cannot alter another read or the next tick.

The input processor evaluates action transitions after each ordered physical
event, preserving a down/up pair within one tick. Multiple alternatives holding
one action do not create duplicate presses or early releases. Edge flags indicate
whether a transition happened, not a count of transitions. Focus loss clears
presses, releases active controls and neutralizes analog values; disconnects
release actions owned by that controller. Invalid maps or event suffixes change
neither physical state, actions nor gesture time.

## Rebinding and persistence

Changes made during a behavior tick apply on the next input tick. A new map is
baselined from held physical controls; remapping itself does not fabricate press
or release edges. Consumers should use `active` to observe the resulting held
state, and a gameplay rebinding flow can reset its own action-dependent state in
the same script transaction. This policy also makes saved replay continuation
independent of obsolete binding history.

Runtime bindings are part of the saved project. The save resource check keeps
project identity, tick rate and asset/script manifests fixed while allowing
binding changes. Authored project/script revision checks remain in force.
Loading a save starts physical controls neutral; supplying an input recording
seeks physical history and seeds named values without replaying prior presses.
The following tick processes real recorded transitions normally. No new save
format or recording version is needed.

## Verification and measured scope

Six input tests cover alternative bindings, short taps, axes, dead zones, diagonal
normalization, cancellation, thresholds, disconnect/focus, touch gestures,
unbound actions, rebinding, limits, atomic rejection and replay seeding. A command
test verifies legacy serialization, agent provenance, Undo/Redo and invalid
transaction rollback. A script test changes bindings while controls are held,
saves immediately, resumes input in a new session and checks exact subsequent
state, save data and log equality. The full workspace passes 212 Rust tests,
315 UI tests/build, five tool tests, Clippy, generated contracts, strict
TypeScript and the native release build/package.

The public `tools/probes/game-actions.py` probe creates a project and bindings
through RPC, tests Undo/Redo and invalid-prefix rollback, typechecks/compiles a
TypeScript behavior and records keyboard, gamepad, focus and touch events. The
script uses action values to move the player and rebinds at tick 6. Saving after
tick 7 and resuming 13 ticks in another process preserves exact final state and
logs. Four intended presses occur at ticks 1, 9, 15 and 17; the held remapped key
does not create an extra press. Final position is `[0.275,0,0.3]`, matching the
independently calculated movement. Authored project/journal/recording hashes stay
unchanged. Existing public save, input/character/assertion and timer probes pass.

The CPU-only `actions_bench` example runs 200 samples after warmup. On this local
Apple M5 Pro, 12 actions × 3 bindings × 4 events measures median 0.002291 ms and
p95 0.002500 ms. The maximum 64 actions × 16 bindings × 1,024 events measures
median 2.582917 ms, p95 2.803333 ms and maximum 2.966541 ms. This synthetic workload
exercises repeated digital transitions; it excludes device polling, physics,
scripts, rendering and slower reference hardware. It does not pass a full-game
or mobile performance gate.

Platform smoke now checks named edges, a controller dead zone, focus release and
replay equality using synthetic events. Local macOS execution passes; hosted
checks remain pending. No app was opened and no local captures were made.
Physical adapters, hardware calibration, chords/contexts, automatic per-player
device routing, pointer/wheel action sources, touch regions/virtual controls,
haptics and the graphical binding editor remain open.

Machine-readable evidence: [input-actions-2026-10-10.json](evidence/input-actions-2026-10-10.json).
