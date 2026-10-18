# Native view display-link wake diagnostic

The default-off `native-display-link` player feature tests a different wake source
for the existing fixed-step scheduler. A trusted process selects it with
`--native-wake-source display-link60 --window --stop-after N`, with N in
1..36000 and a cooked 60 Hz scene. Unsupported platforms/API versions, conflicting
presentation experiments and unbounded runs refuse explicitly. Ordinary exports
and ordinary arguments retain the existing timer route.

The available Mac's view-bound CADisplayLink requests 60 Hz and posts a Tao user
event. The main-thread callback retains only its newest observation and at most
one outstanding event. It does no simulation or rendering. Actual callback
arrival and Apple's future target timestamp remain separate; the target never
advances simulation time. The existing rational wall clock, fixed simulation
delta, four-tick catch-up bound, dropped-tick accounting and presentation policy
are unchanged. A callback only permits that existing scheduler to run.

Focus, suspend, resume, resize and restart boundaries discard stale callback
authority without releasing an outstanding event token prematurely. OS input
and health timers cannot authorize gameplay. The active callback watchdog fails
after 500 ms without a valid callback; inactive retries retain their existing
schedule. The link invalidates before releasing the callback target and the
Tao-owned view, including error shutdown.

Independent source review identified a queued background callback that could
authorize a tick after same-batch focus gain. The corrected eligibility boundary
and two regression cases address it. Local Mac qualification passes 158 library
controls (one unrelated ignored control), strict Clippy with the combined native
presentation features, formatting and locked metadata. The tests cover bounded
callback delivery, stale events across focus boundaries, missing callbacks and
early CLI/cooked-scene refusal. They do not run an actual display link or qualify
native window behavior, visual quality or performance.

The retained local CPU record is
`~/.codex/incant-resume-20261017/native-displaylink-cpu-001/`. Its actual wrapper
exited zero; all 142 observed wrapper/child identities were absent at closeout.
The attempt consumed 178.748 seconds, continuing the existing target account at
1776.328 seconds. Allocation peaked at 12,053,086,208 bytes under an explicit
16,328,871,936-byte ceiling. Earlier target history and copied binaries remain
preserved. These sampled resource observations are not a sandbox.

The integrated main393 source at `559ada1079db5dfee13b59ae4f69e7b0b2ffe574`
also passes all 158 controls and strict lint. Its separate CPU002 wrapper exits
zero with all 21 observed identities and three groups independently absent.
The copied test producer hashes to
`9d55aee43fe787e8c59b87cc509466e57a57bc9579d08541e985bb2745fbf915`.
This follow-up takes 28.443 seconds, continuing the same account at 1804.771
seconds; target allocation peaks at 12,058,796,032 bytes under the unchanged cap.
It still contains no native display-link execution.

## Shipping and native result: negative for adoption

A fresh producer at `35f67313eba66c1de2089c6db1b56c4f657e4852` passes the
ordinary export dependency and symbol checks, with shipping fat LTO, one codegen
unit and abort-on-panic. The source-bound diagnostic player hashes to
`7bd3e561b9c777f4fecb0dc8a389809e7685d58f4d278b709d0ba80a937bc0c3`.
Its private build tree and authoring inputs are removed before startup. The full
2,156-step CPU replay then matches the entire known gameplay report and all
13,798,400 PCM bytes. Build and replay take 521.420 and 5.972 seconds; the build
uses a separate fresh target and peaks at 4,031,863,348 scratch bytes. The earlier
CPU qualification account remains separate, unchanged and cumulative.

The actual host is a Mac17,9 with Apple M5 Pro, macOS 27.0.1 (26A434).
Both native arms use a 1280×960 surface and 640×480 logical viewport at scale 2.
The ordinary native arm completes. The first display-link arm loses native focus
at step 2,155, pauses, then reaches its 120-second limit. Its failed receipt and
partial state log remain retained; it produces no usable final timing capture.
A separately reviewed second selected arm, with identical player/package and
settings, completes without focus loss. Both completed arms retain positive
presentation timestamps for every gameplay frame and all 2,155 adjacent intervals.
The separate initial refresh has a zero timestamp and is never bridged into the
gameplay interval series. All wrappers and observed child groups are absent after
their actual terminal results.

| Observation | Ordinary timer | View display link |
| --- | ---: | ---: |
| Presented interval p95 | 16.666750 ms | 25.000042 ms |
| Intervals below 12 ms | 11 | 330 |
| Intervals above 21 ms | 9 | 276 |
| Two-tick scheduler batches | 0 | 269 |
| Dropped wall ticks | 0 | 0 |
| Drawable acquisition p95 | 0.029416 ms | 0.029959 ms |
| First-to-last gameplay presentation span | 35,899.994 ms | 35,916.674 ms |

This is negative for adopting the wake policy. The selected callback permits the
unchanged wall scheduler, which still produces zero- and two-tick batches; asking
for 60 Hz does not establish even delivery. The selected run consumes 2,158
callbacks with no coalescing. Its reported Apple duration remains 8.333 ms while
its aggregate arrival span is 35.952 seconds; those values describe different
observations and are not substituted for the simulation clock. No normal export
selects this diagnostic and the ordinary timer stays the default.

The selected event loop waits on a 500 ms health deadline, so the existing timer
wake-to-simulation-deadline equality check does not apply. The retained decoder
therefore reports incomplete timer attribution. It is not reclassified as valid
callback timing. Full host stages, acquisition boundaries, callback counter
conservation, complete gameplay and finalization remain required; individual
callback-to-step timing is not recorded. The first interrupted attempt remains
failed rather than becoming a discarded warm-up.

This single nonrandomized pair is a scoped observation, not a causal benchmark,
continuous display/FPS test or visual acceptance. Rust `Instant` and Metal
`presentedTime` have different epochs; their absolute values are never subtracted
for queue depth or latency. Offline PCM equality is not a live audio recording.
No frame-pacing policy, script-budget, shader-stall or phase gate is approved.
[Portable records, compressed raw captures and exact hashes](evidence/native-display-link-20261017/manifest.json)
include the independent saved-data review. Gzip captures use a deterministic
header and decompress to the recorded original hashes.
