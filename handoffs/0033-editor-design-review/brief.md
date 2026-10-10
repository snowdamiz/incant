# Full editor design pass — Opus 5.5, Max thinking

## Director's latest request — overrides earlier narrow visual scopes

"which thinking level are you running the claude models at? It should always be opus 5.5 at max thinking which is one level lower than highest which is ultra. Tell opus at this thinking level to do another thorough UI pass, especially the inspector right sidebar. It feels messy, cluttered, and slooppy, header to tall with bad layout, etc. Claude can change anything about the app it has full creative power to do what it think needs to be done. Do this while you continue your own work."

The runner has explicitly selected and verified Opus 5.5 with Max effort. Stay at
that model and effort. You own visual design and implementation for the entire
app with full creative freedom. This is a fresh, thorough design pass, not a
request for minor spacing tweaks or adherence to the earlier "do not redesign
unrelated UI" instruction in packet 0031. Read current PLAN.md and CLAUDE.md.
Astra continues renderer/camera and sprite correctness independently.

## Direction and scope

Rethink the Inspector right sidebar first: information hierarchy, overly tall
header, selection identity and actions, component headers, field grouping,
labels/values, dense nested objects/arrays, spacing, alignment, resizing and
scrolling. The director finds it messy, cluttered and sloppy. Design a coherent,
professional tool that makes complex entities easy to scan and inspect. Examine
all other app areas and change anything you judge necessary: shell, titlebar,
hierarchy, assets workspace, problems/console/history, agent and account states,
toolbars, dialogs, empty states, focus and responsive behavior. You can restructure
components and CSS, not just retouch existing rules. Choose the design yourself.

Earlier preferences remain useful context, not a layout straitjacket: connected
panels rather than spaced floating cards; neutral colors with restrained accent
rather than excessive purple; correct macOS traffic-light/logo alignment; assets
should have a considered home, not be crammed beside compiler problems. Balance
useful concise explanations with graphical communication. Do not hide important
content merely to reduce clutter. Keep actual product UI free of implementation
jargon and obsolete phase/spike guidance.

## Correctness boundaries and inputs

All app presentation files, tests, fixtures, icons and design documents are in
scope. You may propose any native shell or data-binding change needed and specify
it precisely in result.md for Astra to implement; do not let those requests prevent
you finishing the independently reviewable UI. Do not change Rust engine behavior,
command/schema/bridge transport contracts, auth storage or CI. Preserve offline
use, command-bus mutations, undo/redo, unknown/malformed value visibility, stable
IDs and accessible names. No GUI-only mutation path, fake working controls or
concealing unsupported capabilities. Refactor freely while preserving these facts.

The worktree includes the latest Camera/orthographic/pixel-perfect Inspector and
420 passing UI tests. Review representative complex entities: Transform, mesh
materials, compound colliders, character controller, navigation, lights and all
Camera variants, including unusual values and unsupported data. Include asset
selection/details and no selection. Native schema fixture parity must remain exact.
SpriteRenderer is being implemented on another branch and is not yet an app input.

## Review and acceptance

Do a genuine rendered review and iterate. Capture before/after at the normal app
size (1440×874), minimum window (1000×650), default and minimum Inspector widths
(approximately 331 and 280 px), plus larger space as useful. Design should remain
legible, deliberate and usable at each size with no accidental horizontal clipping,
overlaps, scroll traps or oversized repeated headers. Test long names, dense
components and a selected numeric field. Check keyboard navigation and focus,
contrast, loading/error/empty states, resizing and meaningful accessible labels.
Use browser rendering for iterations; computer use/screenshots are permitted again.
Astra will build the native app and supply native captures after integration for
your final rendered-pixel review. Do not claim native acceptance from a browser.

Run `npm ci --ignore-scripts` at the workspace root if necessary (the lockfile is
there), then all UI tests, typecheck and build with editor/ui/package.json scripts. Keep meaningful behavior tests, adapt
selectors to intentional design changes, do not remove coverage to make it green.
Use existing pinned tools. No Cargo in this handoff: request native integration.
Do not start local servers on an occupied port; use a free dedicated port and clean
up owned processes. Keep UI updates lightweight; avoid new heavy dependencies or
continuous work that competes with the native viewport.

Return handoffs/0033-editor-design-review/result.md with exact model and Max
thinking, changed paths, design rationale, commands/results, before/after screenshots,
accessibility/overflow measurements, requested bindings/native changes and precise
remaining limitations. Commit concise account-free evidence; full native captures
with an account label belong only in ignored artifacts. Commit your work with
Built-by: claude. Do not publish, merge, sign, access credentials, change external
accounts or approve phase gates. Work only in this worktree. Begin the redesign now.
