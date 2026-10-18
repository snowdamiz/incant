# Rust input views across the native Wasm hosts

This slice adds actual Rust guest test sources for existing input APIs. It adds
no import, ABI version, host permission or production input behavior. Native
qualification is pending; source and preparation controls are not execution
evidence for either Wasmtime or wasm2c.

The Phase 1 SDK deliverables and section 12 decisions are in PLAN.md. The audit compared
`sdk/ts/src/native.d.ts`, `sdk/rust/src`, the generated ABI and both host dispatch
routes. Existing public command/query families already have Rust routes; adding
more wrappers would not close a demonstrated capability gap. The useful small
gap is typed input conformance: `input_parity.rs` executes WAT that hashes the
entire raw input frame, while the previous real Rust gameplay sample reads only
the named `toggle` action. Previous SDK snapshot tests use empty gamepad and
gesture collections.

## Method-level input coverage

| Rust API | Existing evidence/test source | Added actual Rust guest contract |
| --- | --- | --- |
| `Tick::input`, `InputSnapshot::parse` | Raw two-host WAT copy/length/epoch controls; SDK unknown/truncated/unaligned-layout tests | Full accepted snapshot consumed with initialized fixed scratch; ignored short-scratch failure rolls back |
| `actions`, `action` | Real Rust gameplay sample observes one button action | Every named row and lookup agree; missing lookup is absent; button, axis1, axis2 and gesture actions/edges match the native frame |
| `keyboard`, `mouse` | Full raw-frame hash and native TS snapshot tests | Exact held/pressed/released masks, high key tag F12, optional position, motion, both wheel units and focus cancellation |
| `gamepads`, `Gamepad::value` | Raw-frame hash; prior Rust view tests use empty collections | All 17 values, raw slots, presence mask, both sticks and button edges; explicit zero differs from absence |
| `connected`, `disconnected` | Raw-frame hash | Sorted nonempty IDs including `u32::MAX`; disconnect/reconnect discards prior values/edges |
| `touches`, `touch_changes` | Raw-frame hash | Position/delta and ordered Down/Move/Up/Cancel rows; high IDs, focus cancellation without End, ignored unfocused movement and same-ID reuse |
| `gestures` | Raw-frame hash; prior Rust view tests use empty collections | Nonempty Tap, LongPress, Swipe and Pinch, with exact IDs/coordinates/duration/direction/scale/rotation |

The new `rust_input` test projects the independent native `InputFrame` into
typed expected rows. The guest logs decoded SDK fields, never raw ABI bytes or
a checksum of them. Floating values use hexadecimal bit patterns, retaining
signed zero. Assertions additionally require that every gesture kind is reached,
all 17 gamepad values are present, cancellation occurs, and reused IDs are clean.
The existing TypeScript snapshot test independently compares its complete
public projection against that same native frame representation; the new test
does not claim to compile an additional TypeScript guest.

Three adversarial Rust variants stage numeric writes and a log first, then
ignore an input scratch failure, return callback failure, or use an expired
epoch. Both hosts must reject the complete tick and retain only the previously
committed prefix after development reload. Existing WAT missing-input,
out-of-bounds, stale-token and startup/namespace refusals remain unchanged.

## Other SDK families and remaining boundaries

The source audit found existing routes and focused actual-host test sources for
numeric IDs/transforms/velocities; structural spawn/despawn/parent/velocity;
flipbook and skeletal playback; graph/pose/IK/root motion; 2D/3D physics queries
and character movement; navigation path/grid/steering; audio transport/gains;
HUD/world text/disabled state; locale/message/number/date/report; haptic
capabilities/play/stop; timers, coroutines, initial instance properties,
persistence and restart. See the corresponding `crates/incant_wasm_dev/tests`
and `sdk/rust/samples` files. This is a source inventory, not a statement that
every argument combination or release target is qualified.

Larger gaps rank after this bounded conformance addition:

1. Multiple authored TypeScript source/instance callbacks versus one scene-wide
   Wasm module with an immutable initial-owner catalog. Dynamic attachment,
   mixed TS/Wasm and multiple module ownership need an explicit architecture
   and lifecycle contract; they are not missing forwarding methods.
2. Browser Wasm module-host parity needs a browser runtime and export/conformance
   route, beyond the current native Wasmtime/wasm2c pair.
3. TypeScript source/instance JSON persistence and Rust explicit module state
   have deliberately different ownership contracts. Arbitrary VM memory, Rust
   stacks and borrowed views must not become implicit checkpoint state.

The added helper `input_guests` prepares four fixed source-bound guests through
the existing pinned compiler and translation machinery. A focused later native
episode should prepare those guests plus the five existing input WAT controls,
then run only `incant_wasm_dev` tests `rust_input` and `input_parity` with
`static-conformance`, followed by strict feature lint and the ordinary SDK
snapshot tests as appropriate. It must not invoke the full shipping-Wasm suite
merely to qualify this small addition. No device, UI routing, audio-device,
window, rendering, performance or full-SDK parity gate is claimed here.

Source preparation passed 29 Python controls (`test_wasm_input`,
`test_wasm_sdk`, `test_rust_guest`), including two new fixed-input preparation
controls. Rust formatting, generated ABI/conventions checks and `git diff
--check` also pass. These checks did not compile or execute the new Rust guests.
