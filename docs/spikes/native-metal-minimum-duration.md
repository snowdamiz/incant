# Fixed Metal minimum-duration diagnostic

This is a default-off experiment following the uninterrupted Core
trace in `native-wake-acquisition-live.md`. That trace's frequent three-quantum
interval followed by one short interval motivates a comparison; it does not
establish a scheduler or presentation bug. Native producer
`c0ba6ba54ae31aae050cd685f27de7655bf76d16` now passes scoped CPU compilation,
behavior tests and strict lint. A same-player native pair now gives a negative
adoption result: minimum-duration selection removed short intervals but increased
drawable acquisition wait and gameplay presentation span. The selected controller
also retains its original strict timing failure at the terminal batch boundary.
The ordinary default is unchanged.

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

The fresh shipping producer is documentation head
`380a03a4da2e3882dce8e199e85fcce3d13e4006`; all 4,749 executable/tool inputs match
the qualified native source above. One fresh private target produced a fast-physics,
source-free player with the exact project/window/profile/presentation/compilation/
minimum-duration feature selection. No importer was rebuilt: 23 hash-matched
cooked asset inputs were restored from retained metadata and the historical
`a621ef02` package. The current trusted compiler freshly emitted the script and
player. Its observed metadata differs from that historical producer only in
`build_id`; bytecode, hardening, binding and map hashes were compared after the
build. Source equivalence was not assumed.

The 549.151-second build passed actual graph, target-unit, shipping codegen,
source-compiler symbol exclusion and package checks. Its private target and copied
input storage were deleted before the standalone startup. Startup completed one
gameplay tick and 800 PCM sample frames, with `gpu_submitted:false`. A separately
granted 7.342-second CPU replay then completed all 2,156 ticks, matched the entire
retained logical report (eight checks and 17 observed entities), and matched all
13,798,400 PCM bytes. Peak logical scratch was 4,028,761,969 bytes, package
412,415,083 bytes, metadata 39,708,169 bytes and aggregate logs 73,478,739 bytes.
All 56 managed build calls and the full CPU child returned zero; wrapper and owned
process groups were independently absent. Earlier qualification costs and the
unexecuted first build packet's source-review refusal remain retained.

Two separately granted quiet native arms used the same executable
`02e9c05b0d83554e473b8d83deff7d09c597af7d66b4747ad1a0c34686338d83`, bundle,
cooked package and 2,156-step physical recording. The selection flag and fresh
output paths were the only argument differences. Both native children returned
zero, completed every requested step, matched the CPU gameplay observations,
reported all 2,156 gameplay callbacks positive and had no focus loss or audio
diagnostic notifications. Native audio counters do not establish captured or
audible PCM. Neither arm used CUA, input, resize, refocus or capture. Host metadata
queried after cleanup identifies Mac17,9, Apple M5 Pro, macOS 27.0.1 (26A434);
both reports identify the same Metal adapter and 1280×960 physical window.

The ordinary controller completed. The selected controller returned failure:
`partial_or_invalid_scheduler_batches`. Its final due-four batch executed ordinals
`[0,1,2]`, corresponding to host steps 2154–2156; stop-after-2156 left ordinal three
unconsumed. The unchanged v1 projection requires complete ordinals, and the strict
v2 analyzer conservatively marks those same three wake rows inconsistent. Every
per-row acquisition/body timestamp is present, but full scheduler/wake attribution
is not accepted. The failed result and original analyzers remain unchanged.

| Saved gameplay measurement | Ordinary | Minimum duration |
| --- | ---: | ---: |
| First-to-last positive callback span | 35,899.937542 ms | 36,058.297458 ms |
| Mean adjacent callback interval | 16.658904 ms | 16.732389 ms |
| Empirical one / two / three 120 Hz quanta | 29 / 2099 / 27 | 0 / 2138 / 17 |
| Total reservation p95 | 0.171500 ms | 13.543667 ms |
| Isolated drawable acquisition-call p95 | 0.037625 ms | 13.535875 ms |
| Total reservation sum, 2,156 rows | 284.032294 ms | 25,800.014636 ms |
| Drawable acquisition-call sum, 2,156 rows | 69.550122 ms | 25,768.737054 ms |
| Active committed-body p95 | 4.128625 ms | 4.295750 ms |
| Discarded wall-clock ticks | 0 | 4 |
| Final relative presentation-minus-host span proxy | 3.521208 ms | 23.656500 ms |

Total reservation is `reserved_ns - began_ns`. Isolated acquisition is the sum of
`ended_ns - began_ns` over the non-null `host_timing.acquisitions` slots. Every
gameplay row has exactly one successful call and no retry. The strict v2
decomposition independently agrees with those raw calculations. These are
observer-inclusive wall spans around `get_current_texture` and the whole
reservation; neither is CPU time or GPU completion time. Percentiles are ranked
separately and cannot be subtracted to obtain a remainder percentile. The
120 Hz labels are empirical interval bins, not a queried display mode.

The presentation span increased 158.359917 ms while short intervals disappeared.
Drawable backpressure and catch-up batching became substantial: the ordinary arm
had 2,156 single-update batches; the selected arm had 1,043 batches, including
246 due-four batches. This is sufficient to reject adoption from this pair. It
does not establish a general timer/runloop defect. Whole process durations also
include unmatched startup and fixed run order, so their decrease is not evidence
of gameplay improvement.

Metal presentation time and Rust `Instant` have separate epochs. The proxy uses
each clock's independently anchored first matched gameplay row, and measures only
relative span divergence. It is not queue depth or absolute input latency. The
pair is not statistical, continuous-display, FPS, appearance, shader-stall, 2 ms
script-budget or phase-gate acceptance. No retry or policy adoption followed it.

The [87-record saved-data packet](evidence/native-metal-minimum-duration/live-pair-001/manifest.json)
retains original plans, grants, reports, callback rows, process receipts, build log,
source/tool identities, CPU reference and independent review. The repetitive full
Cargo stream, package, symbols and PCM remain outside Git with exact hashes;
selected actual target Cargo rows are included. Replay creates no native process:

```sh
python3 docs/spikes/evidence/native-metal-minimum-duration/live-pair-001/verify.py
python3 docs/spikes/evidence/native-metal-minimum-duration/live-pair-001/verify.py --originals --artifacts --git-source 380a03a4da2e3882dce8e199e85fcce3d13e4006 --source-root .
python3 docs/spikes/evidence/native-metal-minimum-duration/live-pair-001/test_verify.py
```
