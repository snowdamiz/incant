## Priority revision: final typed import reconciliation (2026-10-11)

Finish the existing review first if this revision is read during resumption. Your
first result remains valid for its recorded source; preserve its evidence and
limits. Astra will integrate the reviewed current source into this handoff before
resuming it. Do not repeat the entire 102-state capture sweep without a reason.

The current reviewed source now includes Astra commit c131ed83975610861498f6993e52f04eebd3e35b:
`paths.ts` recognizes exact case-insensitive `.atlas.json` plus WAV/OGG, while
rejecting generic JSON, unsupported audio and unsafe traversal. `AssetsContext`
exports a typed `reimportRequest`; draft/reimport requests include texture usage
only for ordinary image sources, never atlas/audio/model. Existing ordinary-image
saved settings remain. Root independently reviewed the five source/test files;
515 full UI tests, strict typecheck and production build pass. This fixes the
import blocker you observed. Do not undo or broaden the validated mutation logic.

Astra also fixed your account-switch finding: the existing stateKey now includes
activeAccount. Both the focus/Escape regression and stale sign-out confirmation
regression fail on the old source and pass with the fix; all 27 account tests and
strict typecheck pass. In the focused browser review, verify that Use moves focus
to Close and Escape dismisses, and that a host account change clears an old
sign-out question. Do not modify the reviewed account request or credential logic.

Please reconcile presentation for these supported types using the existing plain
style: correct asset/import source labels/icons and atlas detail controls. A
texture whose source is `.atlas.json` is a color atlas; do not present an editable
normal/mask interpretation that native import does not support. Ordinary-image
texture settings must remain editable. Audio presentation must accurately describe
WAV/OGG, with no invented capability. Preserve layout and all five requirements.
Any additional behavioral issue belongs to Astra; report it rather than changing
Rust/schema/transport/credentials. Tests for your actual presentation changes are
in scope. Run the full UI tests/typecheck/build after final code changes.

Make a focused rendered check of successful atlas and WAV/OGG source selection,
atlas details/reimport, ordinary-image interpretation/reimport, and refused generic
JSON. Drive account-free browser doubles through the actual routes, and distinguish
native-supported behavior from mocked responses. Preserve the prior comprehensive
Camera/SpriteRenderer review; recapture affected states, not unrelated baseline
screenshots. Keep exact source hashes and evidence. No Rust/Cargo and no interaction
with the director's running native app. Native final-pixel acceptance stays pending.

Commit final source/evidence with `Built-by: claude`, and update result.md to state
both the earlier complete review and this final focused source revision. Do not
publish, merge, or push. Astra will build the final native executable and publish
the reviewed UI+sprite+PLAN union for a single final hosted matrix.

# Final plain-editor and sprite/atlas union review

Astra owns integration; you own rendered review and any narrowly justified visual
corrections. Required model: Claude Opus 5.5, thinking Max, explicitly selected and
confirmed by the ACP runner before this prompt. No substitution. This is a focused
release-union review, not another broad redesign.

## Source and current verification

The code source is a20d258ba75fd00b2c8bea91cf71de8bead78370, merging plain editor
ce06c31 (including your cdf0fab status-dot correction) with sprite/atlas PR58
8a7fbaf. PLAN7 authored commit8fe00eca is now merged as65129c8; that merge changes
no editor, Rust runtime, schemas, dependencies, Cargo flags or workflows. Read the
current PLAN.md/CLAUDE.md: no calendar targets; no phase gate is approved.

The union uses the real current Camera.schema.json, including perspective,
orthographic and pixel_perfect. SpriteRenderer/SpriteAtlas schemas and rendering
are now present. Do not restore the obsolete main49-only Camera fixture or invent
unsupported native schema values. Preserve actual malformed/unknown field display.

Astra independently verified logs:451 release workspace tests,56 ignored GPU tests,
release all-target Clippy, strict bridge generation check and native custom-protocol
build pass. All503 UI tests and production build pass. The previous504 count was
from the pre-atlas fixture; inspect current tests rather than claiming an old count.
40 Python tools tests and conventions pass after the PLAN union. See
`docs/spikes/editor-release-union.md` and its JSON evidence. No Cargo is needed.

## Director's five requirements — preserve the completed plain design

1. Remove useless explanatory text and rows throughout the app, including the
   compound-collider paragraph and permanent Entity/ID/read-only metadata row.
2. Keep the account dialog compact and professional while preserving switching,
   adding, sign-out, loading/error behavior and focus handling.
3. Keep keyboard shortcuts as a useful professional searchable table, with all
   actual actions, platform notation and accessible associations.
4. Keep only the profile icon/avatar in the top-right account control, no long pill.
5. Keep decorative button edges, glows, icon backdrops, nested boxes, repeated
   titles, excessive metadata and similar ornament removed throughout the app.

The original exact feedback and the plain-pass matrix are recorded in
`handoffs/0033-editor-design-review/brief.md`, `result.md`, `native-delta-0d14f4f.md`
and `editor/ui/DESIGN.md`. Preserve Astra's menu/focus, live status, diagnostic
reveal, clipboard failure, bounded lists and paging fixes. All mutations continue
through the existing bridge/command bus. Keep offline behavior and real auth logic.

## Required rendered review

Use an isolated browser review and synthetic/account-free fixtures. Reuse existing
0033 capture helpers when appropriate, but inspect the current source/schema union.
Review all three real Camera projection variants, sprite/atlas inspector data and
the changed integration fixture, normal1440x874 and minimum1000x650 windows, normal
and narrow Inspector widths. Include long names, malformed/unknown values, compound
parts and large navigation data, account dialog states, keyboard shortcuts,
profile-only titlebar, menus, output paging, agent states and empty/no-selection.
Exercise keyboard/focus transitions, End then Tab reveal, grid paging and repeated
pager announcements where feasible. Record actual observed states and failures;
headless fixture results are not native or device evidence.

You may edit editor/ui presentation, fixture/test/capture helpers and this packet
only for concrete regressions. Do not broaden into redesign. Do not touch Rust,
Cargo, CI, schema contracts, bridge transport, credential storage or PLAN. No new
external runtime dependencies, no fake capability or removed assertions. If code
changes, rerun full UI tests, strict typecheck/build and capture the affected result.
If no source change is needed, return precise rendered evidence and remaining gaps.

## Native safety boundary

The director is actively using the older native0d14f4f preview. Do not launch,
focus, type into, close, capture account-bearing content from, or otherwise drive
that app. No system-wide input or focus change. Do not read account data or copy
its settings into a new instance. The currenta20 native binary was built but has
not been opened or visually accepted. Do not label older captures as this union.

A separate review app is optional only if safe independent app-state isolation
can be established without changing the running app or rebuilding Rust. Otherwise
finish the full browser review and explicitly retain the exact native checklist
as unobserved. Do not let an unsafe native route block useful browser verification.
Astra can later provide a separately isolated native capture; never manufacture it.

## Return

Write `handoffs/0035-editor-union-review/result.md` with exact model/effort, source
identity, observed coverage, commands/results, concise account-free screenshots
and hashes, any narrowly justified changes and native limitations. Keep bulky raw
captures under ignored artifacts; commit only useful account-free evidence. Commit
with `Built-by: claude`. Do not push, publish, merge or run Cargo. Astra will review
source, evidence and any behavior changes before publication.
