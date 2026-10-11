# 0045 native agent conversation and approvals: result

## Status

**Done for this packet's scope, in the browser.** The Agent pane now shows the real
bounded native session: streamed replies, current and finished turns, approval review
and decisions, Stop, safe failures, actual token accounting, and model and approval
selection. It follows the plain design (revision 5): no stripes, glows, badges,
tutorial text or identity rows.

- **Evidence is deterministic and browser-only.** Headless Chrome ran the production
  build with the real compiled bridge (`editor/bridge/native.generated.js`, unchanged)
  on a scripted fake host IPC. These are bridge and UI behaviour results. They are not
  live-provider, native-host or native WebKit results.
- **No native renderer was launched.** No graphics grant or binary was supplied. No
  account was used or changed, and no provider was called.
- **Not claimed:** native acceptance, any phase gate, and any performance figure.
- **Packet revisions:** the current brief has no priority-revision section and no new
  director feedback. The worktree was clean at `830de69`; no interrupted attempt existed.
  The standing direction (restrained, clean, useful, spacious at small widths) is applied
  throughout.

## Model and effort

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`), as reported by this session's runtime.
- **Thinking effort:** max, as configured for this session.
- **Verification limit:** this session cannot query ACP itself, and no ACP verification
  record existed in this worktree (`artifacts/` was empty at the start). These values are
  the session's own configuration, not an independent ACP check.

## Changed paths

| Path | What |
| --- | --- |
| `editor/ui/src/components/AgentPanel.tsx` | Container. Phase, header status, requests (send/cancel/approve/retry), announcements, the cannot-run note, focus recovery |
| `editor/ui/src/components/agent/AgentTranscript.tsx` | Conversation log. Follows the live edge, Latest jump, overflow keyboard stop, bound note |
| `editor/ui/src/components/agent/AgentTurn.tsx` | One turn: prompt, reply, decided rows, approval card, outcome |
| `editor/ui/src/components/agent/AgentComposer.tsx` | Message field, model field, approval mode, the single Send/Stop/wait action |
| `editor/ui/src/components/agent/agentPhase.ts` | Idle / starting / running / approval / stopping / finishing / unavailable, from the snapshot plus in-flight requests |
| `editor/ui/src/components/agent/agentText.ts` | Pure wording: failures, tokens, approval modes, reviews built from the exact arguments, escapes for hidden characters |
| `editor/ui/src/components/agent/agentMemory.ts` | Per-bridge store for the draft, settings and in-flight requests (`useSyncExternalStore`) |
| `editor/ui/src/styles/agent.css` | Agent pane styles (already imported by `main.tsx`) |
| `editor/ui/src/components/agent/agentText.test.ts` | 31 tests: wording, accounting, reviews, phases |
| `editor/ui/src/components/agent/AgentSession.test.tsx` | 33 tests: panel behaviour on a test double, plus one on the real `NativeBridge` with mocked IPC |
| `editor/ui/src/App.test.tsx` | One existing test rewritten (see "Shared file touched") |
| `handoffs/0045-native-agent-chat/tools/` | `fake-host.js` (scripted IPC), `scenarios.mjs` (invented data), `capture.mjs` (captures and flows) |
| `handoffs/0045-native-agent-chat/screenshots/` | 59 captures, 3 flow images, `report.json` and one recapture report |
| `handoffs/0045-native-agent-chat/followup-*.patch` | Exact follow-up patches, **not applied** |

Not touched: `App.tsx`, `main.tsx`, `ShellContext`, `Icon`, `base.css`, `ViewportPanel`,
`InspectorPanel`, Play files, `editor/bridge/*`, `DESIGN.md`, engine, compiler, CI and
dependencies.

## Design

- **Header** (36 px, visible when collapsed): Ready, Starting…, Working…, Needs
  approval, Stopping…, Finishing…, plus the existing sign-in and unavailable states.
  Work in progress shows the shared small spinner in place of the dot. "Needs approval"
  is a steady amber dot.
- **Turns**
  - Divided by hairlines. The prompt is the one filled block, so a long conversation
    scans by request. Prompts over 8 lines clamp, with Show all.
  - The reply is the provider text exactly as streamed: plain text, never markup.
  - A running turn ends with a wordless spinner; the words live once, in the header.
  - Outcome line:
    - a failure sentence after the error icon;
    - Stopped, for a cancelled turn;
    - edits, saying "No edits" only when the report says so;
    - the allowance charged: "(estimated)" when unconfirmed, and "Token use unknown"
      instead of "0 tokens";
    - Show in History, which opens and focuses the History tab.
  - The session's allowance used is the conversation's last line, in amber at 80 % and
    red when used up.
- **Approval card**
  - The request is a question read from the exact arguments: "Apply 4 changes to the
    project?", "Capture a 640 × 360 screenshot from Close-up Camera?".
  - The agent's own description is shown in quotes.
  - Operations are listed like a diff (+ add, − remove, ~ change, ! unreadable), with
    project names resolved. At most 8 are listed; the rest are counted.
  - "Exact arguments" opens a bounded, wrapped JSON well (64 KB until Show all). The
    card shows its size.
  - Deny / Allow sit in a footer. In a short pane the footer sticks to the bottom of the
    view and the view keeps the question at the top.
  - Untrusted names are isolated (`<bdi>`), bounded, and hidden direction or zero-width
    characters are shown as escapes.
- **Composer**
  - A one-line field that grows. The draft stays editable while the agent works.
  - The model is an editable monospace name, default `gpt-6-astra`. No model list is
    invented; clearing it restores the default.
  - The approval mode is a native select under its own label: Ask before edits
    (destructive), Ask every time (always), or Never ask (auto), which is shown in amber.
  - One action button: Send, Stop while a turn can be cancelled, or a spinner while
    Sending…, Stopping… or Finishing…. It is one element, so focus stays on it.
- **Cannot run:** one sentence and its single next step (Continue with ChatGPT, the
  account dialog, or Retry after a transport failure). Turns the session still holds
  remain readable above it, in a bounded scroller.
- **Transport failure while a worker still runs:** one line (the warning icon, the safe
  message and Retry) above the composer. Stop stays usable.

## Interaction, safety and accessibility

- **Requests use the snapshot's explicit IDs.**
  - `agent.send {text, model, approval}`, with text trimmed and settings explicit.
  - `agent.cancel {turnId}`: the last active turn, or the turn a send receipt reserved
    before the session lists it.
  - `agent.approve {turnId, approvalId, approved}` from the pending approval itself.
  - Duplicates are refused synchronously from the store.
- **Nothing is fabricated.** A send adds no turn until the session lists one. The draft
  is cleared only if it still equals what was sent. A failed send keeps it.
- **Never silently approved.** Nothing focuses Allow. The card is keyed by its turn and
  approval IDs, so a following approval is a fresh card and never the previous card's
  focused button. Decisions are blocked, with a reason, while signed out, after a
  transport error, while stopping, or on hosts without `agent.approve`.
- **Focus:**
  - After Allow or Deny, the decided row takes focus.
  - Stop keeps focus through Stopping… and back to Send.
  - If the focused control disappears (note to composer, Retry succeeds), focus stays in
    the pane. It is never pulled back after the user has left.
  - Keyboard focus is kept clear of the sticky decision row (`scroll-padding-bottom`).
- **Announcements:** the conversation is a log with `aria-live="off"`. Approval
  requests, outcomes, background transport failures and Retry results are said once in
  the status line. Streamed text never is.
- **Scrolling:** the view follows the live edge only while the reader is at it.
  Expanding something to read it holds the view. Following resumes after a send or a
  recorded decision. Otherwise "Latest" returns to the end. Browser scroll anchoring is
  off for the conversation, so only the reader or the pane moves it.
- **Survives remounts:** hiding the Inspector unmounts the pane. Draft, settings and
  in-flight requests live in a per-bridge store, so a send that resolves meanwhile still
  lands.

## Commands and results

UI tool binaries come from the parent checkout's `node_modules`, installed from the same
`package-lock.json` at the same commit. They were used read-only, and nothing was
installed.

| Command (in `editor/ui`) | Result |
| --- | --- |
| `vitest run` (baseline at `830de69`) | 31 files, 577 tests passed |
| `tsc -b --noEmit` (strict; includes tests) | passed |
| `vitest run` | **33 files, 641 tests passed**: 64 new, one App test rewritten |
| `tsc -b && vite build` | passed |
| `git diff --check` (repo root) | passed |

`native.agent.test.ts`'s 32 transport tests are part of that run and pass unchanged.

**Browser evidence** comes from `node handoffs/0045-native-agent-chat/tools/capture.mjs release`.
It ran headless Chrome 155.0.8059.40 through playwright-core 1.64.0 at deviceScaleFactor 2,
with reduced motion and the page's real CSP; axe ran through DevTools, not an inline script.

- 59 captures over 24 states, with exact side columns of 280 and 380 px in both
  1000×650 and 1440×900 windows.
- 0 axe violations (colour contrast included), page errors, remote requests, step
  errors, window scroll, elements escaping the pane, horizontal scrollers or leaks of
  the raw injected error text.
- 8 flows with real keyboard and pointer input. All passed with no page errors:
  - **send-approve:** two quick Mod+Enter presses produce one `engine_agent_start`.
    Focus stays in the composer while the approval waits. Allow sends
    `engine_agent_decide {turnId:1, approvalId:1, approved:true}` and focus lands on the
    "Allowed" row. Then "1 edit", the status line says "The agent finished: 1 edit.",
    and Show in History opens and focuses the History tab.
  - **stop:** `engine_agent_cancel {turnId:2}`. Focus stays on the action through
    Stopping… and back to Send. The outcome reads "Stopped · 1,180 tokens (estimated)".
  - **stop-signed-out:** after sign-out the header stays Working… and Stop sends
    `{turnId:5}`.
  - **retry:** a transport failure keeps all 4 turns and shows the safe message, never
    the raw one. Retry recovers, focus moves to the composer, and the status line says
    "Agent session refreshed."
  - **scroll:** at scrollTop 120, three new outputs leave the position unchanged and
    Latest appears. Latest goes to the end and focuses the log; following resumes.
  - **send-failed:** the host refuses the start. Its safe message and Retry appear.
    After Retry the draft is intact.
  - **review-reading:** at 1000×650 and 280 px, the question is in view on arrival.
    The keyboard-focused disclosure sits above the sticky row, and expanding and
    tabbing do not move the view. After Allow the view is back at the end.
  - **deny:** `approved:false`. Focus lands on the "Denied" row, and the turn says
    "Ended after you denied a request."

## Screenshots

All are in `handoffs/0045-native-agent-chat/screenshots/`. Names are
`<state>-w<window width>-s<side column>-side.png`, a 2× crop of the Inspector column; a
few are `-window.png`. Key ones:

- Approval, short pane: `approval-w1000-s280-side.png`, `approval-w1000-s280-window.png`
- Approval, wide: `approval-w1440-s380-window.png`, `approval-w1440-s280-side.png`
- Arguments: `approval-arguments-w1440-s280-side.png`
- Screenshot request in "always" mode: `approval-always-w1000-s280-side.png`
- Decision blocked while signed out: `approval-signed-out-w1000-s280-side.png`
- Decision in flight: `approval-deciding-w1440-s380-side.png`
- Streaming: `streaming-w1440-s380-window.png`, `streaming-w1000-s280-side.png`
- Finished, failed, stopped and denied turns with usage: `history-w1440-s380-side.png`,
  `history-w1000-s280-side.png`
- Stopping and Finishing: `stopping-w1000-s280-side.png`, `finishing-w1000-s280-side.png`
- Signed out with history kept: `signed-out-history-w1440-s380-side.png`
- Transport failure: `transport-error-w1000-s280-side.png`,
  `transport-error-running-w1440-s380-side.png`
- Allowance used up: `exhausted-w1440-s280-side.png`
- Send refused, and model byte limit: `send-failed-w1000-s280-side.png`,
  `model-limit-w1000-s280-side.png`
- Long prompt, reply and 189 KB arguments: `long-w1440-s280-side.png`,
  `long-top-w1440-s280-side.png`
- Reading earlier turns while output arrives: `many-scrolled-w1440-s380-side.png`
- Never ask: `auto-mode-w1440-s280-side.png`
- Collapsed: `collapsed-approval-w1440-s280-side.png`
- Keyboard focus: `focus-model-w1440-s280-side.png`, `approval-arguments-*`
- Flows: `flow-send-approve-window.png`, `flow-stop-signed-out-side.png`,
  `flow-review-reading-side.png`

`model-limit-w1000-s280-side.png` comes from a recapture (`report-model-limit-recapture.json`).
The release-run image had a transient raster glitch over the Inspector *title* while CJK
fallback fonts loaded; that is outside this pane. Two later captures are byte-identical
and clean, and every measurement in both runs was clean.

## Review and corrections

The rendered review in the browser changed the design:

- The default pane is about 142 px of conversation at 1000×650. The approval question
  was scrolled away above its buttons. That led to the live edge at the question and
  the sticky decision row.
- "Working…" was said twice; the in-turn indicator is now wordless.
- The select was sized to its widest option; it is now a label over a transparent
  native select.
- The two-line composer and its placeholder wrapped at 280 px; the composer is now one
  line that grows.
- The follow logic misread its own scroll after a font reflow, and later snapped back
  while the arguments were being read. Both were fixed and verified with numbers.

An independent `code-review` pass (high) found 8 issues, all fixed with regression tests:

- **Card reuse (safety).** Without a key, a following approval could inherit the focused
  Allow. The test was confirmed to fail without the key.
- **Remount mid-send** lost the result. The per-bridge store fixes it.
- **Focus recovery** could pull focus back after the user left the pane.
- **Untrusted text.** `texture_usage` and component names were not escaped or bounded.
- **Model byte limit** was not checked in the UI.
- **Review computed three times;** it is now computed once and shared.
- **Hard-coded bound inference.** The bound is now stated as policy.
- **Wrong fallback phase.** "Finishing…" became the neutral "Working…".

## Shared file touched

`editor/ui/src/App.test.tsx`: one hunk at the old "disables the composer while the agent
works" test. It asserted the old behaviour: the message field was disabled while the
agent worked, which drops keyboard focus right after Mod+Enter. It now asserts the field
stays editable and nothing is sent. Handoff 0044 may also edit this file; the change is
one contiguous test.

## Follow-ups (exact patches, not applied)

1. **`followup-app-resizable-history.patch`** (`App.tsx` and `AgentPanel.tsx`): give the
   pane its separator whenever the session holds turns, not only when the agent is ready.
   Today, history kept above a sign-in or Retry note is capped at `min(40vh, 320px)`
   without a separator. Verified: applied to this tree it passes the strict typecheck
   and the 106 Agent and App tests. It was then reversed, so `App.tsx` is unchanged here.
2. **`followup-design-agent-pane.patch`** (`DESIGN.md`): replaces the stale "Agent pane"
   section with the design above. Checked with `git apply --check`.
3. **For the App or layout owner:** at 1000×650 the agent pane opens at 260 px (the
   current default), about 142 px of conversation. The sticky decision row keeps the
   question and buttons usable, but a taller default while a session is active would
   read better.

## Contract gaps (not invented)

- **No "trimmed" signal.** The pane says "Only the latest 16 requests are kept" at the
  bound instead of claiming that turns were dropped.
- **No model catalog.** The model is free text with the default.
- **No per-step tool activity while running.** Only text and approvals stream.
  `report.tool_calls` arrives at the end and is not shown.
- **Decided approvals keep only tool, bytes and decision.** The row reuses the wording
  this pane showed while pending; after a reload it falls back to the tool name
  ("Project changes").
- **Denial is inferred.** A denial ends the turn as `failure: 'tool'` (from
  `incant_agent` `lib.rs`). The pane says "Ended after you denied a request" only when
  the turn has a denied approval. A dedicated failure code would remove the inference.
- **Not available yet:** dollar cost, configurable budget, transcript persistence, image
  transcript, inline diff and per-turn revert. History is the undo path.

## Limitations

- Chrome only. WKWebView behaviour remains unobserved natively: the select overlay,
  `:has()`, sticky positioning, `scroll-padding` and focus on click (Safari does not
  focus buttons on click).
- The fixture is scripted. Native timing (250 ms polling, slow provider stops) is
  represented only by fixed delays.
- jsdom tests cannot check layout. Scrolling, sticky and focus geometry are covered only
  by the browser flows.

## Open questions

- Is "Never ask" the right wording for `auto`, given that it is amber and its
  description says every edit is undoable in History?
- Should the agent pane default taller while a session is active? This belongs to the
  App and layout owner.
- Could the session report denial and trimming explicitly, removing the two inferences
  above?

No phase gate is approved or claimed.
