# Fixed Metal minimum-duration diagnostic

This is a default-off experiment following the uninterrupted Core
trace in `native-wake-acquisition-live.md`. That trace's frequent three-quantum
interval followed by one short interval motivates a comparison; it does not
establish a scheduler or presentation bug. Native producer
`c0ba6ba54ae31aae050cd685f27de7655bf76d16` now passes scoped CPU compilation,
behavior tests and strict lint. Actual selected presentation remains unrun.

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

The CPU controls cover exact ordinary argv, feature refusal, supported
selection, malformed/duplicate/bound/conflict refusal, actual parser ordering,
scene rate, actual cooked-scene early refusal without creating outputs, native
backend/configuration admission, cold sealing, immutable acquired policy,
transaction/non-FIFO refusal and recreated-surface selection. The feature-enabled
player includes the actual pure HAL policy file in its tests; those are not a
second model of its state machine.

The final source-identical qualification passed 11 feature-enabled, 6 ordinary
native-window and 5 feature-off controls: 22 test executions across 11 unique
definitions. Selected and ordinary player all-target strict Clippy passed, followed
by primary-package strict HAL lint with the exact five features
`diagnostic-compilation`, `diagnostic-minimum-duration`,
`diagnostic-presentations`, `metal` and `portable-atomic`. Rust 1.99 on arm64 macOS
used the CI profile, locked/offline Cargo, two jobs and two Rayon threads. These
tests exercise early refusal and cold policy without creating a window or running
a standalone player, GPU or shipping export.

Two failed attempts remain part of the evidence. CPU001 passed 17 controls before
the base-feature build selected stale local `incant_types` metadata from a prior
worktree. The current source exports coroutine types unconditionally; no feature
or production dependency correction was needed. Saved Cargo artifact selection,
dep-info, fingerprints and source text establish the mismatch. The old metadata
bytes and original failing rustc invocation were not retained, so those particular
observations cannot be replayed from the archive alone. Before CPU002, 451
fingerprint directories covering all 61 current workspace/patched-path packages
were quarantined inside the same target; registry fingerprints, libraries and
executables were retained. Cargo then rebuilt the local variants and passed all
22 controls plus both player lints. Its final direct HAL command refused Cargo's
external-package feature selection before compilation. CPU003 changed only that
command to workspace-qualified feature selection and passed the remaining lint.
No native source or acceptance condition changed across these attempts.

All attempts total 604.303 seconds. The shared target grew from 4,246,511,616 to
6,698,582,016 allocated bytes under an explicitly extended 8,541,478,912-byte
effective ceiling; the original 12 GiB shared ceiling remains recorded. The
395,296,856 copied bytes include 13 protected Wasm executables preserved before
Cargo touched the target and five test artifacts. All originals/copies were
verified, 4,898,887 log bytes retained, and owned process groups independently
absent after terminal cleanup. No target or prior-attempt accounting was reset.

The [portable packet](evidence/native-metal-minimum-duration/manifest.json)
contains 65 saved text records, raw output, exact commands, source/tool hashes,
grants, failures and cleanup. Binaries remain outside Git. Replay without native
execution:

```sh
python3 docs/spikes/evidence/native-metal-minimum-duration/verify.py
python3 docs/spikes/evidence/native-metal-minimum-duration/verify.py --originals --binaries --git-source c0ba6ba54ae31aae050cd685f27de7655bf76d16 --source-root .
python3 docs/spikes/evidence/native-metal-minimum-duration/test_verify.py
```

After CPU qualification, a separate proposal may build one same-source shipping
player for ordinary and selected 2,156-step Core replays. Each run needs a quiet,
exclusive window lease and exact same-package gameplay/CPU reference. Retain
focus boundaries, callback zeros/missing/invalid states, dropped wall ticks,
acquisition/wake/body costs, total callback span and relative span divergence.
Metal presentation time and Rust `Instant` do not share a clock epoch; relative
span divergence is only a backlog-growth proxy, not measured queue depth or input
latency. Reducing short
intervals by forbidding them is insufficient evidence of improvement; extra
residence can increase queueing or input latency. No adoption, continuous display,
FPS, appearance, performance or phase-gate acceptance follows from this experiment.
