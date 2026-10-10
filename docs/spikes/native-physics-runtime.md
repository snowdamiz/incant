# Native cooked rigid-body runtime

## Scope and ownership

The first native physics slice loads typed 3D rigid bodies, colliders and angular
velocity from cooked scenes into the disposable ECS world. `NativeSession`
advances that world before its synchronous script update, using the cooked tick
rate. The tick path contains no document projection, command-bus transaction,
JSON, authoring-schema validation, full world clone or physics-configuration
rebuild. It uses the existing Rapier backend; this is not backend adoption or a
performance-gate result.

`incant_physics_runtime` owns the solver and typed validation without authoring or
navigation dependencies. Its ordered generic ID preserves the legacy facade's
exact case-sensitive text IDs, while native storage uses the existing 16-byte
`StableId` API (shared through `incant_types` and re-exported by `incant_runtime`).
Legacy query/character schemas and behavior remain compatible. The legacy
character query's cloned collider path is not exposed as a new native query API.

Native builds retain Rapier 0.36 parallel plus enhanced determinism. WASM retains
the serial solver. This does not introduce a fast/SIMD product variant or replace
the separate raw-solver serial/joint parity and Jolt candidate timing evidence.

## Cooked and structural contracts

`CookedPhysics` is a keyed table alongside the unchanged `CookedEntity` API.
Each entry has an existing entity ID, optional rigid body, required collider and
optional angular velocity. Physics entities must be roots with unit scale; all
existing typed shape, body, motion and compound-part domains are checked.
Stable IDs, declaration pairs and compound part identities remain exact.

Scenes without physics still emit and accept version 1. Version 2 appends an
explicit little-endian physics table with bounded counts, tagged shapes and
strict flags; it does not serialize Rust layout or JSON. Unknown versions/tags,
malformed IDs, invalid pairs/order/counts, invalid booleans, truncated data and
invalid UTF-8 fail explicitly. The existing digest detects corruption, not an
authenticated publisher. The scene limits remain one million entities, 128 MiB
and 64 parts per compound collider.

The cooker recognizes `Transform`, `Velocity`, `RigidBody`, `Collider` and
`AngularVelocity` at the authoring boundary. Unmigrated components still fail
explicitly. A rigid body without motion receives zero linear/angular components;
a collider-only entity can remain in a Transform-only table.

Native `FrameCommands` gains `SetPhysics` and `SetAngularVelocity`. Angular
commands, including `None` (reset to zero), require a rigid body in the final
batch state. A rigid body cannot lose its `Velocity`; use zero instead.
Removing physics leaves linear motion as an ordinary native mover. All final
physics/topology/numeric checks finish before ECS or solver mutation. Numeric
velocity/angular batches stage only touched IDs; configuration/topology batches
may rebuild ID/parent metadata and clone changed configuration, not the world.
Unchanged solver bodies retain their handles, contacts and sleep state.

No JavaScript physics-configuration/angular-command or query binding is added.
Existing Transform/Velocity loans and structural commands use the same native
validation. Configuration inspection is an explicit cloning API outside ticks;
state/events expose typed values and IDs, never solver pointers or handles.

## Step, script and failure semantics

Every step checks all current numeric inputs and candidate ordinary integration
in reusable hierarchy scratch before the first solver write. Motion-only solver
sync compares the last published state, so untouched stationary/sleeping bodies
are not woken. After the solver step, candidate solved poses and composed globals
are checked before publishing ECS components, derived transforms and the clock.
Ordinary integration excludes physics-bound entities in both preflight and
commit, preventing double integration.

A pre-solver validation failure leaves solver/ECS/clock unchanged and can be
retried through the native world API. A failure after solver advancement leaves
the published ECS/clock unchanged and poisons the world; no solver rollback is
claimed. Later stepping/structural operations fail, physics inspection/events
are withheld, and `NativeSession` treats an accepted packet followed by runtime
or script failure as terminal.

Untrusted script loans revoke every foreign view before numeric/hierarchy and
physics validation. Invalid numeric or structural writes restore the whole
numeric undo set through fresh native borrows and permanently fail the host.
Trusted native immediate-write APIs retain their documented responsibility and
do not gain automatic rollback. The new integration adds no unsafe code.

Sensor transitions are ordered by exact stable IDs. The backend bounds active
sensor pairs at 65,536 and events at 131,072; overflow is terminal rather than
publishing a partial native frame. Despawned-pair exits are delivered by the next
successful step, including when the last physics entity has been removed.

Native save/restore rejects any world that has used physics, even if all physics
is later removed. Loading a forged save containing a physics scene also fails.
This deliberately preserves the boundary around solver/contact/event history;
plain native Transform/Velocity/hierarchy saves retain their behavior. Physics,
VM/input and complete-session save/restore are not claimed.

## Verification — 2026-10-10

All local checks below passed on the frozen implementation (including the final
mixed angular/velocity fast path). All local Cargo builds/tests/Clippy used two
jobs and the pinned lockfile; source inputs stayed fixed while Cargo ran.

| Check | Observed result |
| --- | --- |
| `./tools/cargo test -j2 -p incant_runtime -p incant_script -p incant_cook -p incant_physics_runtime -p incant_physics --locked` | 119 tests/doctests passed |
| `./tools/cargo test -j2 --workspace --release --locked` | 423 tests/doctests passed; 44 existing GPU tests ignored |
| Workspace all-target Clippy, warnings denied | Passed |
| Compiler-disabled `incant_script` all-target Clippy | Passed |
| Release headless/cooker build and UI production build | Passed |
| 30 Python tool tests, generated schemas/SDK/bridge/conventions, strict SDK TypeScript, actionlint, formatting | Passed |
| Resolved runtime dependency guard under workspace feature unification | No authoring dependency for types, physics runtime, world, host or player |
| Four runtime/facade roots across seven targets | All 28 feature graphs preserve native parallel plus enhanced determinism, WASM serial |
| `incant_runtime --lib` checks for iOS device/simulator and Android | Passed; compilation only |

[Validation/source hashes](evidence/native-physics-validation-2026-10-10.json),
[target feature graphs](evidence/native-physics-feature-graphs-2026-10-10.json),
[full parity report](evidence/native-physics-parity-2026-10-10.json) and
[shipping player observations](evidence/native-physics-shipping-2026-10-10.json)
retain the measured results. The independent review's final-state angular reset
finding was fixed and its observable atomicity regression and numeric fast path
were re-reviewed without further findings.

The observable regressions cover unchanged legacy schemas and case-sensitive
IDs; binary/string solver streams; stationary/sleeping bodies; collision and
compound shapes; kinematic/ordinary motion without double integration; body
installation, replacement and despawn; final-state angular requirements and
atomic mixed numeric batches; whole-batch validation before any solver sync;
post-solver hierarchy overflow; bounded sensor events; save refusal; and script
view detachment, complete undo and permanent failure on invalid physics edits.

The separate native ECS parity probe executes actual cooked `NativeWorld`s:
35 bodies, 640 fixed dyadic steps plus initial state, direct numeric wake edits,
typed angular commands, contact/sleep/wake and sensor transitions. Each record
includes exact 16-byte ID, all 13 public position/quaternion/linear/angular
numeric fields and sleep state. Repeated one/four-worker native processes are
compared byte-for-byte with fresh actual Chromium/WASM serial processes. This
does not claim hidden solver-state equality or a broad physics workload gate.

Final command:
`python3 tools/probes/native-physics/run.py --output artifacts/native-physics-parity-final --browser --wasm-bindgen /tmp/incant-pinned-tool-live/wasm-bindgen/wasm-bindgen`.
Six native processes and three fresh Chromium 156.0.8078.4 processes passed.
All nine 2,714,667-byte streams have SHA-256
`a2a870b8ce1ae3be839dd968a39f8fdcc259c374f06e5c1080674e9dc6f2df4c`;
the workload reached 32 sleeping bodies, 16 wake transitions and four sensor
transitions.

The source-free shipping probe preserves the existing 24 empty/recorded-input
processes and adds eight normal-player physics runs at 0, 1, 60 and 600 ticks.
It checks falling/contact/sleep, fixed floor, angular motion and single kinematic
integration after deleting the authoring/script compile inputs and temporary
build storage. Startup input/action exchange files and cooked scenes remain as
intentional runtime data. Compiler-disabled source routes and matched split
symbols are audited on the actual packaged executable.

Final command:
`python3 tools/probes/cooked-script.py artifacts/native-physics-shipping-final --physics`.
All 32 fresh processes passed on macOS arm64 with Rust 1.99.0. The stripped
player is 3,941,816 bytes, SHA-256
`1339df08dfe10453a50cb275ea6ae1dc79834f4d8db899465188418b859de378`,
with matching arm64 dSYM UUID `b1d8affe-951b-3b24-a9de-cceaef229ed0`.
Actual target compiler commands confirm optimization level 3, one codegen unit
and aborting panics for the runtime, script/input host, physics backend and
Rapier, plus fat LTO on the player. The whole fixture/player is 1,770,848 bytes
larger than the preceding 2,170,968-byte numeric-boundary player; that is neither
an isolated physics cost nor a frame-time result. This local proof inspected
Mach-O/dSYM, not Windows PDB or Linux symbols.

Hosted CI retains the original solver candidates and timing outputs, adds the
native ECS stream to the existing three native jobs with serial browser
execution on Linux, and compares cross-host artifacts separately. Local target
compilation and feature graphs are not device execution. Native character and
navigation, 2D/joint components, full legacy scripting SDK, physics saves,
mobile exports and reference-phone/shipping performance gates remain open.

## Publication union with numeric/audio extraction — 2026-10-10

The physics branch merges the reviewed numeric and cooked-audio stack through
audio head `8fef22cb3a101270cc4a6527ffe8b09cbef75ec2`. Integration source head
`d4a8505e41cd18c3637854bb17e830762954ff32` has tree
`0481a0088f5b0243c10bb3b8a2560b62f787ed59`. Physics/runtime/cook/script/type
source is unchanged from reviewed `32a720c`; audio/assets source is unchanged
from the reviewed audio stack. Both runtime guards/platform path sets, all tool
cache/timing changes and existing probes are retained. The only additional Rust
change is the copied-player test isolation described below. Audio loading/mixing
remains separate from future native ECS audio integration.

The [union report](evidence/native-physics-audio-union-2026-10-10.json) retains
source hashes, target features, full body streams, both shipping proofs and log
identities. The changed dependency union passed 162 scoped runtime/physics/audio/
asset tests and 428 release workspace tests/doctests (44 existing GPU tests
ignored), workspace Clippy, compiler-disabled script Clippy and the audio device
feature compile. All 38 tools tests, generated schema/SDK/bridge checks, strict
SDK TypeScript, conventions, formatting and actionlint pass. The resolved
dependency guard covers seven runtime roots under workspace feature unification.
The current 28 feature graphs and nine native/actual-browser streams pass again;
the body-stream SHA remains `a2a870b8…` above. The earlier mobile runtime checks
remain compile-only evidence for the unchanged physics source.

Fresh union shipping observations are separate from the pre-union identities:

| Executable / observed processes | Stripped bytes | SHA-256 | Matching arm64 dSYM UUID |
| --- | ---: | --- | --- |
| Compiler-disabled physics/input/script player, 32 | 3,941,816 | `87c14258a11c923ba0635d697db1521d1e7b7bf75e132a922470d883ed2bf10c` | `1cc810aa-38af-365f-b80c-34326d0148b2` |
| Separate cooked-audio player, 64 | 585,072 | `825acac5f630e559a23ad2e4dbceca021a13da59e74f8e3105d8b121a1084013` | `89de1499-9297-3a3e-9bc4-631e133e3338` |

Both used their normal source-removal proofs on macOS arm64. Physics retained
the empty/recorded-input and cooked rigid-body checks; audio retained static/
streaming modes, playback-rate and render-batch invariance. These are separate
executables, not proof that native world audio or a full game export is complete.
Raw results remain in `artifacts/native-physics-audio-union-{shipping,audio}`.

### Copied-player test process isolation

Two Linux jobs (main53 and PR54) failed at the copied player's first process
launch with `ExecutableFileBusy` / `ETXTBSY` 26. That test copied an executable
while the other test in the same Rust test binary could spawn children. The
pinned Rust 1.99 [file implementation](https://github.com/rust-lang/rust/blob/b940084d7eb6a299eb4bfeb8e34901bc051e7ac4/library/std/src/sys/fs/unix.rs)
opens the copy's writer with close-on-exec; the [process implementation](https://github.com/rust-lang/rust/blob/b940084d7eb6a299eb4bfeb8e34901bc051e7ac4/library/std/src/sys/process/unix/unix.rs)
uses spawn/fork paths. Close-on-exec does not remove the descriptor from a child
before exec occurs.

A controlled Linux 7.0.14-linuxkit arm64 probe reproduced that mechanism in all
16 trials: a forked child retained an `O_CLOEXEC` writer, the parent closed its
writer, the copied binary still failed with error 26, and it ran successfully
after the child exec closed the descriptor. The pinned, network-disabled Python
container identity, complete probe source and observed results are in the union
report. This supports the race diagnosis; the precise hosted interleaving was
not traced.

The isolated fix `1de1f209c4d43c38523741b2d80dcad957c74940` adds a test-local mutex
around both process-launching tests. It preserves the actual copied executable,
every assertion and immediate launch-error failure; it adds no retries or
production changes. The existing two-test release run and matching Clippy check
pass after the fix using the workspace feature graph. Upstream PR54/55 received
the same fix; the physics merge preserved its already validated tree exactly.
