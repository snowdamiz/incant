# 0046 persistent project agent allowance controls: result

## Status

**Done for this packet's scope, verified in the browser only.** The Agent pane now shows
and edits the durable per-project token allowance. All edits go through the real
`agent.budget.set` command and the existing native bridge contract.

- **What changed:** the pane's last line states the allowance against recorded usage,
  and Change edits it in place.
- **Older hosts:** they keep their per-session allowance, with no editor and truthful
  wording.
- **Fences:** cap edits, Undo and Redo show "Updating…" instead of flashing a note.
  Decisions wait while the session's cap lags the project's, and Stop stays usable.
- **Not native evidence.** Evidence is deterministic and browser-only: headless Chrome ran
  the production build with the real compiled bridge (`editor/bridge/native.generated.js`,
  unchanged) on a scripted fake host IPC. These results cover UI and bridge behaviour only.
- **Nothing native or external was run.** No native renderer was launched, as the packet
  says. No account, credential, provider or external state was read, used or changed.
- **Not claimed:** native acceptance, any phase gate, any performance figure, any cost or
  pricing feature.
- **Packet revisions:** the current brief has no separate priority-revision section and
  no new director feedback beyond its text. The worktree was clean at `79b5b7c`, with no
  interrupted attempt. As the brief says, it supersedes handoff 0045's statements that all
  usage is session-only and that no budget command exists. Both are corrected here; see
  "Wording".
- **Commits** (all with `Built-by: claude`):
  - `710f9a1`: source, tests, DESIGN.md, tools and screenshots.
  - `ff6d9ae`: Latest is offered only for new conversation output (found in rendered
    review).
  - `36b4ee5`: one test reordered to match the native bridge's event order.
  - The last commit adds this file and the follow-up patches.

## Model and effort

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`), as reported by this session's runtime.
- **Thinking effort:** max, as configured for this session (`reasoning_effort: max`).
- **Verification limit:** this session cannot query ACP itself, and the worktree has no
  ACP verification record. These values are the session's own configuration, not an
  independent ACP check.

## Design

The director wants thoughtful restraint, not a dense form. So the allowance is one quiet
line with an in-place edit, built from patterns the app already has: the rename field,
the Inspector's value box and the existing notice line.

- **Where it lives.** The allowance line is the open pane's last line: under the composer
  when the agent can run, under the cannot-run note when it cannot.
  - It is outside the conversation log, so editing never interacts with the log's
    follow-scroll or the sticky approval row.
  - It stays reachable without an account, so the allowance can be changed offline.
  - It replaces the old in-log usage line, so the fact is still said once.
- **The line.** "7,712 of 250,000 tokens used", plus a quiet Change (a text link) on the
  first line's trailing edge.
  - The allowance is the project's authored cap. Usage is the session's recorded total,
    marked "(some estimated)" when part of it is a conservative charge.
  - The numbers turn amber at 80 %, red when used up, and stay muted at 0, which is a
    deliberate setting rather than an alarm.
  - Usage above a lowered cap stays visible. Only "N tokens left" (spoken, and in the
    tooltip) is clamped at zero.
  - There is no reset and no money anywhere, only tokens.
- **Changing it.** Change turns the line into: "Allowance", the Inspector's 28 px value
  box, Cancel and Save.
  - The box shows the value in monospace digits, fully selected, with "tokens" following
    the digits.
  - Enter saves, Escape cancels, and focus returns to Change.
  - Save is the raised button, so Send or Stop keeps the pane's one accent action. The
    first render had three accent buttons stacked at 280 px.
  - One quiet line under the box says only the most important thing:
    - the input problem;
    - the host's reason a save failed (what was typed stays);
    - "Saving stops the current request." while a request runs, because native hosts
      cancel an active turn on a cap change (`docs/spikes/agent-project-budget.md`);
    - "Changed to 400,000 by another edit.";
    - "0 turns the agent off.";
    - "No new requests: 7,712 already used.";
    - otherwise, the usage the new value is weighed against.
- **Header statuses.** The header, which stays visible when the pane is collapsed, now
  says something specific: "Allowance used up" when the allowance is exhausted, "Off"
  when it is 0, and "Updating…" during a refresh.
  - The cannot-run note's one sentence names the next step: "Raise the allowance to send
    new requests." (or "…to turn the agent on." at 0). The step itself is the line's
    Change.
- **Fences.** After an allowance edit, Undo or Redo, or an account read, the bridge holds
  requests until it re-reads the session.
  - The pane keeps the composer and its draft, shows a quiet wait action and says
    Updating…. Without this, every Undo would swap the composer for a note and back.
  - Decisions wait with a reason while the session's cap differs from the project's.
    Stop stays usable throughout.
  - If the host never finishes the refresh, after 4 s the host's reason and Refresh appear
    in the existing notice line above the same composer. The 4 s count starts after the
    pane's own save finishes.
- **Narrow panes.** When the line is narrower than 320 px (the 280 px column), or the
  value is longer than 12 characters, the actions move beside the feedback line. All
  digits stay visible up to 9,007,199,254,740,991.

## Correctness and safety

- **Dispatch.** Every save is `shell.dispatch({ type: 'agent.budget.set', maxTokens })`
  through the shared command bus. The pane never writes local snapshot state and never
  calls host IPC directly.
  - An unchanged value sends nothing, so History gets no empty entry.
  - A save is sent once however many times Enter or Save is pressed: the in-flight guard
    is set synchronously in the per-bridge store.
- **Values.** Input accepts plain digits, or digits grouped by one consistent separator
  that is not the locale's decimal mark: "1,000,000", "1.000.000" in de-DE, "1 000 000"
  and "10,00,000".
  - Refused inline, with nothing rounded or clamped: negatives, fractions, exponents,
    letters, uneven groups (such as "1,000,00,000") and anything above
    9,007,199,254,740,991.
  - Zero and the maximum are sent exactly.
- **Truthful sources.** The authored cap (`projectAgentBudget`) is the allowance.
  - While the session catches up, its lagging `max_tokens` is never displayed.
  - Usage always comes from `agentSession.budget.used_tokens`.
  - On older hosts the session's own budget is the allowance, and there is no editor.
  - The 1,000,000 default appears only because the bridge sends it from the published
    schema; the pane never fabricates it.
- **Results.** A failed save keeps the typed value and shows the host's safe message next
  to the field.
  - If the save succeeded but the following session refresh fails, the saved value is
    shown along with the existing transport notice and Retry. The pane never claims the
    save failed.
- **Reconciliation:**
  - A success closes the editor only if nothing new was typed meanwhile.
  - An edit belongs to one project ID (`connection.project.id` when ready). It is never
    shown in, or saved into, another project.
  - A change made by another edit (Undo, another window) is reported in the feedback line
    and does not overwrite the user's typed value.
  - The open edit and a save in flight both survive hiding and re-showing the pane.
  - Recorded usage is never touched by Undo or any control here.
- **Accessibility:**
  - The field is named "Allowance in tokens for this project" and starts with the visible
    label word. Change's name is "Change allowance".
  - The feedback line is the field's `aria-describedby`. `aria-invalid` is set only for
    input problems.
  - The line speaks its scope ("Project token allowance:"), what is left, and the
    cause-neutral uncertainty sentence.
  - Validation and save outcomes are announced once in the status line.
- **Cause-neutral uncertainty.** "Some usage could not be confirmed or saved exactly, so
  a conservative amount was recorded." Root's denial and accounting corrections from
  `7e31cc1` are untouched.

## Wording

| Where | Before | Now |
| --- | --- | --- |
| Cannot-run, project host | "This session’s token allowance is used up." | "Raise the allowance to send new requests." / "…to turn the agent on." (0); without the capability: "The project’s token allowance is used up." / "…is 0." |
| Cannot-run, older host | same | unchanged: "This session’s token allowance is used up." (true for them) |
| Turn failure `budget_exhausted` | "The session’s token allowance ran out." | "The token allowance ran out." (neutral) |
| Header when used up / 0 | "Unavailable" | "Allowance used up" / "Off" |
| Native bridge fallback text | "The native agent session token allowance is exhausted." | unchanged here; scope-aware wording is follow-up patch 2 |

## Changed paths

| Path | What |
| --- | --- |
| `editor/ui/src/components/AgentPanel.tsx` | Allowance state, refresh presentation (Updating…, stall notice), headers, scoped reasons, decisions held while the cap lags, Retry wording |
| `editor/ui/src/components/agent/AgentAllowance.tsx` | New: the allowance line and in-place editor |
| `editor/ui/src/components/agent/tokenAllowance.ts` | New: pure allowance model, parsing and wording |
| `editor/ui/src/components/agent/agentMemory.ts` | Per-bridge store: open edit and save in flight |
| `editor/ui/src/components/agent/agentText.ts` | Neutral `budget_exhausted` text; shared number formatting; removed `sessionUsage` (no production caller left) |
| `editor/ui/src/components/agent/AgentTranscript.tsx` | Usage line moved out of the log; Latest only when content actually grew below the reader |
| `editor/ui/src/styles/agent.css` | Allowance line and editor styles |
| `editor/ui/src/components/agent/tokenAllowance.test.ts` | New: 42 cases |
| `editor/ui/src/components/agent/AgentAllowance.test.tsx` | New: 34 cases, including one on the real `NativeBridge` with mocked IPC |
| `editor/ui/src/components/agent/agentText.test.ts` | One existing case now calls the production allowance helpers instead of the removed `sessionUsage`; its expectations are unchanged |
| `editor/ui/DESIGN.md` | Agent pane section, plus the one Composition bullet on when the pane is resizable |
| `handoffs/0046-project-agent-budget/tools/` | `fake-host.js` (scripted IPC), `scenarios.mjs` (invented data), `capture.mjs` (captures and flows) |
| `handoffs/0046-project-agent-budget/screenshots/` | 58 state captures, 3 window captures, 2 flow images, `report.json` |
| `handoffs/0046-project-agent-budget/followup-*.patch` | Exact follow-up patches, **not applied** |

Not touched: bridge/backend, ShellContext, App, Play, Viewport, Splitter, Inspector, base
styles, CI, dependencies. `agentPaneResizable` (called by App) lives in AgentPanel.tsx.
It now also counts a session refresh as resizable, so the pane keeps its height while the
composer is kept.

## Commands and results

UI tools come from the parent checkout's pinned `node_modules`, symlinked into the
worktree. The symlink is ignored by `.git/info/exclude`, and the `package-lock.json` hash
matches (`e33abebd…`). Nothing was installed or upgraded.

| Command (repo root) | Result |
| --- | --- |
| `npm run test --workspace editor/ui -- --maxWorkers=4` at `79b5b7c` (baseline) | 40 files, 777 passed |
| same, final at `36b4ee5` (run twice) | **42 files, 853 passed** (777 prior + 76 new) |
| `node node_modules/typescript/bin/tsc -p editor/ui/tsconfig.app.json --noEmit --incremental false` | passed |
| `node node_modules/typescript/bin/tsc -p editor/ui/tsconfig.test.json --noEmit --incremental false` | passed |
| `npm run build --workspace editor/ui` | passed (`tsc -b && vite build`) |
| `node tools/build_bridge.mjs --check` | passed (the generated bridge is unchanged) |
| `git diff --check` | passed |
| `node handoffs/0046-project-agent-budget/tools/capture.mjs final` | 58 captures over 24 states, 10 flows: all clean (below) |

The packet asked to "preserve prior 777 passing UI cases". All 777 are present and pass,
including the `editor/bridge` suites the UI config includes. One of them now exercises
`allowanceOf`/`allowanceText` instead of the deleted helper, with the same three
expectations.

**Browser run**
- Chrome 155.0.8059.40 through playwright-core 1.64.0, at deviceScaleFactor 2 (flows at
  1), with reduced motion and the page's real CSP. axe ran through DevTools evaluation.
- Sizes: exact 280 and 380 px side columns in 1000×650 and 1440×900 windows.
- Every capture recorded zero for all of these: axe violations, page errors, remote
  requests, elements escaping the pane, horizontal scrollers, window scroll, raw-error
  text leaks, and clipped allowance values.

**Flows** (real keyboard and pointer input through the real bridge)
- **raise-from-exhausted** (1000×650, 280 px):
  - Tab reaches Change with a visible focus ring. Enter opens the editor with "250000"
    selected.
  - Typing 500000 and Enter sends exactly one `engine_execute`:
    `{commands:[{op:'set_agent_budget',settings:{max_tokens:500000}}], description:'Set project agent token cap', expectedRevision:7}`.
  - Header: Allowance used up → Updating… → Ready. The composer appears, focus returns
    to Change, and the status line says "Token allowance set to 500,000.".
  - Usage is unchanged: "250,000 of 500,000".
- **undo-redo:** titlebar Undo and Redo send `engine_history`. The cap moves
  250,000 ↔ 400,000 and usage stays 7,712. Statuses: Ready → Updating… → Ready, twice.
  **No cannot-run note appeared at any point.**
- **lower-while-running:**
  - The warning "Saving stops the current request." is shown.
  - While the change applied, Allow was `aria-disabled` with "Waiting for the agent to
    apply the new allowance. You can still stop the agent.", and Stop stayed enabled.
  - The fixture then ended the turn the way the native host is documented to:
    "The project changed while the agent was working.". The header ended at Ready.
- **escape-cancel:** no command was sent, and focus returned to Change.
- **duplicate-submit:** three submissions sent one `engine_execute`. Past the 4 s grace
  the header still said Updating…, the composer was kept and no Refresh was offered,
  because the save was still in progress.
- **save-failed:** the value was kept and focus stayed in the field, with the host's
  message inline. The status line said "agent.budget.set failed: Project revision
  changed." (see follow-up 1). Saving again succeeded.
- **stalled:** the session never caught up by itself. Updating… was shown first; after
  the grace, the host's reason and Refresh appeared above the kept composer and draft.
  Refresh recovered.
- **offline** (signed out): the save went through. Only engine IPC was used, with no
  provider call.
- **external-change:** an engine-side cap change produced Updating… → Ready with no note.
- **latest-only-for-output** (1000×650, 280 px): with the reader scrolled to the top, an
  allowance change kept the position and offered no Latest (content height stayed
  281 px). New conversation text then grew the content to 321 px, and Latest appeared
  without moving the reader.

## Screenshots

All are in `handoffs/0046-project-agent-budget/screenshots/`. Names are
`<state>-w<window>-s<column>-side.png` (a 2× crop of the side column), or `-window.png`.

- Ready, default allowance from the schema: `ready-default-w1440-s380-side.png`,
  `ready-default-w1000-s280-side.png`
- Ready with history and estimated usage: `ready-w1440-s380-window.png`,
  `ready-w1440-s380-side.png`, `ready-w1000-s280-side.png` (Change stays on the first line)
- Keyboard focus on Change: `change-focused-w1440-s280-side.png`
- Editing: `editing-w1440-s380-side.png`, `editing-w1000-s280-side.png`,
  `editing-w1440-s380-window.png`
- Invalid input: `editing-invalid-w1000-s280-side.png`
- Below usage, and 0: `editing-below-w1440-s280-side.png`, `editing-zero-w1440-s380-side.png`
- Running request (warning): `editing-running-w1000-s280-side.png`,
  `editing-running-w1440-s380-side.png`
- Saving (Updating…): `saving-w1000-s280-side.png`
- Save failed: `save-failed-w1000-s280-side.png`
- Used up, and raising it: `exhausted-w1000-s380-side.png`, `exhausted-w1440-s280-side.png`,
  `exhausted-editing-w1000-s280-side.png`
- Over a lowered cap: `over-cap-w1440-s380-side.png`
- Off (0): `zero-w1440-s380-side.png`, `zero-w1000-s280-side.png`
- Whole allowance reserved, uncertain: `reserved-w1440-s280-side.png`
- Signed out, editing offline: `signed-out-w1440-s380-side.png`,
  `signed-out-editing-w1000-s280-side.png`
- Older host: `older-host-w1000-s280-side.png`, `older-exhausted-w1000-s280-side.png`
- Largest value: `ready-max-w1000-s280-side.png`, `editing-max-w1000-s380-side.png`
- Collapsed: `collapsed-exhausted-w1440-s280-side.png`
- Stalled refresh: `stalled-w1000-s280-side.png`, `stalled-w1440-s380-side.png`
- Saved, then the session refresh failed: `refresh-failed-w1000-s280-side.png`
- Flows: `flow-raise-from-exhausted-side.png`, `flow-lower-while-running-side.png`

Every image listed was inspected. These files come from the final run, and 59 of its 63
images are byte-identical to the earlier run's inspected images. Of the 4 that differ,
the flow image was re-inspected. The other three differ by 15–36 pixels (maximum channel
delta 16), which is raster noise around the header's status dot, not content. The full
measurements are in `report.json`.

## Review and corrections

**Rendered review changed the design:**
- At 280 px, Change was orphaned on a third line. It now holds the first line, and the
  text wraps beside it.
- The unit sat at the box's far edge. It now follows the digits, as the Inspector does.
- Three accent buttons were stacked (Allow, Stop, Save). Save is now the raised button.
- A 16-digit value was clipped at 380 px. Long values now stack the actions.
- Consequence sentences wrapped to three lines beside the buttons. They were shortened.

**The flows found two bugs:**
- A save slower than the grace period was treated as a stalled refresh. The grace now
  starts after the pane's own save.
- In the keyboard flow's image, a floating Latest appeared after raising the allowance
  although nothing new had been written. Tabbing through the conversation's links had
  left the reader above the end, and the cap change bumped the session version, which
  the conversation treated as new output. Latest now appears only when the content
  actually grew below the reader.

Each fix has a regression test that demonstrably fails without it.

**A full-suite run with the patches applied found one racy test.** The test double
resolved the save before publishing the failed session read, so two status-line
announcements raced. The test now uses the native bridge's order, and the full suite
passed twice afterwards.

**An independent `code-review` pass (high) reported 10 findings:**
- **Fixed:**
  - A stalled refresh no longer swaps in a note inside a resizable pane. It keeps the
    composer and uses the notice line, which also resolves the resizable/capped mismatch
    the review raised.
  - Retry and Refresh now announce the pane's own sentence, not the host's raw text.
  - Uneven digit groups are refused.
  - The dead `sessionUsage` was removed.
  - Number formatting is shared.
  - The locale's decimal mark is cached, and input is parsed once per render.
  - The save outcome is decided in one place.
  - This result was written.
- **Defensive change, no bug claimed:** the stall flag now resets during render. The
  review described a one-frame note flash. Testing showed that sequence was not reachable
  with the earlier code, and the new test passes either way. It asserts the invariant
  only, and is not a claim of a fixed bug.
- **Kept as a limit:** the refresh signature is inferred from what is absent. Follow-up 3
  is the exact contract fix.

## Follow-ups (exact patches, not applied)

Each was applied to this tree at `36b4ee5`, verified, saved as a diff and reversed. All
three apply together with `git apply`, in the order 2, 3, 1 (1 is independent). The
combined state passes 855 tests (run twice), both strict type checks and
`node tools/build_bridge.mjs --check`. Blank context lines are written as empty lines
(GNU diff style). `git apply` accepts that, and it keeps the patch files clean under
`git diff --check`.

1. **`followup-1-shell-allowance-wording.patch`** (ShellContext, resolve.ts, one
   assertion).
   - A failed save is announced as "Token allowance not saved: …" instead of
     "agent.budget.set failed: …". The save-failed flow recorded the second form.
   - The capability label becomes "Change token allowance".
   - Verified alone: 853 pass, plus strict types.
2. **`followup-2-bridge-allowance-wording.patch`** (native.ts, regenerated
   native.generated.js, two description assertions).
   - The exhaustion reason is scope-aware: "The project’s agent token allowance is used
     up." versus "This agent session’s token allowance is used up." Today every host gets
     "The native agent session token allowance is exhausted.".
   - The refresh reason becomes "The agent session has not caught up with a recent
     account or project change.".
   - Bridge error messages use allowance wording.
   - The History description becomes "Set agent token allowance".
   - Verified alone: 853 pass, plus strict types and the bridge check.
3. **`followup-3-agent-unavailable-code.patch`** (apply after 2: contract.ts, native.ts,
   generated bridge, AgentPanel.tsx, plus 2 tests).
   - Adds optional `AgentState.unavailable.code`: `loading`, `signed-out`, `refreshing`,
     `transport`, `unread` or `exhausted`.
   - The pane believes a host's code, and the code can only narrow the refresh inference.
     A host that names another reason is never shown as Updating….
   - Verified together with 2 and 1: 855 pass.

Not patched (backend-owned): the Rust host's own messages, "The project's agent token
allowance is exhausted." and "The project or token cap changed. Retry the agent request."
(`editor/app/src/agent.rs`, `editor/app/src/agent/budget.rs`). They can surface as a send
error or turn error.

## Limitations

- **Chrome only.** Behaviour in native WKWebView and WebView2 is unobserved: container
  queries, `:has()`, `color-mix()`, the select overlay, and focus on click (Safari does not
  focus buttons on click). Root's immutable native build is needed for combined acceptance.
- **The fixture is scripted.** Native IPC latency and 250 ms polling are approximated. That
  a cap change ends an active turn as `project_changed` is modelled from
  `docs/spikes/agent-project-budget.md` and its review record, not observed natively. The
  "Saving stops the current request." warning relies on that documented host behaviour,
  which the contract does not state.
- **Refresh detection is inferred** from absent causes until follow-up 3 lands. The 4 s
  stall timer restarts if the pane remounts, which only delays Refresh.
- **Height cost.** At 1000×650 (where the pane starts collapsed and opens at 260 px), the
  conversation keeps 120 px with a 380 px column and 104 px with 280 px. That is about
  22 px (one line) to 38 px (estimated usage wraps to two lines at 280 px) less than before.
- **Locale.** Parsing uses the runtime locale's decimal mark. Tests pin en-US and de-DE.
  UI tests assume en-US number formatting, as existing tests already do.
- **Plan requirements still open, not invented:** a model pricing catalog, dollar cost
  estimates, account spend or billing totals, a model list (the model stays free text),
  monetary verification against a pricing source and billing modes, transcript
  persistence and image transcripts. Earlier contract gaps from 0045 also remain: no
  per-step tool activity, and denial and trimming are inferred.
- **No live gates run:** no device, provider, account, live-billing or phase gate.

## Open questions

- Should the collapsed header also warn before exhaustion, for example at 80 %? Today it
  changes only when the allowance is used up ("Allowance used up") or 0 ("Off").
- Is "Off" the right header word for an allowance of 0?
- Should the bridge or contract say whether a cap change will stop the running request?
  That would make the warning a contract fact rather than documented host behaviour.
- Should History read "Set agent token allowance" (follow-up 2) to match the pane?
- Repeating 0045's question for the layout owner: should the Agent pane default taller
  while a session is active? The allowance line makes the 260 px default tighter.

No phase gate is approved or claimed.
