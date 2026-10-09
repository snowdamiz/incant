# Resume native editor and landing-page visual review

## Latest director authorization — 2026-10-09

The director now says: “you can start using computer use or screen capture again
when needed”. This explicitly supersedes the temporary capture/computer-use stop
in older handoffs and historical reports. Browser review, screenshots and needed
native interaction/capture may resume. Keep all raw native screenshots containing
real account information local/uncommitted. Never expose credentials or change,
sign out, revoke or reauthenticate the director’s working OpenAI connection.

The director also authorized agents to merge completed PRs once reviewed and
passing checks. Astra owns integration and those merges; return reviewed commits
from this scoped worktree without pushing or merging the separate worktree.

## Goal and ownership

Use the configured Claude Opus 5.5 through ACP. You own visual assessment and all
layout/style/look-dev changes. Complete the deferred review and polish obvious
issues in the current connected native editor and product site. Do not restart
the design: preserve neutral charcoal connected editor panels, warm paper/ink
landing page, restrained accents and the established wisp mark. Purple excess
was rejected. The director wants clean, modern, easy-to-use UI and correct native
traffic-light/logo placement. Astra continues engine logic separately.

The landing-page correction must find a middle ground: clear graphics and genuine
platform SVG marks, plus a short visible explanation for each workflow step.
Do not turn it back into dense paragraphs or remove all meaning from the steps.
Public copy describes the planned product as shipped, as explicitly requested;
internal evidence must distinguish implemented features and illustrations.

## Specific review

1. Native titlebar: traffic-light position, logo clearance and vertical alignment,
   drag region, active/inactive appearance, minimum size and resize. Compare actual
   native pixels, not just browser window-chrome doubles. The original concern was
   logo overlap/closeness and mismatched traffic-light alignment.
2. Connected editor: panel boundaries, hierarchy/inspector legibility, focus states,
   account dialog opening on a safe control, keyboard/resize behavior and little
   clipping/alignment defects. No disconnected spaced-out panel redesign.
3. Landing page: render current workflow and modeling sections at 320, 390, 768,
   1280 and 1440 px. Check icon/explanation grouping, middle-ground copy density,
   spacing, SVG proportions, wrapping and overall hierarchy. Review the actual
   current content, not the older palette screenshots. Improve concrete defects.
4. Native menu Undo/Redo correctness was implemented by Astra after earlier native
   review. If you can perform supported native interaction, verify text-field Undo
   edits text while project Undo reverses the command-bus transaction. Do not claim
   a button-only undo proves Cmd+Z. Report any logic defect; Astra owns Rust/bridge
   changes. Use disposable project data and keep the logged-in account intact.

## Paths and boundaries

- `editor/ui/src/`, `editor/ui/DESIGN.md`, `editor/ui/TITLEBAR.md`
- `editor/app/src/window.rs` and `menu.rs`: read for context; request exact changes
  in `native-requests.md` rather than editing nonvisual Rust integration.
- `website/src/`, `website/tests/`, `website/README.md`
- Prior evidence: `handoffs/0006-connected-editor/`, `handoffs/0007-landing-neutral/`,
  `docs/spikes/connected-editor.md`, `docs/spikes/landing-page.md`.
- Ignore older active-sounding capture prohibitions: they were superseded above.
- Do not edit engine crates, auth/credential storage, project schemas or command
  behavior. No other model substitution, environment HOME/CODEX_HOME overrides,
  synthetic native-input helpers, account changes or external publication.
- Supported native automation may not be available to the Claude CLI. If it is not,
  write the exact app path, disposable project path and interaction/capture request
  to `native-requests.md`, then continue browser review. Astra can operate the
  native test mechanically and supply local captures for your visual assessment.

## Commands and constraints

Repository uses pinned dependencies; do not upgrade them for this review.
- Root: `npm ci`, `npm run build --workspace editor/ui`,
  `npm test --workspace editor/ui`, `node tools/build_bridge.mjs --check`.
- Site: `npm ci --prefix website`, `PAGES_BASE_PATH=/incant/ npm run build --prefix website`,
  then `npm test --prefix website` (Playwright with its configured browser).
- Read `tools/editor-dev.py`, `editor/ui/NATIVE_VIEWPORT.md`, and prior native
  requests for the current app-launch path. Avoid rebuilding the whole Rust
  workspace unnecessarily; request the current root development bundle if useful.
- The main checkout’s landing preview currently serves `/incant/` on 127.0.0.1:4176;
  use a separate port for your worktree if needed. Do not replace unrelated servers.
- Preserve bundled local fonts/icons and existing performance budgets. Keep CSS,
  DOM and image work modest; record production bundle size. Avoid source-token or
  cosmetic snapshot tests that merely mirror implementation. Use behavioral checks.

## Return

Commit focused visual changes with `Built-by: claude`. Write `result.md` with exact
model, findings, changed paths, tested commands/results, and honest limitations.
Keep before/after browser screenshots inside this packet when safe; raw native
screenshots with real account labels stay ignored/uncommitted. Separate native
observations from browser-fixture observations. Record unresolved native requests
promptly so Astra can continue in parallel. A successful build alone is not visual
review, and no phase gate is self-approved by this work.
