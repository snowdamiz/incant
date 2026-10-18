# Independent simulation deadlines: private scheduling diagnostic

This default-off source experiment tests one scheduling hypothesis. It is not an
adopted presentation policy, interpolation implementation, performance result or
phase gate. The ordinary route and `--present-batch latest` remain separate and
unchanged in selection and scheduling. Source starts at main `557c196c`; it does
not include the concurrent pose-interpolation experiment.

The retained three-arm comparison's latest arm completed its 2,156-step route
but failed its original host-timing analyzer. Saved-data inspection found missing
wake metadata on its 239 committed-only draws and a startup drawable reporting
zero. Neither finding turns that receipt into a pass. Across all positive adjacent
callbacks, including committed-only draws and role transitions, 186 of its 193
gaps over 20 ms followed two-tick batches. The source clock was only visited on
display callbacks: an early callback can admit zero steps, then the next callback
admits two. This is a concrete hypothesis, not proof that a different wake policy
will improve actual display intervals. The immutable analysis is external at
`native-batched-presentation-001/independent-analysis-002/review.json`, SHA-256
`a9668788be94ba307363bb28994382f040272fda6aa00c5e1f617ac128b7fcac`.

## Selection and ownership

A private native player must explicitly enable
`native-independent-presentation` and select both `--present-batch independent`
and `--native-wake-source display-link60`. It requires the existing supported Mac,
60 Hz cooked scene, one native window and a finite `--stop-after 1..36000`.
Unsupported platforms/features and incompatible offline/checkpoint/pacing modes
refuse before window, device or GPU creation. Public export selection is unchanged.
No script, project or gameplay input gains control over clocks or OS state.

The existing rational clock remains the only simulation-time authority. While
active, `WaitUntil` chooses the earlier of its next deadline (subject to an
existing surface retry) and the display-link health deadline. A display callback
can still wake the loop, but its future target time is only recorded metadata.
The clock admits only elapsed ticks; no epsilon, future advance or variable dt
is added. Existing max-four catch-up and explicit discarded-debt accounting stay
in force. Focus, suspension, minimization and supported-size fences stay active.
Resume first refreshes through a real reservation, then resets the rational clock
and waits for its next deadline; resume does not force an immediate extra tick.

Each tick uses the ordinary game session, input/replay packet, input consumption,
UI receipts, structural/resource validation, audio and haptic paths. Only GPU
presentation is deferred through the existing mandatory `AdmitOnly` consumer.
Its target slot is empty: an unexpected target acquire fails. No drawable is
retained across event-loop callbacks. Intermediate resource replacements stay
owned by the accepted consumer. Existing refusal of temporal/motion/MetalFX and
dynamic-resolution policies remains unchanged.

On an actual consumed display callback, the host reserves once and refreshes the
latest committed world without advancing simulation, input, audio or haptics.
Stop and restart additionally require a final old-generation draw before owner
retirement. These are explicitly labeled boundary flushes, without an invented
display callback. A deferred boundary flush fails the diagnostic and retains the
accepted simulation receipt; it cannot silently destroy an undisplayed accepted
state and report success. Initial, background and UI-resize refreshes retain their
existing separate roles. UI resize and a delayed resize notification are fenced
by the currently observed native extent and an actual reservation/relayout before
subsequent UI ticks.

An important selected-mode difference is explicit: unknown surface occlusion or
loss is discovered at the next draw acquisition, not before every independent
simulation tick. Known focus/minimize/suspend/size failures still block ticks.
After a failed display acquisition, existing pause/input/surface retry behavior
applies. This experiment must not be presented as equivalent surface admission to
the ordinary path.

## Busy-loop and observation limits

At most four ticks are admitted per due calculation. A busy loop rechecks the
actual current time on the next callback; it does not simulate callback target
time. A past rational deadline may cause an immediate subsequent event-loop wake,
as in ordinary bounded catch-up; there is no `Poll` loop or per-tick sleep/wait.
The extra intentional wake stream is the 60 Hz simulation timer alongside the
already coalesced display callbacks. OS input callbacks and existing health work
remain; this is a bounded-per-turn policy, not a guarantee of bounded total CPU
load under sustained overload. The finite host-step bound and external child
supervisor are still required.

Sidecar v4 is deliberately incompatible with old ordinary/batch analyzers:

- `simulation` / `admitted` rows have schedule identity and completed/finished
  CPU spans, but no reservation, acquisition or drawable. Haptic failure retains
  an unfinished admitted row; a failed tick retains an open row.
- `committed` rows bind the exact current accepted session and host step. Pulse
  draws record the consumed native callback sequence, Rust-Instant arrival and
  native target metadata, plus the current event-loop wake. Stop/restart record
  their reason without a fabricated callback. A shared event-loop wake across
  admissions and a draw is expected, not a reused display callback.
- Simulation rows do not split display segments. Real focus, resize, retry,
  recreation, restart and display-role boundaries remain. All zero, missing,
  invalid, abandoned, equal and positive drawable outcomes must be retained.
- Attempt capacity is shared by simulation and display; 2,156 ticks therefore
  need more than 2,156 attempt slots. Use 8,192 for a future bounded Core probe.
  The existing 36,000 maximum, compile-time 512-byte attempt bound, two bounded
  segments per attempt, one-second final callback drain and 64 MiB sidecar cap
  remain. Snapshot and JSON allocation are outside the preallocated-row bound.
  Exhaustion is explicitly partial, never complete coverage.

No Rust-Instant timestamp is subtracted from Metal time. The experiment does not
convert a native target time, callback count or display interval into GPU time.

## Qualification proposed, not executed

Source controls cover explicit capability selection/refusals; unchanged legacy
selection; an alternating callback phase that yields 0/2 under callback-only
clock visits but 1/1 under independent exact deadlines with identical elapsed tick
counts; busy max-four/debt behavior; focus/resume reset; earliest deadline and
retry/health behavior; exact consumed callback ownership; simulation/drawable
separation; segment continuity and real boundaries; absent/fabricated callback
metadata; boundary flush labeling; and bounded overflow/partial observations.

The first proposal is CPU-only: native player library tests and strict all-target
Clippy with the new feature plus profile/native-compilation, then the ordinary
native-window/profile/native-compilation selection without the new feature.
Use pinned Rust 1.99, arm64 CI profile, locked/offline, two jobs/Rayon threads,
900 seconds per child, 2,400 seconds total, 4 GiB target growth, 6 GiB free floor
and 32 MiB per output stream. Copy each executed test binary immediately. Root
must assign the target/slot and review the frozen managed packet before Cargo.
Only source formatting and diff checks have run for this source milestone.

No native window, GPU execution, shipping export, replay parity, pixel acceptance
or matched display improvement has been established. A later actual run must
retain every role/boundary and every failed/zero/missing sample, compare the same
Core route and source/package identities, and separately report timer delay,
per-tick admission cost, display acquisition/body cost and delivered intervals.
