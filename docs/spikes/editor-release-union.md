# Plain editor release union

The release combines the plain editor and accessibility corrections through
ce06c31 with the reviewed sprite/atlas source. PLAN revision7's authored8fe00eca
commit and the user's main merge6a759c6 are preserved. The final branch also
contains corrected sprite head5b3071b, whose17 hosted checks passed on Windows,
Linux and macOS. Only a STATUS conflict prevented merging that earlier PR directly
after the user's PLAN update; the final UI PR validates the combined tree once.

[Local baseline evidence](evidence/editor-release-union-2026-10-10.json) records
source objects and log hashes:451 release workspace tests,56 ignored GPU tests,
release all-target Clippy, generated bridge,503 UI tests/build and the native
custom-protocol build at a20d258. Later changes below are separately validated;
those historical counts do not claim a build of newer frontend source.

Claude Opus5.5 at explicitly verified Max completed the full browser review in
[handoff0035](../../handoffs/0035-editor-union-review/result.md), commit9fb82e6:
102 captures, the three real Camera projections, sprite/atlas Inspector states,
account flows, shortcut notation, keyboard focus, paging, menus and narrow/minimum
windows. All43 committed evidence hashes were independently verified. The plain
layout survived the union; the review exposed the two behavior defects addressed
below. The final focused reconciliation is now complete at65e1717: atlas color-only
controls, audio labels, real import paths and connected-account recovery were
reviewed in26 additional captures. All60 evidence hashes were independently
verified;14 Camera/SpriteRenderer baseline captures remained pixel-identical.

The generic sprite Inspector still lacks explicit untagged Region presentation,
semantic channel/border labels, schema field order and bounded atlas-frame
metadata. It reports unsupported values truthfully. Those items and the existing
inactive-title contrast finding are separate presentation follow-up work; they do
not imply a complete native 2D authoring or gameplay path.

The director's running native0d14f4f preview remains untouched. The final28e5491
frontend passed524 UI tests, strict typecheck/build and a custom-protocol native
build. Claude's final native review uses a separate app and project, with account
store access and network denied by a verified per-process sandbox. Native review
and exact-head hosted checks are pending; earlier captures and headless Chrome
do not certify this native binary. [Final source/log evidence](evidence/editor-release-final-2026-10-11.json).
The staffing key is now `initial_staffing_confirmed`; device/performance gates,
phase approval and complete engine/game release requirements remain open.

## Typed atlas/audio import guard, 2026-10-11

The release review found that the editor still rejected native-supported atlas
and audio paths before dispatch. Source classification now distinguishes atlas
descriptors and WAV/OGG audio from models and ordinary images. Only the exact
`.atlas.json` suffix admits JSON; case normalization matches the native importer.
Generic JSON, unsupported audio formats and unsafe project-relative paths remain
rejected. Draft and reimport requests omit stale texture interpretation for atlas,
audio and model sources; ordinary image reinterpretation and saved defaults remain.
A bridge regression verifies all three new source formats dispatch as one atomic
`asset.import` command.

Independent review, 46 focused tests, all 515 UI tests, strict typecheck, production
build and bridge generation pass. No styling, layout, Rust or command contract
changed. Claude completed atlas color-only controls and source labels at65e1717. The
final native build includes these changes and the completion-focus correction below.
[Exact logs and limits](evidence/editor-typed-import-2026-10-11.json).

## Connected account switch focus

Claude's final browser review found a pre-existing behavior defect: switching
between two connected accounts removed the focused Use button without recovering
keyboard focus, so Escape no longer reached the dialog. The active account is now
part of the existing host-state recovery key. A changed active account also clears
a stale sign-out question. No credentials, styling or account requests changed.

Both new observable regressions fail on the original source: focus lands on the
body, and the old confirmation remains. With the fix all 27 account tests and
strict typecheck pass. The focused rendered recheck passed in Claude's final reconciliation; final
full UI validation passes524 tests.

## Asset completion focus

Native import completion replaces its initiating control. Recovery now returns
keyboard focus to the reset import path or surviving asset heading only when that
control disappeared and focus remains on the document body. Deliberate navigation
and mounted-control focus remain unchanged. Four regressions cover successful
import, successful/failed reimport and navigation; three fail before the fix.
Root independently reviewed28e5491. Final524 UI tests, strict typecheck/build,
bridge/conventions and40 tools tests pass. The native build completes in28.34s;
that is an incremental local build measurement, not a clean/hosted latency claim.
