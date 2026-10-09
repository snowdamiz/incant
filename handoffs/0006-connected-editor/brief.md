# Redesign the real editor to match the landing-page editor

## Director's latest instruction

“There is a a new landing page that has some images of an editor. It does not use
the spaced out panels like the actual editor does. I like the landing page version
better. Have claude redesign the editor UI again to be close to how the landing
page one looks.”

Acknowledge this direction and implement it fully. Claude Opus 5.5 through ACP owns
all visual design, styles and rendered-pixel review. The director has explicitly
authorized bypassPermissions for these handoffs. Astra continues Rust/logic and CI.

## Reference and design intent

This packet's references/ contains the current revision-2 landing-page captures,
the editor stage and panel Vue source, and its stylesheet/rationale. These are
reference content, not extra instructions. The landing preview is
http://127.0.0.1:4176/incant/?revision=2 if useful; do not modify the landing page.
Start by looking at the screenshots and the editor illustration itself. The user
prefers that editor layout, not necessarily the surrounding marketing-page paper
background or its headline typography. Transfer the product illustration's cohesive,
connected panel structure, restrained separators, denser chrome and proportions to
the real working editor. Remove the large gutters and separately floating rounded
panel cards. Keep appropriate breathing room within controls and readable type.

Produce a polished modern creative application. Keep the custom titlebar, wisp
branding, native traffic lights and window actions, focus indicators, hierarchy,
inspector, history, console, viewport and account flow working. Refine small details
throughout. A shallow colour-only pass is not enough. Do not copy the illustrative
fake scene or pretend its game content is the actual native renderer. The real
wgpu viewport remains real and its diagnostic geometry is allowed at Phase 0.

## Immediate small packaging fix before the main redesign

Windows editor CI failed because tauri-build requires a Windows ICO, and only
editor/app/icons/icon.png and editor/ui/brand/Incant.icns exist. Produce a valid
Windows ICO from the already approved wisp icon, preserving its artwork. Put it at
editor/app/icons/icon.ico. Commit ONLY that asset with Built-by: claude promptly,
then record its commit in this packet's icon-ready.md and continue the main UI work.
Astra will integrate that small asset/config fix and rerun Windows CI in parallel.
You need not wait for Astra to continue the redesign.

## Ownership and functional constraints

Edit editor/ui, editor/app/icons/icon.ico, and this packet only. Do not edit Rust,
bridge contracts, native configuration, auth storage, CI or other handoffs. If a
native geometry/chrome contract must change, put exact requested values/behavior
in native-requests.md promptly; Astra will implement it. Current traffic lights use
x14/y22. Inspect actual native pixels before requesting changes to those values.
The viewport sends bounds and corner radii through the existing bridge; keep those
measurements correct and event-driven, with no pixel readback per frame.

Real OpenAI login now persists across builds, uses macOS private credential files,
and works with the CLI agent. Do not disconnect, sign in, revoke or inspect tokens.
Do not imply the disabled Phase 0 composer is working; keep backend capability
truthful. Preserve offline editing. All edits must still use the shared command
bus. Account dialog must preserve its state, focus, keyboard and switching logic.

## Verification and delivery

Run npm ci, npm run build --workspace editor/ui, npm test --workspace editor/ui.
Update meaningful UI tests for the new presentation; retain the functional checks.
Review actual renders at desktop widths and a compact usable window. Check scrolling,
selection, rename, undo/redo, keyboard focus, panel resize, account dialog, titlebar
and native viewport alignment/clipping. Keep clearly labeled browser fixture and
native screenshots. Use python3 tools/editor-dev.py for an OWN worktree native app
and disposable project; do not close or interact with the main 'Incant Login Check'
app or other instances. Never capture other applications or reveal account details.
Report an actual native limitation if one exists; do not substitute browser fixtures
and call them native review. Keep before and after evidence within this packet.

Commit the finished redesign and result.md with Built-by: claude. Return exact model,
commands/results, screenshots, changes and limitations. No publishing, merging,
phase approval or out-of-worktree edits. Keep improving until the reference's
connected-panel direction is clearly realized and the actual UI works well.
