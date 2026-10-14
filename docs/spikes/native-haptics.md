# Native TypeScript haptics

This slice adds typed ephemeral haptic output to the native TypeScript tick and
an optional desktop Gilrs consumer. It is not physical-device delivery or
performance evidence. The API is not available to static or development Wasm
modules yet; attempting to enable their capability returns `HapticsUnsupported`.
Headless, mobile and browser hosts advertise no capability unless a real trusted
adapter implements it. No mobile actuator implementation is included here.

`NativeCommands.hapticCapabilities()` copies the current host's targets.
`playHaptic(target, effect)` and `stopHaptic(target)` append at most 32 validated
requests per update, also counting toward the existing shared structural-command
limit. Gamepads accept two-channel rumble (finite amplitudes in [0, 1], duration
1–5000 ms). Device impact is a separate target capability, not silently mapped to
rumble. Target IDs include a nonzero generation; unsupported, stale and unknown
requests fail explicitly. A caught invalid active call still rejects the script
transaction, including numeric prefixes. Expired functions cannot act in a later
update.

No physical operation occurs during script execution. Script output is committed
only after numeric and structural acceptance. `NativeSession::take_haptics` is a
trusted host drain; composed game families publish the drain only after audio and
mandatory host acceptance. An undrained nonempty output refuses another tick
before input or simulation advancement. Suspension clears output and capability
snapshot. Capabilities and requests are never encoded in checkpoints; pending
output refuses capture and receiver replacement. Fresh or restored VMs require a
new host snapshot. Hosts must stop their devices and invalidate targets when
replacing/restoring a session; the desktop replacement path does this explicitly.
Saved guest values are not host authority: each requested target is revalidated.

The optional desktop host enables Gilrs force feedback while keeping the original
input-only initialization as fallback. Per-device effect creation failure leaves
ordinary input usable and advertises no haptics. There are at most 16 retained
logical gamepad slots, one creation attempt per supported slot and two reusable
motor effects per slot. Reusing handles avoids Gilrs' monotonically increasing
backend effect IDs. Backend handles, device names and mapping strings stay private.
Disconnect, suspend, error, restart and drop stop existing effects. Reconnect and
session replacement invalidate old generations; exhaustion refuses rather than
wrapping. A batch is fully validated and then coalesced to its last desired state
per device before driver calls. Failed driver acceptance triggers best-effort stop
of all effects and terminates this window operation.

Gilrs 0.11.2 creates a process-lifetime force-feedback worker; drop does not join
or stop that worker. The window process owns one backend and keeps it through
session replacements. Effect stop/drop cleanup is distinct from worker shutdown.

Gilrs 0.11.2 accepts requests asynchronously and quantizes duration to 50 ms ticks
(rounding upward). Its driver/server channel is not an acknowledged bounded
hardware-delivery queue. This slice bounds engine requests, slots, creation
attempts and calls per accepted tick, not backend allocations or physical timing.
A successful adapter call means desired output was submitted, not that a motor
moved. No device, mobile, GPU or reference-phone gate is claimed.

The core types add no dependencies; Gilrs stays behind `native-window`. Runtime
commands use fixed typed fields and a fixed output array, with no project document,
JSON, authoring command bus or operating-system handle on the hot path.
