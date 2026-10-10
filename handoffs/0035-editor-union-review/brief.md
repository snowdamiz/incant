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
