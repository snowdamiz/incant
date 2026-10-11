# 0048 combined native editor acceptance: result

## Status

- **Native review of the granted immutable build: done.** Every check the packet lists has a
  result below. All ran in the actual WKWebView of `incant_editor` `3d059445…6603`.
- **The one granted provider turn: done, exactly as bounded.**
  - Allowance 64,000 set in the UI; approvals set to "Ask every time".
  - One prompt: rename entity …0051 to "Allowance Rename Confirmed".
  - Two approvals were requested and allowed: the read-only "Read the whole project", then the
    patch. The patch's exact arguments were verified before Allow.
  - The rename and its History entry were verified, then undone.
  - Usage was 22,044 tokens after the turn, after Undo, and after quit and reopen. The reopened
    process kept the account, with no login and no provider request.
- **Two UI defects found natively, both fixed in UI source** (D1, D2 below). They have
  regression tests, the full UI suite passes, and the fixes need a newly granted binary. **The
  old binary proves nothing about them.**
- **Native processes:** every process this session started has exited (list under "Processes").
  No native window or GPU work of mine remains.
- **No gate claims.** No phase gate, phone, performance, cross-platform or accessibility-audit
  claim.

## Model and effort

- **Exact model:** Claude Opus 5.5, model ID `claude-opus-5-5`.
- **Thinking effort:** Max. The session reports reasoning effort "max".
- **How it was verified:** the session was started by `tools/handoff/main.py run
  0048-native-editor-integration` through `claude-agent-acp`. Its `configure_model_and_effort`
  sets both settings through ACP, and raises before prompting unless ACP confirms each one. I
  cannot see the runner's own confirmation line from inside the session.
- **No substitution, no routing exception, no auxiliary agent.**

## Packet revisions and feedback

- **This packet:** the brief carries the final grant ("Final immutable build and grants"). There
  is no other priority revision and no new director feedback; it was applied as written.
- **Clean start:** the worktree was clean at `62dcfcf`, with no interrupted attempt.
- **Root's packet files:** `brief.md`, `build/` and `fixture/` are committed in root's routing
  commit; I did not change them.

## Build, fixture and window

| Item | Value |
| --- | --- |
| Source of the binary | `bb1b517034386021b49541221c72500316e0ccb2` (includes main100 `39f5e40`) |
| Manifest | `/tmp/incant-native-review-0048-final-bin-v1/manifest.json`, `b2f83ac7…f606e` |
| Editor | `…/incant_editor`, `3d059445c3691b219ff8cc2c6d990645af7f64bbeccee737b4dae4f9753c6603`, hash checked before each launch |
| Headless | `68a2a04b…873c`, verified, never launched |
| Fixture | `/tmp/incant-native-0048-fixtures/saved/project.json`, project `01M4PEJQNPMDQYEDGT0PRWN78X`. All baseline hashes matched before launch. `project.json` `3ff0ded1…` unchanged throughout |
| Journal | before: `4564ee75…`, 8 records. After: `0fd36d3e…`, 23 records |
| Usage record | before the turn: absent. After: `91cecc82…`, `{"charged_tokens":22044,"accounting_uncertain":false,"pending":null}`, unchanged by Undo, reopen and the cap test. Read by exact path; no credential or account store was opened |
| Host | macOS 26.6 (25G72), arm64, built-in display 1512 × 982 pt at 2× |
| Window | the visible desktop is 1512 × 875 pt (menu bar and Dock), so 1440 × 900 opened at **1440 × 875** (1440 × 874 after an AX resize). Minimum **1000 × 650** |
| Sidebars | Inspector column 280 and 380 px at both sizes |

Fixture after the review: cap 64,000, usage 22,044. Pointer Probe and Allowance Rename Target
carry their original names. Every edit went through the editor UI and command bus; no file was
edited by hand.

## Interaction results

Screenshots are actual native window captures in `screenshots/native/` (prefix omitted below).
Numbers come from accessibility reads of the owned window; they are in
`evidence/native-review.json`.

| Check | Result | Evidence |
| --- | --- | --- |
| No project path | **Pass.** Play is unavailable with "Open a saved project to run Play" in its description. Clicking it and ⌘P both put the sentence in the status line. The header stays Live; there is no notice and no session | `unsaved-01-…` |
| Saved fixture | **Pass.** Play is available. With two scenes, Play opens the scene/camera list; choosing Main · Stationary Camera starts Play and focuses Pause | `play-01-…` |
| Transport | **Pass.** Preparing… → Playing · Main → Paused · tick 263 → "Stepped to tick 264." → "Resumed." → "Play stopped. Runtime changes were discarded." → Live | `play-02-…`, `play-03-…` |
| Runtime vs script and tick | **Pass.** Moving mesh X = 2 + 0.25 · tick / 60 at tick 263 (3.0958), 264 (3.1), 82 (2.3417, read while running) and 278 (3.1583). Authored X stays 2 throughout and after Stop; the runtime block goes on Stop | `play-02-…`, `layout-01-…`, `layout-02-…` |
| Game geometry | **Pass.** Both imported cubes render from the Stationary Camera, and the moving one advances in +X. Stop returns the authoring viewport | `play-01-…`, `play-03-…` |
| Viewport hole | **Pass.** The native surface fills the game view exactly: x 246 pt to the right separator, y 76 pt to the dock separator, at 1440 and at 1000. At 1000 the focused separator's ring overlaps the surface edge by 1 pt | `native-review.json` |
| Titlebar | **Pass.** Traffic-light and logo centres share y 19.75 pt at both sizes. The logo starts 11 pt after the zoom button; hairline at 39 pt | `layout-01-…`, `layout-03-…` |
| Game focus after Resume (0044 fix 1) | **Pass.** Key Probe scale X is 2 only with the game view focused (65 D-ticks in 1 s). Resume with focus elsewhere gave scale X 1 with counters unchanged: Agent region 0.1 s, Hierarchy row 0.3 s, filter field 0.5 s, composer 0.5 s with D held (the D went to the composer). The pointer moved over the view during a Resume did not reach the game: Pointer Probe stayed (414, 224) | `play-04-…`, `play-05-…` |
| Dismiss (0044 fix 2) | **Keyboard: pass.** Tab goes picker → Play → Step → Stop → Dismiss; Space moves focus to Play. **Pointer: the app moves no focus.** WebKit's own press focuses the clicked button's focusable ancestor, the Viewport region, so a previously focused Hierarchy row loses focus, as with any click in that panel. Focus never drops to the page | `play-07-…` |
| Separators | **Pass, both orientations.** Inspector, dock, hierarchy and agent separators, dragged across text past their clamps (280, 120, 200, 560): no selection range at all, the separator focused with no ring. Arrows ±16 then show the ring; Home and End reach 280 and 560. Right-click and Control-click followed by moves do not resize. A separator press closes an open Hierarchy menu. A text drag still selects (83 characters) | `splitter-01…04` |
| Allowance, no provider | **Pass.** Each case behaved as follows: <br>• "12.5" and "abc": "Enter a whole number of tokens."; "-5": "The allowance can’t be negative." <br>• Escape sends nothing, and focus returns to Change. <br>• 500,000, then 100, then 0: Off, "Raise the allowance to turn the agent on.", no Send. <br>• Titlebar Undo/Redo/Undo give 100/0/100; then 64,000. <br>• Each change is a `set_agent_budget` user transaction in the journal, with undo/redo records. History says "Set agent token allowance". The editor had no network socket | `allowance-01…02` |
| Usage below a lowered cap | **Pass** (reopened process). 10,000 previews "No new requests: 22,044 already used." Saving gives Allowance used up, "Raise the allowance to send new requests.", and a red "22,044 of 10,000 tokens used", with no Send. Undo restores 64,000; the usage record is unchanged | `allowance-03-…` |
| The real turn | **Pass:** <br>• Sent once at 23:01:12Z; Working… with Stop available throughout (not pressed). <br>• 23:01:18Z: approval "Read the whole project?" (`{"path":""}`), allowed. <br>• 23:02:08Z: "Apply 1 change to the project?". Its exact arguments were `rename_entity` …0051 in Main → "Allowance Rename Confirmed", `expected_revision` 14 (current). Allowed at 23:04:02Z. <br>• 23:04:08Z: Ready, the reply in plain text, and "1 edit · 22,044 tokens". The line read "0 of 64,000" while working and "22,044 of 64,000" after; no invented numbers. <br>• History: "Rename Allowance Rename Target to Allowance Rename Confirmed · Agent · gpt-6-astra". Show in History focused the tab. Undo restored the name and marked the entry "(undone)". <br>• No unrelated proposal occurred | `turn-01…04`, `evidence/turn/` |
| 0049 approval readiness | **Pass; the hold itself was unobserved.** At the read-only approval I pressed titlebar Undo, then Redo, of "Pointer Probe Review" (never the allowance), and Allow about 0.1–0.2 s after Redo. Result: exactly one decision, no account or refusal text anywhere, and the following patch was current. AX polls every 0.2–0.4 s never showed the hold sentence or a held Allow, so any hold was shorter than a poll. **A stale patch was not exercised** | `evidence/turn/hold-*.jsonl` |
| Quit and reopen | **Pass.** ⌘Q, then the same binary on the same path. The account button is still signed in (name redacted), with no login. Agent Ready, "22,044 of 64,000 tokens used", History 9. lsof sampled every 0.5 s for 15 s: no socket | `reopen-01-…`, `evidence/turn/reopen-startup-sockets.txt` |
| Layout and wrapping | **Pass** at 1440 × 875 and 1000 × 650, with 280 and 380 px columns. The notice wraps to 3 lines at 1000/380 and 2 at 1000/280. The picker label truncates, with its full name in the accessible name. Hierarchy names ellipsize. The Inspector stacks labels at 280 | `layout-01…04` |

## Defects found natively, fixed in UI source

Both need a newly granted binary before native acceptance.

| | D1: focus lost when ⌘P starts Play from the picker | D2: model name always clipped in WebKit |
| --- | --- | --- |
| Seen | The scene/camera picker unmounts during a session. With it focused, ⌘P left focus on the page (AX focus: the web area). That is a normal keyboard flow: choose a target, focus returns to the picker, then ⌘P | The composer showed "gpt-6-astr…" at every width (`defect-model-name-clipped-crop.png`). Chrome showed the full name |
| Cause | The focused element was removed | The width was `calc(Nch + 10px)`, which is exactly text plus padding. An offscreen system-WebKit probe measured clientWidth 89 against scrollWidth 91, and overflow for every length 4–32. +12 px fits all of them (`evidence/webkit-model-field-probe.json`) |
| Fix | `PlayControls.tsx`: while the picker unmounts with focus, focus moves to Play. DESIGN.md says so | `AgentComposer.tsx`: `calc(Nch + 12px)` |
| Tests | Two new Play tests. One fails on the reviewed source (focus on the page). The other pins that focus elsewhere stays. An over-broad variant is masked by React's own focus restore (`evidence/mutations.txt`) | An assertion in the existing composer test fails on the reviewed formula. Layout itself is measured only by the WebKit probe; jsdom has no layout, and Chromium does not show it |

## Observations, not changed

1. **Unattributed change between two bursts (22:34:43–22:35:59Z).** The Agent pane shrank
   289 → 243 px, which only a pointer drag of its separator can do, and the Agent region had
   focus. None of my input touched it. The 10 later start-of-burst focus snapshots all matched
   the previous burst's end. Likely a person using the window.
2. **The message composer applies macOS text substitution:** smart quotes changed the typed
   prompt's quotes, and an autocorrect bubble appeared (`play-05-…`). The model field disables
   spellcheck. Harmless here.
3. **In the default 288 px agent pane, a patch card's "Exact arguments" sits under the sticky
   Deny/Allow row.** My pointer click hit the row. Shift+Tab reaches it and scrolls it into view
   (`turn-02-…`). This is the open "taller pane while a session is active" question from
   0045/0046.
4. **History follows the page that holds Now, not its scroll position.** In a 236 px dock the
   agent's entry and Now are below the fold after Show in History and after Undo (`turn-04-…`).
   Follow-up suggestion: scroll Now (or the turn's transaction) into view. This is dock code
   outside this packet.
5. **For Astra (host wording):** "Open a saved project to run Play" has no final period. The
   no-camera refusal still uses CLI wording ("--camera"), as noted in 0044.

## My review's own slips, recorded

- **Dropped typing.** The helper's synthetic Unicode typing dropped characters three times:
  "abc" after ⌘A, the first rename attempt, and "10000" after the reopen. Each was caught by an
  AX read.
  - **Consequence 1:** one Return saved an unchanged value. It sent nothing.
  - **Consequence 2:** my following titlebar Undo undid my own "Pointer Probe Review" rename,
    not the allowance or usage.
  - **Fix:** later entries used key codes, and the value was verified before Return.
- **The first send attempt stopped before sending.** Smart quotes failed my exact pre-send
  comparison. The quote-normalized check passed and the request was sent once.
- **The first relaunch did not start.** `launch.sh`'s "already running" guard matched my own
  shell command. No process was started.
- **The front window during the turn.** During the turn (23:00–23:05Z) the editor kept the
  front between my steps instead of handing it back after each burst.

## UI checks after the fixes

Run from `editor/ui` with the parent checkout's pinned tools; nothing was installed.

| Command | Result |
| --- | --- |
| `tsc -b --noEmit --verbose` (build info removed first; app, node and test projects) | exit 0 (`evidence/typecheck.txt`) |
| `vitest run --maxWorkers=4` | **44 files, 904 tests passed**: root's 902 plus 2 new (`evidence/vitest-full.txt`) |
| `tsc -b && vite build` | exit 0 (`evidence/build.txt`) |
| `node tools/build_bridge.mjs --check` (repo root) | exit 0; bridge unchanged |
| `git diff --check` | exit 0 |
| Mutation checks | `evidence/mutations.txt` |
| WebKit probe (`tools/webkit-probe/`) | `evidence/webkit-model-field-probe.json` |

Raw-log hashes are in `evidence/raw-log-hashes.txt`. Paths in the logs are redacted to
`<worktree>`, `<checkout>` or `~`.

## Changed paths

- **Edited:**
  - `editor/ui/src/components/play/PlayControls.tsx` and `Play.test.tsx`;
  - `editor/ui/src/components/agent/AgentComposer.tsx` and `AgentSession.test.tsx`;
  - `editor/ui/DESIGN.md` (one sentence).
- **New in `handoffs/0048-native-editor-integration/`:**
  - `result.md`;
  - `tools/native/`: `winctl.swift`, which is 0044's tool plus AX reads, exact matching,
    typing and account redaction; `launch.sh`, `burst.sh`, `cap.sh`, `watch_turn.py`,
    `poll_hold.py`, `verify_patch.py`;
  - `tools/webkit-probe/`;
  - `evidence/`;
  - `screenshots/native/`: 25 PNGs and a README.
- **Not edited:** bridge, Rust, styles, the fixture files, root's packet files, and every path
  outside this worktree.
- **Ignored and local:** `artifacts/`, which holds the raw captures, AX dumps and logs (about
  37 MB); `editor/ui/dist`; `editor/ui/node_modules/.tmp`.

## Processes

| PID | Launch | Ended |
| --- | --- | --- |
| 77528 | no project path, 22:21:49Z | ⌘Q, before 22:25Z |
| 80574 | fixture, 22:25:08Z | ⌘Q 23:05:05Z |
| 39232 | fixture reopen, 23:06:42Z | ⌘Q 23:10:04Z |

Short-lived helpers (`winctl`, `watch_turn.py`, `poll_hold.py`, `webkit-probe`) all exited.
`pgrep` finds no `incant_editor`, `incant_headless` or helper process. Nothing outside the
owned window was captured, and no account dialog was opened.

## Limitations

- **Native coverage:** macOS on this display only. No Windows, Linux, VoiceOver, IME, touch,
  other DPRs or phones.
- **Not exercised natively:**
  - a real `pointercancel` or lost capture;
  - provider and account errors;
  - uncertain accounting;
  - a stale patch.

  No error occurred, `accounting_uncertain` stayed false, and the brief forbade sign-out and
  extra turns. These rely on the 0045–0049 browser and IPC tests.
- **Network evidence is sampled.** It comes from lsof every 0.5 s and could miss a very short
  connection.
- **Not native evidence:**
  - D1 and D2 are verified by jsdom tests and the offscreen system-WebKit probe only.
  - The WebKit probe reproduces the field's computed styles, not the whole app.

## Open questions

1. **For root:** grant a binary built from `10ee9e5` (the UI fixes; later commits change only
   this file), so D1 and D2 can be checked natively.
2. **For root or the layout owner:** observations 3 and 4.
3. **For Astra:** observation 5.
4. **For the director:** none.
