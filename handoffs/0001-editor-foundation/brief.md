# Phase 0 Spike 6: hierarchy panel and editor foundation

## Goal
Prove a Claude 5.5 ACP handoff end to end, and deliver the visual shell for the
Incant native-surface spike. You own design, implementation of visual UI, and
pixel review. The repository starts from PLAN.md; no visual assets exist.

## Deliverable and scope
Create a React + TypeScript + Vite editor in editor/ui, with a hierarchy panel,
selection, schema-driven inspector presentation, viewport area, history and console
panels, agent/chat area and provider connection state. Use locally bundled assets.
Design and document accessible typography, spacing, colors and keyboard focus.
Keep all engine state behind an injected typed bridge in editor/bridge. Astra owns
that bridge and the Rust command bus; do not create a second document model or fake
live AI features. A sample fixture is acceptable only if explicitly labeled.

Return a working hierarchy-panel first if the full shell needs a second packet.
Do not implement other phases or visually approve native rendering before it runs.

## Acceptance
- Hierarchy, selection and errors remain legible at 1280×800 and 1920×1080.
- Keyboard-operable actions, visible focus, semantic labels and contrast checks.
- Empty/loading/error states; no credential values in screenshots or fixtures.
- `npm run build --workspace editor/ui` and TypeScript checks pass.
- Before/after screenshots (initial blank state is an acceptable before).
- Document native viewport integration constraints and open questions.

## Paths and commands
Read PLAN.md sections 1–5, CLAUDE.md, STATUS.md, editor/bridge if present.
Use `npm ci` at the root; add editor/ui as a workspace if needed.
Engine tests: `tools/cargo test --workspace`.
Headless capture will be supplied by Astra at `tools/cargo run -p incant_headless -- screenshot ...`;
until that command exists, use a real browser screenshot of the shell and explicitly
mark native viewport evidence pending. Never synthesize a screenshot of a test.
Do not change crates/, services/, tools/, or sdk/ without requesting integration.

## Return
handoffs/0001-editor-foundation/result.md: model ID, outcome, changed files,
commands/results, screenshot paths, design rationale, limitations and next steps.
Commit with `Built-by: claude`. Leave branch unmerged for Astra's review.

## Manual fallback
After authenticating with Claude Code, run Claude from a git worktree on branch
handoff/0001-editor-foundation and ask it to execute this brief using Claude 5.5.
The original packet and evidence must remain available even when ACP is blocked.
