# Native editor Play transport and lifecycle

The editor now has a separate native Play worker over `incant_play::PlaySession`.
Start clones the authoring document once, cooks/compiles on a blocking worker, then
transfers owned runtime inputs to the render thread. A revision check refuses
startup after concurrent authoring changes. No project, command bus, authoring
validation or JSON document serialization enters the live tick path. The existing
static authoring viewport resumes after Stop.

The worker owns native VM/world/audio/GPU state together. Pause refreshes the
committed world without a tick; Step accepts exactly one tick while paused; Stop
drops the entire composition. Inspection returns one requested native entity and
its tick, read-only. Authoring continues through the existing command bus and is
not overwritten by gameplay. Saved projects and an explicit spatial camera are
required by shared startup.

Generation-tagged requests prevent late cooking, old inspection or buffered input
from crossing sessions. Preparation has one owner, the command queue holds 16
requests, input packets hold at most 128 events and the native pending input limit
is 1024. Focus loss/minimization/unavailable surfaces suspend simulation and clear
held input; resume resets elapsed time. Rational ticking retains at most four
catch-up ticks and counts discarded overload. Lost/invalid surfaces and device
failure terminate with an error rather than retry an invalid surface indefinitely.

The bridge polls only small Play metadata, at most 4 Hz and with one read in flight.
It preserves Play state across authoring responses, rejects stale successful and
failed reads, and pins input to the generation in which the UI collected it.
Control intent cannot be redirected to a new session during metadata recovery.
No Play operation creates a document transaction or requires an engine account.

## Verification

21 scoped Rust tests pass with all-target Clippy, including paused-state retention,
accepted-tick failures/unwinds, retained-host destruction, bounded queue pressure,
stale failure, input validation and the rational clock. The existing ignored
render-scene GPU case is not counted as a pass. Strict TypeScript, 545 UI tests,
generated bridge/conventions and native custom-protocol builds pass. Independent
lifecycle and bridge review found and corrected stale failure/generation races,
a startup-gate release race, accumulated suspended time and surface-loss handling.

`tools/probes/native-editor-play.py` authors its fixture through public command-bus
RPC, then runs a binary built with the opt-in `play-probe` feature. On this actual
macOS native window, the retained GPU consumer submitted 13 frames by tick 5. Pause
held X=10/tick 5 while presented frames advanced 14→39. Step produced X=12/tick 6.
Minimization held tick 7, unminimization resumed, Stop removed inspection access,
and a fresh generation restarted at tick 0 before producing X=4/tick 2. Project,
script and journal bytes, authoring revision and history were unchanged. The first
probe correctly refused an omitted explicit camera; only its fixture camera option
was corrected before the passing run. Both outcomes are retained.

[Source, log and binary identities](evidence/native-editor-play.json) bind the
measurements. This is numerical native-window correctness evidence, not pixel
review or a phone/performance gate. Device audio suspension takes effect at a
callback boundary; stepped device audio is not sample-exact.

## UI integration and remaining native review

Claude Opus 5.5 at explicitly verified Max implemented the controls, bounded
desktop event routing, shortcuts and read-only runtime Inspector in
[handoff 0044](../../handoffs/0044-native-editor-play/result.md). Its 606-test UI
suite and 73 labelled browser-double flows pass; the combined agent/Play UI now
passes 658 tests, strict types and build. Root reviewed the control/input logic
and integrated the returned commit. Browser captures show no actual game pixels.
The exact compiled native binary is ready for Claude's actual-window visual
acceptance; that review remains required before calling this feature done.
Gamepad, touch, raw pointer motion, complete session persistence and cross-platform
native-window verification are not supplied by this work.
