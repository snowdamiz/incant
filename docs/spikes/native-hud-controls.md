# Native HUD and world-text controls

This slice adds runtime HUD updates through the existing native structural
transaction and TypeScript callback. It changes committed declarations, not the
authoring document, journal, fonts, layout styles or widget topology.

`commands.setUiText(id, text)` changes the literal content of an existing Label,
Button or TextInput. `commands.setUiDisabled(id, disabled)` changes an existing
widget's own disabled flag. `commands.setText(id, text)` changes an existing
world-space Text. Native Rust uses `StructuralCommand::ControlUi` with
`UiControl::{Text, Disabled}` and `ControlText` with `TextControl::Content`.
Localized messages use the existing `localization.localize(request).text` result;
the controls do not introduce a second localization or layout system.

Calls stage in order. A preceding SetUi/SetText replacement is visible to later
controls in the same batch; removing or despawning an owner before a control is
an error. The final hierarchy, font bindings and combined text-byte bounds still
validate before publication. Changes share the numeric, structural, scheduling
and input transaction. A guest trap, caught invalid native call, bad final role,
quota failure or consumer refusal cannot publish a prefix. The existing failed
session policy remains terminal.

The whole native command batch admits at most 256 HUD controls, 16 KiB per text
and 1 MiB of submitted HUD text bytes. Existing aggregate scene text limits also
apply. Repeated controls coalesce through the staged per-owner declaration; an
identical final value retains the existing generation and immutable binding.

`tick.hud.widgets` and `tick.hud.texts` are bounded, readonly, owned pre-command
declaration snapshots sorted by stable entity ID. They carry no native loans or
authoring data. A retained snapshot stays unchanged; command functions expire
after the callback. Widget declaration text is distinct from a TextInput's live
edit buffer. Changing its declared content resets that buffer; assigning the
same declaration preserves edits. Reconciliation preserves eligible logical
focus and compatible pending typing, retires old receipts and preedit, and
clears focus when an ancestor becomes disabled.

The ordinary CPU composed player exposes `render_input.text_sources` and
`render_input.ui.sources` in its one final report. These are the actual retained
shaping sources, bounded by the existing consumer input. They are not per-tick
JSON projections or evidence of GPU drawing. `tools/probes/native-hud-export.py`
uses the public command bus and exporter and checks changed shaped text, native
motion, PCM, ordinary-load continuation and precise refusal paths after removal
of its disposable authoring/cache/build inputs and packaged debug source maps.

Source review is accepted; final independent execution audit is pending. The
affected runtime/script/render host suite passed 444 tests and the player suite
passed 10, with no failures or ignored tests. Scoped all-target Clippy, strict SDK
and fixture typing, generated SDK/bridge equality, compiler-disabled static/UI/audio
check, wasm32 runtime check and the normal-runtime dependency guard passed. Early fixture
compile/type-complexity failures and the corrected prototype-setter test scope
remain in the evidence rather than being counted as successes.

The fresh public export produced a 9,400,896-byte shipping player. Three separate
12-tick processes observed the shaped button `Santé 8`, TextInput `Done`, and
world Text `Pickup 12`, after English/French locale changes. Every callback checked
and logged its pre-command declarations. All three final saves and PCM outputs
were identical. A six-tick save followed by a fresh six-tick restore reproduced
the same final save and byte-for-byte concatenated PCM, including retained input
text/focus at the middle boundary. Three additional processes rejected a wrong
widget role, an uncaught guest trap and a caught HUD quota error with their exact
diagnostics, empty stdout, and no PCM or checkpoint file.

The preliminary helper expected the generic structural quota diagnostic instead
of the precise retained HUD quota error. That run and its outputs are retained;
the corrected helper repeated all five positive processes and three negatives
against the identical frozen package, with no engine rebuild. The public exporter
checked shipping codegen, normal dependencies and removal of its build storage.
The helper then removed its own authoring/cache directory and debug source maps.
It retains the original HOME value without overriding it, and supplies only the
copied player's directory in PATH. [Evidence](evidence/native-hud-controls.json)
binds the source, commands, raw/compressed logs and actual package files;
[source review](evidence/native-hud-source-review.json) covers the 19 production,
SDK and behavior-test inputs.

Rust/Wasm HUD imports and snapshots remain an explicit parity follow-up. No UI
styling, native window, OS IME, GPU appearance, mobile performance, physics save
continuation or complete Game UI gate is claimed. Existing session checkpoint
limitations remain in force. PCM and save publication are separate files, not a
multi-file atomic transaction.
