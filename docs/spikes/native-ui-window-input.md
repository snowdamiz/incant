# Native window UI delivery — first CPU slice

The existing desktop game loop now supplies opaque pointer and committed-text
receipts to its ordinary composed `tick_with_ui`. It uses the existing packaged
font settings and the actual window's logical viewport. `resize_ui` prepares the
same retained UI owner without advancing script, world, input or audio clocks.
This branch still refuses a UI game before window/audio/GPU creation while its
compiled rendering consumer lacks UI support; Claude0056 supplies the separate
real consumer. No new player mode or authored mutation path exists.

A left-button sequence belongs to the eligible clipped UI hit or gameplay. An
owned UI sequence never also emits a gameplay mouse event. A held gameplay button
handing off to UI emits one neutral release and remains suppressed until physical
release. Pointer cancellation on viewport change, departure or replacement never
activates a widget. The new `CancelPointer` operation uses the same world/owner,
layout generation and delivery epoch checks as pointer input; stale well-formed
receipts are counted and discarded. Equality of opaque receipts compares Arc
identity, not only equal widget IDs or generation numbers.

Viewport-only relayout preserves keyboard focus and accepted composition. Resize
acquires a surface reservation before publishing the new UI layout and refreshes
that exact frame in the same callback, even when no simulation tick is due.
Pointer delivery stays fenced until that frame is consumed. A press without a
fresh known position over an interactive layout is suppressed. Temporary surface
acquisition deferral pauses clocks but preserves delivered input, text focus and
composition; it does not fabricate an OS blur. Actual blur/minimize/suspend
neutralizes pending input. The same delivery predicate keeps gamepad releases during this wait, preventing
a stuck accepted hold. These controls close the independent review findings. Native
focus loss/suspension clears pending native receipts and delivers the existing
physical neutral-focus transition at the next accepted tick. Accepted widget and
simulation state still commit together. A refused prepared candidate cannot edit
the accepted UI, and input queues are cleared only after success. Event delivery
is bounded by the existing256 receipts/64KiB packet and16KiB per text operation.

Physical keys do not become characters. Pinned Tao0.37.1 delivers ordinary and
IME text through `ReceivedImeText` on macOS/GTK, overlapping `KeyEvent.text` there;
this adapter uses only the former. Windows ordinary text uses `KeyEvent.text` and
its IME uses `ReceivedImeText`. Control characters are not inserted a second time
beside typed submit/navigation commands. Full preedit/activation and a native-side
session serial are the next focused Tao bridge, not claimed by this first slice.
Native platform execution and rendered acceptance remain separate from these
synthetic tests. The no-UI game loop retains its original physical path.

Validation:72 UI/player tests passed in9 groups, including12 new adapter tests;
all-target Clippy, formatting, conventions and normal runtime dependency checks
passed. Initial Clippy caught only helper placement after a test module; it was
moved unchanged before the final successful run. No GPU/window/provider was
started. Exact source/log identities and limitations are in
[evidence](evidence/native-ui-window-input.json).

The prior authored/cooked/composed UI foundation and map-free public exported
player proof remain immutable at02290953; its final root review is retained in
[the receipt](evidence/native-ui-layout-final-root-review.json). This input slice
is not the full Game UI, desktop IME, checkpoint, flex-widget styling or device
acceptance gate.

Independent read-only source/CPU review accepted the exact five implementation
inputs at88ad9d88 after the presentation, retry and gamepad corrections. The
[review receipt](evidence/native-ui-window-input-review.json) preserves findings,
verified hashes and excluded native IME/appearance gates.

## Current main131 integration

All five accepted implementation/test files remain byte-identical in the
animation-graph/current-main union; only STATUS needed conflict resolution. The
actual main131 ancestry merge changes no tested file. The integrated 72 CPU tests
in 9 groups, native-window all-target Clippy, wasm32 UI compile, normal runtime
graph, formatting and conventions pass. No native window, GPU or platform IME
service ran. The explicit pre-window consumer refusal remains in force.
[Integrated source and log receipt](evidence/native-ui-window-main131.json).

The [final independent integration review](evidence/native-ui-window-main131-root-review.json)
verified the unchanged source and log identities, with the complete
[checked source freeze](evidence/native-ui-window-main131/freeze.json) retained.
