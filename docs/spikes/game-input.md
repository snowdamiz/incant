# Fixed-tick game input and replay

The shared `incant_input` processor accepts bounded, normalized keyboard, mouse,
gamepad and touch events. `PlaySession::tick_with_input` validates an entire packet
before advancing physics or scripts. `api.input()` gives a fresh isolated snapshot
of that tick; gameplay still changes entities through commands. Scripts receive
no window, hardware, network or filesystem handles. The headless host uses this
same processor for versioned input clips.

## Event semantics

Physical key codes describe positions rather than text or IME composition. Held
buttons persist; pressed/released edges last one tick. Repeated key-down events
do not repeat the press edge. A down/up pair within one tick reports both edges.
Mouse position and motion use logical pixels. Absolute position changes accumulate
motion; relative events add motion directly. Adapters must not report both for the
same physical movement. Wheel lines and pixels are separate; no guessed conversion
is applied. Sticks use [-1,1], with positive Y down; analog buttons use [0,1], with
0.5 as the raw digital press threshold. Named actions add configurable thresholds
and dead zones without changing raw values; see [action mapping](input-actions.md).

At most 16 controllers and 16 concurrent touches are accepted. Controller connect/
disconnect events continue while unfocused. Focus loss releases held buttons,
neutralizes analog input, discards pointer/wheel motion, cancels touches and clears
new press/gesture actions. Unfocused action events are ignored after numeric
validation. Focus regain does not fabricate held keys; adapters must supply fresh
state. Disconnect removes the controller and reports its ID for that tick.

Tap requires at most 0.3 seconds and 12 pixels of maximum travel. Long press fires
once after 0.5 seconds within that travel radius. Swipe requires at least 50 pixels
within 0.5 seconds. Durations are fixed-tick quantized with a one-tick minimum.
Exactly two active contacts can emit pinch scale and wrapped rotation deltas;
coincident points cannot define a pinch. Contacts participating in multitouch
cannot subsequently emit an accidental single-touch gesture. Reusing a touch ID
within a tick does not join unrelated contact lifetimes.

Invalid input commits nothing, including time, edges and gesture history. Limits
are 1,024 ordered events per tick, finite coordinates within one million logical
pixels, normalized analog ranges, and a tick interval between 1/240 and 1 second.
Unsupported codes, fields, device lifetimes and values fail explicitly.

## Recording and saved-game continuation

`InputRecording.schema.json` describes `incant-input`, version 1. A clip includes
its fixed `tick_rate`, absolute `start_tick`, duration `ticks`, and sparse `frames`
with strictly increasing one-based offsets. Each frame contains an ordered event
packet; missing frames mean no new events. All device state starts neutral and
focused at the clip start. A nonzero start tick does not import earlier history.

`play --input-replay FILE` validates the entire clip before creating playback
outputs. The clip must cover the requested game-clock interval and match the
project tick rate. It is bounded to 8 MiB, 10,000 ticks, 100,000 total events and
1,024 events per tick. Clocks cannot exceed JavaScript's exact integer range.
Unknown versions, malformed/future-invalid events and out-of-range replay fail.
Live packets cannot be mixed into an installed replay.

Physical device state is deliberately absent from logical game saves. Ordinary
save loading starts neutral. Supplying the same clip on restore seeks only input
history to the saved tick; it reconstructs held controls and gesture age without
rerunning gameplay or repeating prior logs. This does not change the logical save
format's documented limits on physics contacts, sleep and VM globals.

## Verification and remaining scope

Eighteen new behavior tests cover device edges, focus, disconnects, cancellation,
limits, atomic rejection, touch-ID reuse, gesture timing at 30/60/120/240 Hz,
whole-clip validation, seeking and separate-process replay. Script tests verify
input-copy isolation, command-driven motion and exact save/replay continuation.
The strict TypeScript public-CLI probe creates its character course through
`incant_cmd`, then drives movement, jumping and wall contact with recorded keyboard,
mouse, controller and touch events. Saving at tick 75 during a jump and resuming
105 ticks gives exactly the same final runtime state, behavior state, input and
log suffix as uninterrupted play. A repeated 180-tick run is also exact. Project,
journal and recording hashes remain unchanged; no renderer is initialized.

That strict consumer initially exposed a SDK generator bug: Schemars' integer-key
map pattern became an empty object type. The generator now emits a numeric index
signature with possibly absent values and fails explicitly for unhandled patterns.
Controller access now type-checks with the required missing-device guard.

The platform smoke executable also exercises keyboard/controller state, pinch
scale/rotation, focus loss and replay equality. Its input is normalized synthetic
data: it verifies target execution, not live device integration. Hosted source and
Windows/Linux desktop workflows run the strict TypeScript character probe.

The full workspace passes 190 Rust tests, 315 UI tests, five tool tests and
Clippy. Generated schemas, SDK/bridge/conventions and strict TypeScript checks
pass. The native release build and package pass without launching the app.
Existing public save and character probes pass with the new input host. Hosted
checks all passed at `0021738`; PR #27 merged as `72fb2bd` with an identical
reviewed tree. The main native app was rebuilt without opening it. No local GPU
capture tests were rerun.

OS/browser event adapters, actual hardware polling, IME/text input, pointer lock,
hardware-specific gamepad calibration, rumble, native play
controls and mobile lifecycle integration remain open. This increment is the
shared runtime and replay path, not completion of Phase 1 input. No visual changes
or new local captures were made during the director's computer-use pause.

Machine-readable evidence: [game-input-2026-10-09.json](evidence/game-input-2026-10-09.json).
