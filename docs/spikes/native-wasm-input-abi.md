# Bounded accepted-input ABI for native Wasm

This extends the existing copied numeric `incant_v1` capability without changing
its imports or `incant_abi() == 1`. The separately versioned `incant_input_v1`
namespace provides read-only input snapshots in both the development Wasmtime
host and build-linked wasm2c host. It adds no WASI, filesystem, network, shell,
project editing, native pointer or runtime compiler capability.

## Acceptance and ownership

`tick_with_input(world, dt, &InputRuntime)` accepts only the engine's validated
input runtime. Callers cannot submit a public arbitrary `InputFrame`. The native
session already owns that runtime, validates each packet atomically, advances
native simulation, then dispatches the module with the accepted snapshot. Action
maps remain immutable and validated at construction/rebind; encoding performs no
JSON serialization, schema checking or action-map authoring validation.

The host encodes once per module tick into a reusable vector capped at 128 KiB.
The guest reads a copy into its own bounded linear memory. This is two bounded
copies, not zero-copy. The first input tick reserves the encoder storage; this is
not an allocation-free claim for the whole input/native/module path. Existing
numeric capture, undo and command limits remain unchanged.

An invalid input packet or offline output-buffer length is retryable before
acceptance: no native tick, module counter, edge consumption or audio advance.
After acceptance, native simulation, module or audio/host errors make the session
terminal and hide its world/input/output state. Module import/trap errors discard
staged numeric writes and commands; the VM is not resumable. Already accepted
native simulation and asynchronous device audio are not rolled back. No full
session save/seek support is implied.

Standalone `tick(world, dt)` still supplies no input capability. A module using
input imports there fails explicitly. The direct `tick_with_input` owner must
schedule the input and world on one fixed timeline; it does not advance either
clock. Startup code has no active input epoch. Imported function names, versions
and signatures outside the explicit allowlist fail.

## Imports and bounds

| Import in `incant_input_v1` | Core Wasm signature | Result |
| --- | --- | --- |
| `snapshot_len` | `(i64 epoch) -> i32` | Exact byte length |
| `read_snapshot` | `(i64 epoch, i32 destination, i32 capacity) -> i32` | Exact bytes written |

Epoch is the current `update` epoch, shared with numeric capabilities. Both calls
consume the existing per-tick host-call budget; each snapshot read also consumes
its length from the shared transfer budget. Capacity must be nonnegative, at
least the snapshot length, and its entire destination range must fit initialized
guest memory. Invalid epoch, capacity, range or quota returns an import failure
without writing a partial destination. Neither call supplies host addresses.
Calls after finish/abort or from module startup fail. A guest may retain its own
copied bytes, but they do not become an authoritative future input frame.

The engine's current limits are 1024 events/tick, 16 simultaneous gamepads,
16 active touches, and 64 named actions with names of at most 64 ASCII bytes.
Snapshot fields check those bounds (gesture bound 1041), finite numbers and total
128 KiB before publication. Existing limits of 65,536 calls and 128 MiB of total
numeric/input transfers apply together. These are logical host quotas, not a
process-memory or wall-time guarantee.

## Wire layout, version 1

All integers and IEEE-754 binary64 values are little-endian; records are packed
with **no implicit alignment or padding**. Reserved bytes are zero. Collections
follow the native sorted key/ID order, except touch changes and gestures which
retain the input runtime's event order. A reader must validate the magic, total
length, counts and tags before using data. An incompatible layout needs a new
namespace/magic rather than reinterpretation of these bytes.

| Header offset | Field |
| --- | --- |
| 0 | 8 bytes ASCII `INPINP01` |
| 8 | total byte count: u32 |
| 12 | focused: u32, 0 or 1 |
| 16 | committed native world tick: u64 |
| 24 | action count: u32 |

The remaining sections occur in this order:

1. Actions: `name_length:u8, kind:u8, flags:u8, reserved:u8, value:[f64;2], name:[u8;name_length]`. Kind is button=0, axis1=1, axis2=2. Flags are active=1, pressed=2, released=4 and can coexist for a sub-tick tap. The name is not NUL terminated.
2. Keyboard: three u128 masks, held/pressed/released. Bit positions are the explicit key tags below.
3. Pointer: three u8 button masks (held/pressed/released), position-present:u8, then position, motion, pixel wheel and line wheel as four `[f64;2]` values. Missing position uses zero coordinates with presence=0.
4. Gamepads: count:u32, then each `id:u32`, three u32 button masks, values-present:u32, 17 f64 analog button values, left stick `[f64;2]`, right stick `[f64;2]`. Missing analog entries are zero and distinguished by the presence mask.
5. Connected IDs: count:u32, IDs:u32. Disconnected IDs use the same format.
6. Active touches: count:u32, then `id:u32, position:[f64;2], delta:[f64;2]`.
7. Touch changes: count:u32, then `id:u32, phase:u8, reserved:[u8;3], position:[f64;2]`. Phases down/move/up/cancel are 0/1/2/3.
8. Gestures: count:u32, then tag:u8 and three reserved bytes, followed by the tagged payload below.

| Gesture tag | Payload after tag/reserved |
| --- | --- |
| 1 tap | id:u32, position:[f64;2] |
| 2 long press | id:u32, position:[f64;2] |
| 3 swipe | id:u32, direction:[f64;2], distance:f64, duration_seconds:f64 |
| 4 pinch | ids:[u32;2], center:[f64;2], scale_delta:f64, rotation_delta:f64 |

Pointer button tags: left=0, right=1, middle=2, back=3, forward=4.
Gamepad button tags, in analog-value order: south=0, east=1, west=2, north=3,
left shoulder=4, right shoulder=5, left trigger=6, right trigger=7, select=8,
start=9, guide=10, left stick=11, right stick=12, dpad up=13, down=14, left=15,
right=16.

Keyboard tags are explicit and independent of Rust discriminants:

- 0–25: KeyA…KeyZ; 26–35: Digit0…Digit9.
- 36–50: ArrowUp, ArrowDown, ArrowLeft, ArrowRight, Space, Enter, Escape, Tab, Backspace, Delete, Insert, Home, End, PageUp, PageDown.
- 51–64: ShiftLeft, ShiftRight, ControlLeft, ControlRight, AltLeft, AltRight, MetaLeft, MetaRight, CapsLock, NumLock, ScrollLock, Pause, PrintScreen, ContextMenu.
- 65–75: Backquote, Minus, Equal, BracketLeft, BracketRight, Backslash, Semicolon, Quote, Comma, Period, Slash.
- 76–85: Numpad0…Numpad9; 86–91: NumpadAdd, NumpadSubtract, NumpadMultiply, NumpadDivide, NumpadDecimal, NumpadEnter.
- 92–103: F1…F12. Remaining bits are reserved zero.

## Observable conformance and executable scope

The shared conformance command keeps the original 13 Wasmtime tests, static
numerical/fuel/trap cases, and native state-stream parity. New tests inspect wire
values for raw controls, action tap edges, gamepads, all gesture variants, maximum
accepted packets and quotas; actual modules in both hosts consume identical full
snapshots. Missing authority, stale epochs, startup imports, wrong versions and
out-of-bounds/short destinations fail without publishing a staged transform.

The trusted Rust game fixture reads `jump` through this ABI. A press moves its
source and spawns a native entity; held state controls velocity. Native Rapier,
module writes, accepted replay edges and retained static/streaming audio share
one fixed tick. Delaying the press from tick 1 to tick 5 changes the final spawned
position from 1.96875 to 1.84375. This positive control would fail if the guest
merely guessed the first tick or the proof only observed native input counters.
The offline rational clock produces exactly 44,117 stereo frames over 64 ticks.

The proof uses the shipping profile and a normal-dependency player, then deletes
temporary Rust/Wasm/generated C, translator and shipping build storage before
fresh execution. Cooked scenes, retained PCM and startup recording JSON remain
runtime exchange files. The source repository and validator build are retained.
QuickJS remains a normal dependency of `incant_script`; source parsing is disabled
and audited, but this is not a QuickJS-free binary. There is no renderer, complete
Wasm SDK, zero-copy, full session save, mobile/device or phone-performance claim.

The desktop workflow runs combined dev/static/composed conformance on one dev
feature graph and retains the standalone shipping numerical/fuel/refusal proof.
It adds an isolated fresh composed shipping proof, with distinct logs/artifacts.
That extra compilation cost is explicit; compiler features never enter either
shipping command. Hosted results and timings remain pending until that exact
revision executes there.

## Local result

Frozen source/log identities and the full proof report are in
[evidence/native-wasm-input-local.json](evidence/native-wasm-input-local.json).
On aarch64 macOS, the final combined run passed **101 tests/doctests**, all-target
Clippy with warnings denied, and the actual Rust development guest. The original
13 Wasmtime cases remain. The Python tool suite passed 49 tests; actionlint,
format/conventions checks, default script/game checks, default-only Wasm-target
dev-host compilation and three normal player dependency guards passed.

Commands (all Cargo subprocesses use `-j2`):

```sh
python3 tools/probes/shipping-wasm2c.py /tmp/incant-wasm-input-conformance-final --conformance-only --rustup /opt/homebrew/bin/rustup
python3 tools/probes/static-game.py /tmp/incant-wasm-input-shipping --rustup /opt/homebrew/bin/rustup
./tools/cargo check -j2 -p incant_script -p incant_game_runtime --no-default-features --locked
./tools/cargo check -j2 -p incant_wasm_dev --no-default-features --target wasm32-unknown-unknown --locked
python3 tools/check_runtime_dependencies.py incant_wasm_player incant_game_player incant_script_player --features incant_wasm_player/fixture
python3 -m unittest discover -s tools/tests
```

The fresh shipping Cargo compile took **1m43s** locally; this is a build diagnostic,
not a runtime performance result. Eight successful and 26 refusal subprocesses
passed after compile-input removal. The player is **3,933,488 bytes**, 36,064 bytes
above the previous static-game fixture (3,897,424 bytes). Its SHA-256 is
`6ff066b65272d7467283bff80eec6101cc8561724c25ff43cd92ec9a326aca75`;
the matched arm64 dSYM UUID is `06c3967e-235a-301d-b853-1cd6d4cc5b52`.
The result JSON SHA-256 is
`3a9b181dadfd9cf617f91c91848e9135eac2a9c038c0d5c82ee14b63f3c8dc46`.
Windows and Linux execution for this new input ABI remains pending the added
hosted checks; the earlier MSVC/static baseline evidence does not establish it.
