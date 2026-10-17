# Fixed Metal minimum-duration diagnostic

This is an unqualified, default-off experiment following the uninterrupted Core
trace in `native-wake-acquisition-live.md`. That trace's frequent three-quantum
interval followed by one short interval motivates a comparison; it does not
establish a scheduler or presentation bug. This change has no native compilation
or execution result yet.

Compile the trusted player with `native-minimum-duration`, then select
`--metal-present-mode minimum-duration60` on a bounded native-window replay.
Ordinary exports do not enable this feature. Packages and guest programs cannot
select the policy, and there is no arbitrary duration input.

Selection refuses without the macOS feature, with malformed or duplicate policy,
without exactly one window and `--stop-after` in 1..36000, or with offline capture,
checkpoint, material-recording/prewarm or lifecycle-probe routes. The top-level
parser runs before profiling, capture, material or package setup. A selected
non-60Hz scene or unsupported OS refuses before diagnostic outputs, a window,
audio or GPU setup. Actual surface admission requires Metal, FIFO and the existing
maximum frame latency of 2. Transaction presentation refuses before drawable
acquisition and simulation. There is no silent ordinary-mode fallback.

The HAL owns a fixed cold enum, sealed by first configure, and snapshots it into
each acquired texture. Reconfiguration cannot change selection; a recreated
surface needs its own explicit selection. The selected path calls
`presentDrawable_afterMinimumDuration` with 1/60 second before the existing
command-buffer commit. The existing observer's exact-drawable callback and native
object lifetimes remain unchanged. The API requires macOS 10.15.4 or newer and
specifies a minimum residence of the previously displayed drawable; it is not an
exact deadline. See [Apple's command-buffer API](https://developer.apple.com/documentation/metal/mtlcommandbuffer/present(_:afterminimumduration:)).

Clock, Tao, 60Hz simulation, input, audio, focus eligibility, acquisition retry
limits, FIFO, maximum latency, rendering and shader math are unchanged. Only a
selected final report adds its explicit diagnostic mode descriptor; ordinary
report fields are unchanged. No per-frame observation schema is added.

Prepared CPU controls cover exact ordinary argv, feature refusal, supported
selection, malformed/duplicate/bound/conflict refusal, actual parser ordering,
scene rate, actual cooked-scene early refusal without creating outputs, native
backend/configuration admission, cold sealing, immutable acquired policy,
transaction/non-FIFO refusal and recreated-surface selection. The feature-enabled
player includes the actual pure HAL policy file in its tests; those are not a
second model of its state machine. These Rust controls remain unexecuted pending
a bounded CPU lease. Formatting and source checks are separate from native
qualification.

After CPU qualification, a separate proposal may build one same-source shipping
player for ordinary and selected 2,156-step Core replays. Each run needs a quiet,
exclusive window lease and exact same-package gameplay/CPU reference. Retain
focus boundaries, callback zeros/missing/invalid states, dropped wall ticks,
acquisition/wake/body costs, total callback span and backlog. Reducing short
intervals by forbidding them is insufficient evidence of improvement; extra
residence can increase queueing or input latency. No adoption, continuous display,
FPS, appearance, performance or phase-gate acceptance follows from this experiment.
