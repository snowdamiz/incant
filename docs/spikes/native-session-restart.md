# Whole native game-session restart

The native TypeScript command `context.commands.requestRestart()` asks an opted-in
composed host to replace the complete game after the current tick is accepted.
It takes no arguments. Requests share the structural command budget, coalesce
within a tick, and expire with that tick's capability. A later callback, source
group, structural, audio or consumer failure cannot expose the request. A host
that has not opted in refuses explicitly; catching that refusal does not accept
the tick. A checkpoint refuses an unconsumed request rather than omitting it.

`GameSeed` retains the original cooked scene, compiled native bytecode, action
bindings and immutable cooked clip handles. Replacement drops the previous host,
world, VM, timers, persistent script data, input/router and mixer, then calls the
ordinary constructors with that seed. It creates a fresh world identity and UI
owner, reloads canonical cooked render resources, and publishes no mixed
candidate. Preparation failure or unwind leaves the previous session terminal.
This resets initial physics declarations; it does not restore solver state from
a save, and the existing physics checkpoint refusal remains intact.

The ordinary CPU packaged player retains a separate accepted host-step counter,
session generation and cumulative output position. Existing input/observation
recording ticks address that monotonic host timeline. Simulation ticks, VM logs,
UI state and the rational audio clock restart from their initial values. A run
without replacement preserves its report shape. A run with replacement adds a
`session` object with generation, host tick, current simulation tick and bounded
replacement boundaries. The final logs belong to the current VM; prior VM logs
are discarded with it. PCM is the concatenation of accepted segments. Fractional
sample counts are calculated from each fresh game's own rational clock, with a
checked total output limit; they are not inferred from reset simulation ticks.

Physical ownership stays in the host. Held keyboard, pointer, touch, gamepad
buttons and nonzero stick axes remain suppressed until their actual release or
zero, while connected gamepad topology is announced to the new input owner.
There are no synthetic new presses. Recorded UI identities use the current
owner through the existing resolver; delayed native receipts still carry their
original tokens and cannot target the replacement.

The desktop window source owns the same full replacement, keeping only the OS
window, renderer/surface and physical adapters. It retires the old native text
session using the monotonic IME allocator, clears pointer receipts, destroys the
old consumer before issuing a new surface-provider capability, and creates the
new mixer initially master-paused. A reserved, consumed initial frame precedes
resume and simulation. `--stop-after` counts accepted host steps across resets.
CPU lifecycle tests are not native-window, device-audio or rendered acceptance.
Those remain a separate Claude/platform verification task.

Core and Planar accept the explicitly authored `restart` action only while
terminal. The controllers issue the request; they do not reconstruct entities or
partially clear gameplay variables. Existing projects without that action keep
their prior behavior. The numerical fixtures bind KeyR and gamepad Start through
public commands. No visual prompt, layout, asset or shortcut default is changed.
Headless native Play, offscreen GPU runs, static modules and development Wasm have
not gained an implicit replacement owner or a restart ABI in this slice.

Validation: source002 has 343 passing CPU tests across 29 groups,
scoped all-target Clippy and the headless build. It preserves the initial
structural overflow-error regression and corrected rerun. Actual public-command
fixtures pass seven existing gameplay scenarios plus an idle baseline for each
sample, public Undo/Redo/reopen, and exact unsupported-host diagnostics. The
two fresh shipping exports pass 24 actual packaged-player runs after deletion
of their private authoring inputs and debug source maps. Each preserves eight
ordinary native baselines and verifies win-to-win, gamepad, loss-to-win and held
input restart cases. Canonical UI/flipbook/render data, current VM logs and
selected entity state match fresh games exactly; concatenated PCM matches byte
for byte, including Core static cues and streamed music. Checks at simulation
tick 1 in two sessions execute once each under distinct host steps.

The [evidence packet](evidence/native-session-restart.json) retains source and
build identities, raw/gzip logs, exact public command receipts and package data
hashes. Independent source and final execution reviews are accepted; the
[final receipt](evidence/native-session-restart/final-review.json) binds the exact
reviewed packet. Publisher integration is pending. No save-feature, device, performance or appearance gate is claimed.
