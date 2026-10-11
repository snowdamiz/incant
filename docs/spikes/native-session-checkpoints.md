# Opt-in native session checkpoints

This slice adds a production `NativeSession` ordinary-load checkpoint API. It
preserves committed typed gameplay state, live authored attachments, native
source-scoped timers and world state across a new process using trusted embedded
bytecode. It does not serialize an arbitrary JavaScript heap, resume closures,
or complete the Phase 1 game-save feature.

## Explicit script contract

A behavior opts in with `persistence: { version: 1 }`. Every active behavior must
opt in; checkpoint creation refuses an active behavior that has not done so.
Module initialization runs again on load. All module globals and closures are
**ephemeral**; gameplay state required after load must be in the native world or
`context.state`. This is a new explicit contract, not inferred compatibility with
legacy state hooks. Static/Wasm modules remain unsupported by this checkpoint API.

`state.get(entity?)` and `state.set(value, entity?)` address either source state
or a live attachment of that source. Values are finite typed data: null, boolean,
number, UTF-8 text, arrays and objects. Functions, accessors, symbol properties,
class instances, cycles, sparse/extended arrays and non-finite values refuse.
Observations are owned and deeply frozen. The changed-key overlay commits with
numeric/structural commands, timers and logs only after an accepted script update.
There is no per-tick project clone or JSON state encoding.

Stable source/instance IDs and original immutable props come from the actual
compiled attachment catalog. Despawn permanently retires an attachment; spawning
the same ULID does not revive its script or instance state. Source timer keys are
native `(source ULID, local ID)` pairs. Retired inactive sources do not silently
run callbacks. State scoping does not restrict the existing whole-world authority
of numeric/structural/query APIs.

Expired or wrong-source retained state functions throw without poisoning the new
source update. Invalid data/quota calls on the active state API poison the whole
update even if the guest catches the JavaScript error.

## Trusted load boundary and atomicity

The APIs are:

- `NativeSession::checkpoint_bytes()`
- `NativeSession::from_checkpoint_bytes(bytes, &trusted_initial_scene, program, actions)`
- `NativeSession::restore_checkpoint_bytes(bytes)`

The factory receives the actual trusted initial cooked scene and compiled or
embedded program from its host. The data contains no executable code. Compatibility
covers the actual bytecode, hardening, dynamic-code capability, engine/build/target,
attachment catalog and native runtime source identity. The original cooked-scene
fingerprint, scene ID, fixed tick rate and immutable localization tables must match.
Current locale settings and mutable gameplay state may differ. These checks happen
before starting the candidate VM. Checksums detect corruption; origin fingerprints
and compatibility identities are **not authentication**, signed provenance or
anti-cheat protection.

The runtime supplies a shared read-only immutable-binding guard. Factory loading
does not construct a second baseline ECS world, clone the live world, or share its
host identity. Existing world-only v1/v2/v3 restore behavior remains compatible;
session checkpoint v1 specifically requires a nested bound world-save v3.

Replacement publishes only after the entire candidate validates. Failure leaves
the current world, VM, persistent state, timers and input untouched. Success gets
a new world identity/incarnation; retained external hosts must be rebuilt. Sticky
physics/audio capability refusal also applies to the current destination, even
if the corresponding declarations were subsequently removed.

Ordinary loading resets physical buttons, axes, gamepads, gesture/input timing and
transient edges to neutral without inventing release/press edges. Host action
bindings are trusted startup/current settings. The host must discard old OS events
and explicitly reconnect devices. This is not exact physical-input replay.

## Binary and resource limits

`INCSES01` version 1 has a 128-byte header with program and original-scene
fingerprints, four checked section lengths, input-policy tag and a SHA-256 checksum.
The sections contain world-save v3, canonical live attachment IDs, typed persistent
data and the native schedule. Integers and finite numeric data have explicit
little-endian encodings; no Rust-layout or executable deserialization is used.

| Resource | Bound |
| --- | --- |
| Complete checkpoint | `MAX_SAVE_BYTES + 2 MiB` |
| Nested world | Existing `MAX_SAVE_BYTES` and scene validation |
| Live authored attachments | 32,768 canonical unique IDs |
| Retained persistent data | 1 MiB charged encoding, 65,536 nodes, depth 16 |
| Per-update state operations | 256 across every source |
| Per-update transferred data | 1 MiB / 65,536 nodes across every source |
| Timer records | 128; local IDs 1–64 ASCII bytes |
| Script simulation tick | Exact JavaScript-safe integer through `2^53 - 1` |

Source/instance key accounting reserves 34 bytes. Decoder tags, booleans, lengths,
canonical ordering, UTF-8, finite numbers, depth, live membership, scope and timer
clock/due/period relationships are checked before publication. These are logical
resource limits including explicit bounded candidate overlap, not a universal
peak heap/RSS guarantee. Explicit save creation can copy bounded state; the tick
path only stages changed keys and already-existing native schedule work.

## Verification and corrected failures

Final affected runtime/input/script/cook tests pass: **319 tests/doctests in 45
result groups, zero ignored or failed**. This includes eleven checkpoint cases
and five persistent-data cases. Four-package all-target Clippy, compiler-disabled
static-module compilation, normal runtime dependency guard, strict SDK typing,
generated SDK/bridge checks, formatting and conventions pass. The shipped normal
script/runtime graph remains free of authoring, command bus, CRDT and agent crates.

Tests cover scoped values/timers, intentionally reset globals, attachment retirement
and ID reuse, neutral input, maximum scoped timer rows, unsupported behaviors,
physics refusal, current locale restoration and malformed/oversized/deep/non-finite
sections. Checksums are recomputed in adversarial scene/rate/catalog tests; both
load entry points reject before VM initialization and preserve the current session.

Two implementation failures were reproduced and corrected before intake. First,
`loan.finish()` evaluated before persistent validation, prematurely disarming
numeric rollback on a caught invalid write; validation now precedes disarm. Second,
native state closures captured JavaScript inspector/freeze roots that QuickJS could
not trace when a guest retained the API, causing a `JS_FreeRuntime` assertion. The
fixed engine JavaScript wrapper retains traced references and passes them as
call-local arguments; native callbacks retain only Rust-owned values. Retained API,
proxy/getter failure and caught-error teardown regressions pass. Original failure
logs remain identified in the evidence rather than being treated as environment
flakiness.

The reusable `tools/probes/native-session-checkpoint.py` proof builds actual strict
authored TypeScript through the trusted source compiler, embeds its bytecode and
initial scene, copies the executable, and deletes its disposable copied engine,
source and build storage before launch. Two sources and three instances advance
independent counters plus same-named source-scoped periodic timers. An uninterrupted
40-tick process is compared with a 13-tick save followed by two fresh independent
27-tick restores. The complete world/state/schedule checkpoint bytes must match;
corruption must refuse without writing output. The final receipt records the exact
executables, generated binding, source hashes and process results.

This proof uses the CI correctness profile and a private clone of warm artifacts;
it is not a shipping/codegen/performance measurement. The original repository and
warm cache are retained, while every disposable copied build/source input is
removed. No GPU, provider, native UI, audio device or physical-device gate is claimed.

[Exact source and validation receipt](evidence/native-session-checkpoints.json) and
[independent review](evidence/native-session-checkpoints-review.json) are retained.

## Remaining complete-save work

Retained 2D/3D solver state, contacts/sleep/warm-start/trigger history remain refused;
no scratch Rapier snapshot patch was adopted. Audio/device cursors, complete
`GameSession`/host composition, public player/export save/load orchestration and
platform durable file publication remain separate implementation work. Static/Wasm
persistent-state ABI and any separately selected exact-input replay contract are
also absent. Diagnostics/log history, arbitrary VM globals/closures and external
render/device/provider state are intentionally outside this ordinary-load contract.
