# Physics runtime integration — in progress

The engine now uses pinned Rapier 0.36 with enhanced determinism. The fallback
choice and Jolt binding evidence are recorded in ADR 0003; no phase gate is
claimed. This working increment has not yet completed native visual review or
hosted checks. The full local workspace, actual GPU regression, browser and iOS
simulator checks below pass.

`RigidBody`, `Collider` and `AngularVelocity` are typed authored components.
Create and modify them through ordinary `incant_cmd` transactions, including
complete multi-component creation, Undo/Redo and script-generated edits. Shapes
are boxes (half extents), spheres and local-Y capsules. Fixed/dynamic/velocity
kinematic bodies, density, friction, restitution, gravity scale, damping, sleeping,
CCD, sensor overlaps and two-sided collision masks are implemented. Gravity is
world -Y at 9.81 m/s²; the document tick rate controls fixed stepping.

Each scene has an independent world. The ECS publishes updated transforms and
linear/angular velocities to the isolated play bus before scripts run. Scripts
write the same component commands; the next sync changes only affected solver
state. Unchanged body handles, contacts and sleep state survive every tick. The
authored project and its journal never change during play.

`api.raycast` is read-only, normalizes a nonzero direction, uses world meters,
returns a stable entity ID, distance, point and normal, and supports masks,
sensors and exclusion. Exact-distance ties use stable IDs. Up to 256 queries per
tick share the script deadline. `api.triggerEvents()` returns sorted sensor
entry/exit transitions for the completed tick. Native query bindings survive
compatible hot reload; plain ScriptHost use without a play session fails clearly.
API structural types are generated from Rust schemas.

PhysicsRuntime tests exposed that Rapier 0.36's collision-only refresh clears
new-body flags without registering simulation islands. Incant therefore never
uses that refresh before stepping: queries after author edits inspect current
shapes directly, while stepped worlds use Rapier's BVH. Tests cover immediate
queries, removed/reused internal handles and exact agreement between an untouched
world and a world echoed through document synchronization at every tick.

Current implementation boundary: physics entities must be scene roots with unit
Transform scale; size the shape explicitly. These restrictions are validated on
the complete transaction and never silently flatten a hierarchy. Colliders alone
are stationary. Fixed bodies reject nonzero velocity. Unsupported shapes fail.
Position and velocity ranges are validated; a numerical solver failure stops
further simulation until play restarts. Character controllers, mesh/compound
colliders, hierarchy/scale support and serialized rollback remain open.

Initial new behavior checks: seven physics runtime cases, two script integration
cases and one command-bus atomicity/Undo/Redo case pass. They cover settling and
sleep, replay from authored state, exact sync preservation, all three primitive
shapes, high-speed CCD, kinematics/rotation, masks, scene isolation, sensor
entry/exit, ray hit/exclusion/tie/invalid input, script impulse-by-velocity,
query budget, hot reload, friction/restitution/gravity/damping and unchanged author data. This is deterministic local
behavior evidence, not live-device or complete engine acceptance.


## Integration evidence

The final integrated workspace passes 154 ordinary Rust tests, Clippy, 312 UI
tests/build, five Python tool tests, Rust format, generated bridge/SDK and
convention checks. The earlier 40 explicit real-GPU checks pass; all 26 final
physics look-dev captures are byte-identical to Claude’s reviewed engine frames.
The final native bundle builds with embedded frontend assets. Native macOS and
actual browser WASM execute the integrated falling-sphere probe for 120 ticks and
produce the same final Y of 0.49993008375167847 m. The new iOS simulator app also
executes the real core/physics contact assertion and reports status 0. This proves
this integrated workload runs there; it is not a physical-device performance gate.

`python3 tools/probes/physics-runtime.py <new-output-directory>` exercises public
CLI/RPC creation, atomic unsupported-scale rejection, Undo/Redo, persistent
reopening and play/stop. Two 360-tick runs have exact runtime snapshots. A compiled
TypeScript behavior raycasts the real floor and changes Velocity at tick 121;
its body rises to 0.581562 m by tick 122. Hashes prove the authored project and
journal are unchanged. Desktop CI now runs this probe on macOS/Windows/Linux;
platform CI includes the actual physics assertion on all six targets.

[Machine evidence](evidence/physics-runtime-2026-10-09.json) records build/log
hashes and current pending review. [Supplemental binding audit](evidence/physics-binding-audit-2026-10-09.json)
records the additional published Jolt binding checks behind ADR 0003.


## Simulation overhead

A 512-body, 600-tick workload includes the solver, ECS, simulation command bus,
no-op JavaScript and document synchronization, with sleeping off and CCD on.
Initial p95 measurements varied from 11.7–13.5 ms under concurrent development
load. Removing a duplicate snapshot, a state-map clone, and duplicate project
validation preserves the exact final runtime snapshot. Validation remains shared
through an immutable `ValidatedProject` proof, which cannot be constructed or
held across a project mutation by callers. The atomic invalid-sync tests pass.
Three alternating before/after trials give median p95 values 9.994 ms and 8.798 ms.
All samples, including a 17.774 ms baseline outlier, remain in the evidence. These
are local analytical measurements with concurrent development work, exclude
rendering, and do not pass a game/device performance gate.


## Inspector integration

The shared registry now supplies physics field order, units and collision-mask
widget metadata. The bridge accepts only unambiguous tagged object unions.
Claude’s read-only Inspector presents known shapes, grouped material and solver
fields, raw collision-mask values with group summaries, and exact nested/axis
diagnostics. Unknown or missing tags retain their values and show an explicit
mismatch. Schema order takes precedence over presentation grouping. Damping
units were corrected to 1/s before the final native build.

Claude approved the browser fixture at 1440×900 and 1000×650 and actual headless
physics motion. Final native CUA captures cover the three shapes, AngularVelocity,
damping, collision fields, keyboard focus and scroll/resize behavior at 1440×874
logical and the true 1000×650 minimum. Native visual acceptance is pending Claude’s
review. Captures remain ignored because the titlebar contains the saved account
label; hashes and sanitized accessibility evidence identify the review artifacts.
The saved account restored without another login or Keychain prompt. The native
viewport is an authored static scene; no play-in-editor controls are claimed.
