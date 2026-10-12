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

## Native window focus follow-up

Claude0044's actual macOS window review covered controls, runtime rows, logical
pointer coordinates and titlebar alignment. It found an unfocused game inheriting
window focus after Resume and keyboard focus disappearing after Dismiss. Its UI
corrections and regression tests are integrated. The native worker now resets
game input to unfocused at start/hold/resume; only the game view's explicit input
stream grants focus. A real InputRuntime regression verifies held controls are
released, stale queued input discarded and keys ignored until that explicit focus.

The current union passes 782 UI tests and39 editor tests (one existing GPU test
ignored), plus scoped all-target Clippy. [Independent reviews and logs](evidence/native-play-focus-correction.json)
bind the corrected source. The reviewed native binary predates these corrections;
combined native acceptance after the pending budget and splitter controls remains
required. The splitter selection defect is assigned separately to Claude0047.

## Attached-script isolation in the native fixture

The numerical fixture now filters world-wide chunk views by the script's actual
instances. Previously its movement affected the unrelated camera as well as the
mover. The opt-in native probe additionally inspects the camera after a single
Step and requires its authored origin to remain unchanged. The existing +2 mover
assertion, pause/redraw, minimize/resume, Stop/restart and authoring byte/history
checks remain intact. Independent source review, feature-enabled compilation and
an actual native-window run pass on the integrated gameplay-focus correction.
The new run observes X=10 at tick5, X=12 at tick6 and a fresh X=4 at tick2; paused
presentation advances14→39 without ticking. No provider was called.
[Exact sources, binary hashes, review and result](evidence/native-play-instance-isolation.json).
This is numerical native evidence; the pending budget/splitter UI and corrected
WebView focus interaction still require combined visual/native acceptance.

## Saved-project prerequisite before Start

The native ready snapshot now exposes the same saved-project prerequisite that
Start enforces. The existing Play controls explain an unsaved project before
issuing a request, without creating a failed session. Older hosts without the
optional field retain their existing behavior; malformed present metadata refuses
Start. The bridge rechecks the current prerequisite after asynchronous status
recovery. Pause and Stop remain available for an existing session.

The combined suite passes 795 UI tests; 85 focused bridge/Play tests pass after
the final type correction. Strict app/test TypeScript, production UI build,
feature-enabled all-target editor Clippy and independent source review pass.
[Exact source and evidence](evidence/native-play-start-availability.json) preserve
the initial test/type failures and corrections. Actual unsaved-window interaction
remains part of the combined native acceptance; this readiness hint cannot
guarantee a successful cook or runtime startup.

## Integrated splitter input correction

Claude0047's final pointer correction is integrated. A primary press cancels the
selection default while keeping the menu-dismiss event and keyboard resize focus.
The active drag belongs to its pointer and ends on release, cancellation, lost
capture or a move without its button. Other pointers and secondary-button presses
cannot resize the pane. The final 15 tests include mutation checks for these
conditions; the combined UI passes 810 tests, strict app/test types and production
build. Claude's real-browser review passes all 34 checks.

[Source review and integration checks](evidence/native-splitter-integration.json)
bind the exact component. The browser proves the selection start is prevented;
the original visible WKWebView symptom and native focus still require handoff0048
against the newly built combined app.

## Committed script logs in Console

Native Play now forwards the existing accepted `NativeLogs` prefix to the existing
Console. The render-thread worker publishes only after a successful game operation;
its cursor copies newly committed rows once, including repeated same-tick messages
and their original levels. Pause, surface redraw and repeated status reads do not
copy the earlier prefix or allocate new row identities. Each row has one native ULID.
No frame reads the project, command bus, source files or provider state.

New hosts add optional `play_read.logs:{count,dropped}`. The bridge requests
`play_logs_read({generation,afterIndex})` only when a tail or drop counter changed.
Each response carries at most 64 rows/16 KiB of UTF-8 message bytes, with each message
at most 1 KiB; JSON escaping and bounded row metadata add wire overhead. The native
mirror is bounded to the existing 1,024-row/256-KiB prefix. Browser retained Play
history stays within 1,024 rows/256 KiB across restarts, reserving one warning row
and 256 bytes for an aggregate truncation warning. Native and retention losses
increment that stable Console warning; exposed counters saturate at the JS-safe
integer maximum. Older hosts lacking log metadata make no new IPC calls.

The bridge validates each entire bounded packet before advancing its cursor or
publishing rows. Project identity and Play generation fence late responses and
errors. Stop/restart resets the live cursor, and a project change clears retained
Play history. Already delivered accepted rows remain historical Console entries
through Stop or terminal failure. Terminal native reads refuse unread live history;
a failed callback cannot publish its staged logs. Failed/malformed log reads use a
fixed safe transport error and retry the same cursor on recovery.

Local validation passes 42 CPU editor tests, including an actual OfflineBackend
PlaySession/QuickJS accepted-and-trapped callback test, plus scoped all-target
Clippy, 185 bridge tests, strict app/test TypeScript, and generated bridge checks.
The existing native GPU test remains ignored. Bridge race tests use synthetic IPC;
there was no native window/GPU launch or rendered Console review in this slice.
[Exact sources, logs and limitations](evidence/native-play-console-logs.json).

### Main116 Console integration

Only the accepted Console commit `34dc2af` is applied to actual main116; its older
parent's separately queued Wasm changes are excluded. STATUS required additive
resolution. All nine editor/bridge implementation, test and generated files match
the independent source review exactly. The integrated 42 CPU editor tests and
185 bridge tests pass, alongside all-target Clippy, strict app/test types,
generated bridge, formatting and conventions. One existing GPU test remains
ignored. The actual main merge changes ancestry only, preserving tested source.

[Integrated commands and identities](evidence/native-play-console-main116.json),
[root review](evidence/native-play-console-root-review.json), and
[integration review](evidence/native-play-console-integration-review.json).
Native Console appearance and live window evidence remain separate from these
CPU/transport checks; no new provider or graphical operation was performed.
