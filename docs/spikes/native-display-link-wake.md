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

Next qualification is a fresh scene-bound shipping-profile player, exact full
Core gameplay and PCM parity, then a quiet same-player ordinary/display-link
comparison with actual presentation callbacks. Requested 60 Hz is not proof of
delivered cadence. Zero/two-tick bursts from wall/display clock drift remain
possible and must be reported rather than rounded away. No scheduling policy
is adopted and no FPS, script-budget or phase gate passes from this diagnostic.
