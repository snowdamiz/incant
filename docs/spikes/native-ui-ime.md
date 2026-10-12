# Managed native desktop text input

The existing window host now activates one native text context for the accepted
editable widget. Its monotonic session ID is bound to the opaque `FocusToken`,
including its world/owner identity. Native callbacks carry that immutable ID
before entering Tao's event queue. A retired callback cannot acquire the next
widget's authority at delivery. The adapter discards stale receipts before
examining or copying their payload, counts them, and retains strict typed
refusals for invalid current-owner operations.

This is the next CPU implementation after
[native window UI delivery](native-ui-window-input.md), using its ordinary
composed tick and receipt queue. It introduces no player mode, authoring edit or
second simulation path. The branch retains the explicit unsupported UI draw
guard until the separately reviewed Claude0056 consumer is integrated. These
checks do not establish native IME or rendered-game acceptance.

## Native contexts and ownership

Tao remains pinned to0.37.1, now selected through a local source patch. All122
vendored files are pinned against the registry package; 22 are added or changed.
Licenses and package provenance remain intact. The962 package records in the
workspace lock retain exactly their existing versions and dependency arrays;
only Tao's registry source/checksum becomes the local path. The baseline and
changed hashes are in the [source packet](evidence/native-ui-ime/source-freeze.json).

The opt-in `Window::set_text_input_session(serial, enabled)` API leaves legacy
Tao behavior unchanged until called. No-UI windows never call it. macOS creates
an `NSTextInputContext` and client per owner; GTK creates an `IMMulticontext` per
owner. Callbacks retain an immutable serial and an independent live flag.
Retirement disables callbacks before invoking native discard/deactivation. Cocoa
clients hold the window weakly, with no retained client/context cycle. Rust
mutable borrows and Windows state locks never span native reentrant calls.
Windows creates a separate owned IMM context, verifies the associated handle
before reading composition, and retires queued old character/change hints under
a bounded drain before replacing the owner. This is source-reviewed behavior;
interactive platform execution remains pending.

Cocoa marked contents are retained within the 16 KiB byte ceiling. Active
`unmarkText` accepts the latest marked contents once, following the
[Apple NSTextInputClient contract](https://developer.apple.com/documentation/appkit/nstextinputclient/unmarktext%28%29).
An explicit insertion clears marked contents first, so a later unmark does not
insert twice. Retirement clears them before native discard callbacks, and empty
preedit cancels composition without deleting a committed selection. Both
UTF-16 decoding and range conversion reject invalid surrogates and split scalar
boundaries. GTK scalar cursor offsets and Windows UTF-16 offsets become explicit
UTF-8 byte ranges; no lossy replacement or inferred character keys are used.
Unsupported surrounding-text replacement/reconversion fails explicitly.

Consumed native text keys do not also submit a widget or press gameplay keys.
Any previously forwarded gameplay hold is neutralized once. Receipts are bounded
before copies by the existing 256 events / 64 KiB packet and 16 KiB operation limits.
This bounds engine-owned receipt/state storage; it is not a bound on OS input
methods or Tao's preexisting native event queue. GTK's returned temporary string
is owned by GTK before the engine's checked copy.

Physical blur, suspend, minimize and owner replacement retire the context.
Viewport-only relayout and temporary surface acquisition deferral preserve the
accepted focus, composition, queued text and native context. Successful UI/world
acceptance clears receipts. A new checkpoint-restored UI owner has a fresh token
and therefore needs a new context; native serials and live composition are never
checkpoint authority.

## Verification

The [execution evidence](evidence/native-ui-ime.json) and
[independent root review](evidence/native-ui-ime-review.json) bind the exact 28
changed source inputs and all 122 vendor files. The final checks passed:

- 105 UI/input/player CPU tests across 17 result groups, 0 failed or ignored.
- 8 tests compiled from the exact native protocol/helper source, including Cocoa
  unmark, insert, retirement, empty preedit and UTF-16 admission sequences.
- Scoped all-target Clippy with warnings denied for the selected engine crates.
- Tao compile checks for x86_64 Windows GNU and aarch64 Linux GTK 3.24.41; macOS
  native-window player compilation and CPU tests. All use Rust 1.99.0.
- Authoring-free normal runtime dependency checks, formatting, conventions and
  diff checks. The isolated Linux container exited 0 with no child remaining.

The two review findings have retained red/green controls: the old stale payload
path refused with `UiCapacity`, and the previous Cocoa unmark semantics failed
2 of 8 exact-source sequence tests. The corrected source passes both. Earlier
compile/API/setup failures are retained and described in the evidence; they are
not counted as successful checks. The vendored dependency still emits upstream
and cfg-specific warnings (21 on macOS, 8 on Linux); scoped engine Clippy success
does not claim a warning-free upstream library.

## Remaining native acceptance

No native window, actual input method, GPU, screenshot, provider or device test
ran in this slice. Windows/Linux compile evidence is not interactive platform
acceptance. Precise candidate-window caret placement still needs Claude's
canonical positioned glyph geometry, including negative RTL overflow and
mixed-font baselines. The existing position API is preserved, with an explicit
window-origin fallback until that geometry is connected. No estimated character
width is introduced. Actual native interaction and screen-space drawing must be
validated on the combined consumer branch. Widget styling, visual bidi/hit
geometry, mobile text APIs, clipboard and surrounding-text reconversion remain
separate work; this does not complete the Phase 1 game-UI requirement.


## Actual main136 integration

The accepted implementation is integrated on `d418c6365c2c3ef8c8201a84c64f16359b53488a`.
Only STATUS required a conflict resolution. All 27 accepted non-lock source inputs
and 122 vendor files remain byte-identical. Compared with actual main136,
Cargo.lock changes only Tao's registry source/checksum to the pinned local source;
all 962 package versions and dependency arrays are preserved. Seven local-package
dependency additions already on main remain intact relative to the older source
branch. All 208 records in Tao's transitive lock closure are unchanged.

The [integration evidence](evidence/native-ui-ime-main136.json) binds 1,453 frozen
inputs and nine completed command logs: 119 CPU tests across 18 groups, eight
exact protocol tests, scoped all-target Clippy, editor all-target compilation,
Windows Tao compilation, dependency guards, formatting and conventions pass.
The earlier Linux GTK compile is reused with identical Tao sources and full
transitive dependency closure; it was not executed again. A missing-rustc PATH
setup failure occurred before the protocol compiler launched. The remaining
commands then passed using the exact installed Rust 1.99 compiler; already-passed
commands were not repeated. Both runners are terminal and reaped.

The [final source review](evidence/native-ui-ime-root-final-review.json) and
[independent integration closure](evidence/native-ui-ime-main136-root-review.json)
are retained.
No native window, OS input method, GPU, provider or device was exercised during
integration. The existing pre-window UI refusal remains until the separately
reviewed draw consumer is composed and its native behavior accepted.
