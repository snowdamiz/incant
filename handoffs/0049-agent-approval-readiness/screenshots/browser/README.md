# Browser captures, handoff 0049

**Browser evidence, not native evidence.** All of these images are deterministic fixture
data rendered in headless Chromium 156.0.8078.4, the build in the Playwright cache that
matches the pinned playwright-core 1.64.0. Nothing here comes from native WebKit, a live
provider or account, the network, an engine or the GPU.

- **Window:** 1440 × 900, device scale 2, dark colour scheme, reduced motion.
- **Clip:** the side column, which holds the Inspector and the Agent pane.
- **Widths:** the Inspector column is set to 280 or 380 CSS px by dragging its separator.
- **Data:**
  - handoff 0046's `approval` scenario: turn 3 is awaiting approval 4, and the project's
    allowance (250,000) equals the session's copy;
  - 0046's `tools/fake-host.js`, unchanged;
  - this handoff's `tools/readiness-host.js`, which holds Undo/Redo replies and session
    reads, and records decisions;
  - the real compiled bridge, `editor/bridge/native.generated.js`, unchanged, relabelled
    as a fixture.
- **Script:** `tools/capture.mjs`. Each folder's `report.json` holds the page
  measurements, axe results, recorded IPC calls and flow checks.

## Builds

| Folder | `AgentPanel.tsx` | SHA-256 |
| --- | --- | --- |
| `after/` | this handoff's | `513e583bd0bac0a5415ded100d3d5a1447adaaf0fe2e340804d53a83bd70dcb9` |
| `before/` | base commit `0ca2bcd`, the file named in Astra's review receipt | `b2afbc85629b1e6dd92fa018cd521d6abf289f19130e12fdeca83c84ba005cd5` |

Every other source file is the same in both builds.

## States

Each state has a 280 and a 380 capture, named `<state>-w1440-s<width>-side.png`.

| State | What happened | Bridge |
| --- | --- | --- |
| `pending` | Undo pressed in the titlebar. `engine_history` is held. | running, `approvalReady: false`; both allowances 250,000 |
| `pressed` | Deny, then Allow, pressed with the pointer during `pending`. Playwright's forced click also scrolls the button into view, so the conversation sits at its end from here on. | unchanged |
| `reading` | `engine_history` answered (History: 1 of 2 applied). The fresh session read is held. | running, `approvalReady: false` |
| `recovered` | The session read answered. | running, `approvalReady: true` |

**Flows.** Keyboard input on the decisions:
- `flow-keyboard-held.png` (380): Allow focused with Shift+Tab from the composer, then
  Enter and Space pressed during the hold;
- `flow-keyboard-decided.png` (380): Enter pressed after recovery; the decided row takes
  focus;
- `flow-redo-held.png` (280): Redo held, Allow focused, Enter pressed.

## Measured results

| | `after/` | `before/` |
| --- | --- | --- |
| Allow and Deny in `pending` and `reading` | `aria-disabled="true"` | usable |
| Note in `pending` and `reading` | "Waiting for the agent to update its session. You can still stop the agent." | none in `pending`; then the press's error |
| After pressing during the hold (pointer, Enter, Space) | nothing sent, nothing shown | "Connect an OpenAI account and refresh the native session before deciding an approval." (error) |
| `recovered` | decisions usable, no note | decisions usable, the stale error still shown |
| Stop during the hold | usable; Enter sends `engine_agent_cancel {turnId: 3}` | same |
| `engine_agent_decide` calls | none while held; one `{turnId: 3, approvalId: 4, approved: true}` after recovery | none while held (the bridge refused them); one after recovery |
| Draft "Then add fog over the water." (typed in the captures and the keyboard flow) | kept in each | kept in each |
| Header | Needs approval, then Working… after the decision | same |
| axe (Agent region), `pending` and `recovered` | 0 violations | 0 violations |
| Page errors, external requests, overflow past the pane, horizontal scrollers | none | none |
