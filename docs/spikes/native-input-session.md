# Native fixed-tick input session

Date: 2026-10-10. This Phase 1 slice runs typed input through the native cooked
Transform/Velocity world and synchronous QuickJS host. It adds no legacy
PlaySession/document tick path and claims no device or performance gate.

## Ownership and tick ordering

`NativeSession::new(CookedScene, CompiledScript, InputActions)` owns a fresh world,
VM, physical-input timeline and immutable `ValidatedActions`. The cooked scene's
checked tick rate supplies both native simulation and input/script `dt`; a caller
cannot advance this session with a different clock. `NativeWorld::tick_rate()`
and its storage are the same accessor change as the reviewed native-save work.
This constructor always starts at tick zero; there is no saved-session resume.

Bindings validate once when constructing or explicitly rebinding a session.
`ValidatedActions` owns a private immutable Arc map; its read-only definitions
cannot be modified behind the validation. `InputRuntime::advance_validated`
performs bounded physical-packet validation and action evaluation without
repeating authoring-map validation. Existing `advance_mapped` and `map_actions`
retain their behavior for existing callers; the release workspace regressions
include the legacy input consumers. Rebinding reseeds held named values without
inventing action edges or advancing the physical clock. Invalid rebinds preserve
the old bindings and frame.

A tick validates/accepts the complete ordered event packet first, then advances
the native simulation, then runs the synchronous script and applies its queued
structural commands after loan revocation. This simulation-before-script order
matches the legacy ordering; a velocity changed by the script affects subsequent
native integration. There is no project clone, command-bus call, component JSON
conversion or action-map validation in this tick path. The existing bounded
InputRuntime transaction copies only input state.

An invalid packet leaves clock, pending edges, gestures, native world and VM
unchanged, so it can be retried. Once input is accepted, any runtime/script error
permanently fails the session: subsequent tick, rebind, input observation and
world/saveable-state access return `SessionError::Failed`. Script numeric writes
and structural batches keep the host's rollback, but the preceding native step
and accepted physical packet are not rolled back. No whole-world/VM rollback is
claimed. The failure flag is set before native stepping, so caught Rust unwinding
after packet acceptance also cannot resume the public session.

## Direct input binding and limits

The native context is `update({ chunks, dt, commands, input })`.
`ScriptHost::tick` remains available with an empty input frame; the additional
`tick_with_input` accepts `&InputRuntime`, whose validated state is private. It
does not accept arbitrary publicly constructible `InputFrame` values. The host
still owns only the script phase; NativeSession owns fixed-tick sequencing.

Raw keyboard/mouse/gamepad/touch state, all four gestures and named actions use
the existing shared InputFrame schema. Values are constructed directly as JS
own data properties, using null-prototype records and explicitly defined array
indices; guest prototype setters/getters cannot execute during this conversion.
No serde/JSON projection, native callback handles or borrowed component pointers
enter the input snapshot. Copies live in the VM and may be retained without
keeping an ECS loan or changing subsequent Rust input. The SDK exposes a recursive
readonly `NativeInput` type, and the native-input TypeScript example consumes
named actions alongside the structural example in the standalone proof.

The existing engine bounds apply: 1,024 events per packet, 16 gamepads, 16 active
touches, 64 named actions and 16 bindings per action. Frame collections are
bounded by those packet/device limits (including focus-loss release/cancel
records). JS copies use the VM's 32 MiB memory cap and 50 ms thread-CPU budget;
the deadline is checked after input construction and command binding, before
any numeric ECS loan/publication. Component views retain separate bounded undo,
revocation and validation. Input snapshots contain no closures retaining native
allocations. These are resource limits, not measured device frame budgets.

## Standalone startup exchange

The embedded player preserves its existing empty-input invocation and also
accepts:

```text
incant_script_player world.scene 60 --input-recording input.json --input-actions actions.json
```

Reads are bounded before JSON deserialization: scene 128 MiB, recording 8 MiB,
action map 256 KiB. InputReplay checks the entire typed event history before any
world tick; rate must match the cooked world, start must include fresh tick zero
(therefore start_tick zero), and the clip must cover all requested ticks. Binding
validation happens at session construction. Unknown, duplicate, conflicting and
manual-input arguments reject explicitly. Tick execution only reads retained
typed event slices and immutable bindings. These JSON files are runtime exchange
formats, not cooked binary assets or native-session saves.

The shipping probe's historical `inputs_and_build_storage_removed` boolean means
authoring/compile inputs and temporary Cargo storage were deleted. Explicit
`removed_input_scope` and `runtime_exchange_files` fields identify the intentionally
retained `input.json` and `actions.json`. TS/CommonJS, original project/journal and
build-produced executable-cache storage are absent before either proof path runs.
Source maps stay in the separate matched-symbol directory.

## Verification and measured artifact

All **394 release workspace tests/doctests** pass; the normal suite retains its
**44 ignored GPU cases**, which were not executed locally for this nonvisual
change. Workspace/all-target Clippy and compiler-disabled host Clippy pass with
warnings denied. The scoped input/runtime/script/player suite passes **89** tests
and doctests. All 25 Python tool tests, strict SDK TypeScript, generated
SDK/bridge/conventions checks and runtime dependency guards pass. Cargo.lock adds
only local-package dependency edges; no external version/checksum changes.

New behavior tests cover immutable-map equivalence, fixed rates 1/30/60/120/240,
simulation-before-script order, sub-tick raw/named edges, gesture clock preservation
across forty rejected packets, retry, atomic rebind, failed-session inaccessibility,
retained input snapshots and detached component loans, every gesture/device
schema field, hostile prototype accessors, deadline before publication, a full
1,024-event snapshot, and bounded/compatible player startup files. The numeric
host fixes reviewed separately are included: component validation after revocation
and safe fresh-borrow rollback, with no cached-pointer dereference after revisits.

A fresh arm64 macOS shipping build runs **24 standalone processes**: the original
six tick counts (0/1/2/3/4/60), each twice without input, plus those same counts
twice with recorded raw input, named actions, tap gestures, controller disconnect
and focus changes. Both paths preserve the cooked scene and reproduce exact
component writes/structural behavior. The actual target compile commands verify
opt-level 3, one codegen unit, panic abort and player fat LTO. Source-parser symbols
remain absent, bytecode execution remains present, and dynamic source routes fail
in the actual compiler-free player.

Final stripped player: **2,154,408 bytes**, **+364,272 bytes** against the corrected
1,790,136-byte cooked-script baseline on the same host/target/profile. This whole
fixture-player delta includes startup JSON decoding, input/session code and the
changed guest fixture; it is not an isolated input binding cost or performance
measurement. SHA256:
`d71e3706602f5090b65fee9165823b0f8bd9d6c263bf3286e2c8cdf14c671b60`.
Matched arm64 dSYM UUID: `30be507f-ade5-388b-b1ce-4aa1786f2efd`.
[Recorded proof](evidence/native-input-session-2026-10-10.json) includes compiler,
codegen, symbol identity, observations and explicit retained-file scope.

Commands (all local Cargo compilation used `-j2`):

```sh
./tools/cargo test -j2 -p incant_input -p incant_script -p incant_script_player -p incant_runtime --locked
./tools/cargo test -j2 --workspace --release --locked
./tools/cargo clippy -j2 --workspace --all-targets --locked -- -D warnings
./tools/cargo clippy -j2 -p incant_script --no-default-features --all-targets --locked -- -D warnings
python3 tools/probes/cooked-script.py artifacts/native-input-final-shipping --headless-binary /Users/andreyyurlov/.codex/worktrees/cooked-native-scripts/incant/target/release/incant_headless --cook-binary /Users/andreyyurlov/.codex/worktrees/cooked-native-scripts/incant/target/release/examples/cook_scene
python3 tools/check_runtime_dependencies.py incant_input incant_types incant_runtime incant_script incant_script_player
npm run test:tools
node tools/generate_sdk.mjs --check
node tools/build_bridge.mjs --check
node_modules/.bin/tsc -p sdk/ts/tsconfig.json
python3 tools/generate_conventions.py --check
```

The existing read-only authoring/cook fixture tools above come from the corrected
cooked-script worktree; the fresh shipping executable is built from this native
input source in a disposable target directory. Raw proof is ignored under
`artifacts/native-input-final-shipping/`. Logs:
`/tmp/incant-native-input-{tests,workspace,workspace-clippy,no-compiler,tools,final-shipping}.log`.
The existing general CI shipping step runs the extended proof without a second
shipping build or a new large dependency cache. Hosted results for this slice
remain pending publication.

## Remaining scope

This is fresh native input/session execution, not persistence or complete gameplay
migration. VM state/input save/restore/seek, live device adapters, remaining legacy
SDK systems, general exports and native phone execution remain open. Startup
InputReplay seeking support does not imply session/VM seeking. Shipped embedding
still requires host==target; the symbol audit supports Mach-O/ELF and explicitly
lacks MSVC parser-symbol auditing. The existing native schedule/derived-hierarchy
overflow limitation for huge finite component values remains documented in the
[cooked-script follow-up](cooked-native-scripts.md). No physics/backend adoption,
AOT, 100k-entity performance or reference-phone gate is claimed here.
