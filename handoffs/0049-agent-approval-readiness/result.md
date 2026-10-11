# 0049 active approval readiness: result

## Status

- **Fixed in source, tested on the real NativeBridge with deferred mocked IPC, and reviewed
  in a browser. Not accepted natively.**
  - **The defect:** while Undo or Redo waited for `engine_history`, and then for the fresh
    session read, Allow and Deny stayed usable whenever both cached allowances matched. A
    press reached the native guard, which refused it, and the card showed an unrelated
    account error: "Connect an OpenAI account and refresh the native session before
    deciding an approval." In the browser, that stale error stayed under the card even
    after the host was ready again.
  - **Now:** while the host publishes `running.approvalReady === false`, the pane holds
    both decisions in the card's existing footer line: "Waiting for the agent to update
    its session. You can still stop the agent." The conversation, draft, header status
    ("Needs approval") and Stop stay as they were. Once the host has verified the session,
    the decisions come back, and the real decision is sent.
  - **Older hosts:** a host that omits the field behaves exactly as before.
  - **Other reasons:** the existing, more precise reasons still take precedence. They are:
    the capability is missing; the agent is stopping; the user is signed out; a transport
    error; or the pane's own allowance save, or a session allowance lagging the project's.
  - **The native guard stays authoritative:** the pane adds a hold only on the explicit
    `false`, and never infers readiness of its own.
- **Root review:** the root's independent source review
  (`review/root-consumer-review.json`) records `bounded_source_acceptance`, with no
  actionable findings, for exactly the committed source (hashes under "Review").
- **Gates and claims:** no phase gate is approved or claimed. Integration and the
  immutable native 0048 acceptance are the root's. Nothing here is a native, device,
  account, provider or performance claim.

## Model and effort

- **Exact model:** Claude Opus 5.5, model ID `claude-opus-5-5`.
- **Thinking effort:** Max. The session reports reasoning effort "max".
- **How it was verified:**
  - This session's prompt is the handoff runner's prompt.
  - The runner's `configure_model_and_effort` (`tools/handoff/main.py`) sets both settings
    through ACP, and raises before prompting unless ACP confirms each one.
  - I cannot see the runner's own confirmation output from inside the session.
- **No substitution, and no routing exception.**

## Packet revisions and feedback

- **First session:** the packet had no priority revision and no new director feedback. I
  applied it as written, starting from a clean worktree at `0ca2bcd`.
- **This session: "Priority finalization revision, root review complete".** Acknowledged
  and applied:
  - **Finalization only:** no implementation, styling or test-contract change was made.
  - **No reruns:** none of the passed checks, captures or reviews were repeated. The one
    new check is the commit-time whitespace check of the documentation finished here (see
    "Commands and results").
  - **Logs preserved:** the final check logs were copied unchanged, except for path
    redaction, from the previous session's run into `evidence/`. The placeholders below
    now hold their observed results.
  - **Reviews recorded:** the root's source review and my own visual review are under
    "Review".
  - **Auxiliary review:** not waited for and not counted.
    - **What it was:** a read-only review subagent started in the previous session.
    - **Settings:** its model and effort were never confirmed through ACP.
    - **Outcome:** it reported nothing before that session ended. It is no longer
      running: this session lists no subagents.
  - **Processes:** none of mine remain. Both capture runs closed their own browser and
    preview server. The headless Chromium processes now running belong to another
    session's video render in another project, started after my captures ended, and were
    left alone.
- **Interrupted attempt:** the files from the previous session were inspected and
  finished here. Their source hashes match the root's review exactly.
- **Follow-ups:** the 0046 follow-up patches and the Latest correction were already
  integrated, and were not reapplied.

## Source commits

- **Base:** `0ca2bcd6e2b900f02e5f0cc111653cd99b88342e` (branch
  `handoff/0049-agent-approval-readiness`). It contains:
  - Astra's `0e81494503b527c869cdf7685eeedff6ddd45391`, `approvalReady`;
  - `9c50c7185a832b2f4bb2aec8dfe4fa3fa63a5735`, the Latest correction.
- **Bridge sources:** byte-identical to the hashes in
  `review/bridge-correction-review.json`. I tested against exactly the reviewed bridge:
  - `editor/bridge/native.ts` `fa299952…`
  - `editor/bridge/contract.ts` `ddacc748…`
  - `editor/bridge/native.generated.js` `fd67d6db…`
- **Base `AgentPanel.tsx`:** SHA-256 `b2afbc85…`, the file named in
  `review/original-review.json`.

## Change

**`editor/ui/src/components/AgentPanel.tsx`.** One condition, and one last branch in the
existing decision hold:

```ts
const approvalHeld = agent?.status === 'running' && agent.approvalReady === false;
// … existing reasons, unchanged and first …
: approvalHeld
  ? 'Waiting for the agent to update its session. You can still stop the agent.'
  : null;
```

- **Precedence.** The hold is checked last, so every existing reason keeps its precise
  words. A sign-out, a transport error, the pane's own allowance save and a lagging
  session allowance also make the native bridge publish `false`. The new sentence shows
  only for an otherwise connected active worker whose host is refreshing. That means one
  of:
  - the Undo/Redo barrier;
  - the fresh session read after it;
  - an account read in flight;
  - a project revision ahead of the cached read.
- **No other component changes.** `ApprovalCard` (`agent/AgentTurn.tsx`) already turns any
  non-null hold into three things:
  - `aria-disabled` on both buttons, which stay focusable;
  - the reason in the footer note, which is part of the card's accessible description;
  - a `choose()` that returns without deciding. Pointer presses, Enter and Space all
    honour it.
- **Wording.** It follows the sibling "Waiting for the agent to apply the new allowance.
  You can still stop the agent." and the composer's "The agent is updating its session."
  It names no cause the pane cannot see.
- **Header.** It stays "Needs approval": the worker still needs approval and is not
  unavailable.
- **Not changed:** styles, layout, contract, bridge and existing tests.

**`editor/ui/DESIGN.md`.** In the Agent pane's "Updating." item, decisions now also wait
"while … the host says it is not ready for them". The item quotes the new sentence.

## Tests

**New file:** `editor/ui/src/components/agent/AgentApprovalReadiness.test.tsx`, with 12
tests.

**On the real `NativeBridge`, over mocked IPC** whose `engine_history` and
`engine_agent_read` replies are held until the test releases them (6 tests):

1. **Undo and 2. Redo** (`it.each`), each driven from its titlebar button. The worker is
   awaiting approval and both cached allowances match.
   - **While `engine_history` is held:**
     - the bridge publishes `approvalReady: false`;
     - Allow and Deny are `aria-disabled` and show the new reason;
     - Stop stays usable;
     - the reply and the draft stay visible.
   - **Presses:** both decisions are pressed while focused. No `engine_agent_decide` IPC
     is sent, and no refusal is shown.
   - **Then:** the history reply lands, and the fresh session read is held. The same hold
     and checks apply.
   - **Then:** the read answers. The decisions return, and Allow sends exactly one
     `engine_agent_decide {turnId: 3, approvalId: 7, approved: true}`. The decided row
     appears, the header says Working…, and the draft is kept.
3. **Stop during the hold:** the pane says "The agent is stopping." and sends
   `engine_agent_cancel {turnId: 3}`. It never sends a decision.
4. **The precise reason after the pane's own allowance save:** the allowance reason, not
   the new one. `approvalReady` is false here too.
5. **After a sign-out:** the sign-in reason.
6. **After a failed session read:** the retry reason, with the transport notice above the
   composer.

In tests 4 to 6, both decisions are pressed: nothing is sent, no refusal appears, and
Stop still sends a cancel.

**Against test doubles** (6 tests):

7. An explicit `false` with matching allowances holds both decisions and keeps Stop. When
   the host later says `true`, the decisions return and Allow sends `agent.approve`.
8. **Older host:** the field is omitted, including after a later session update. The
   decisions stay usable, and Deny sends.
9. to 11. With an explicit `false`, the precise reason still wins: signed out; transport
   failure; session allowance lagging the project's.
12. With an explicit `false`, a host without `agent.approve` still says so.

**jsdom limit:** jsdom does not turn Enter or Space on a button into a click. The tests
click the focused button. The browser review presses the real keys.

**Before the fix.**
- The new file on the base-commit `AgentPanel.tsx`: 4 fail, 8 pass
  (`evidence/vitest-before-fix.txt`).
  - **The 4 that fail:** tests 1, 2, 3 and 7. On the base component, the decisions are
    not held.
  - **The 8 that pass:** they pin behaviour that must not change, namely precedence and
    older hosts.
- Astra's observational test (`review/approval-history.review.test.tsx`), copied into
  `src/test/` for the run and then removed:
  - **before the fix:** passes, so it still observes the bug
    (`evidence/review-observation-before-fix.txt`);
  - **after the fix:** fails at its "Actual observed bug" assertion, line 55: Allow's
    `aria-disabled` is `'true'` (`evidence/review-observation-after-fix.txt`).

**Mutation checks** (`tools/mutations.mjs`, `evidence/mutations.json`). Each mutation
changes one part of the final source and runs the 12 tests. Each file is restored and
compared byte for byte; SHA-256 checks afterwards confirmed both files. Every mutation is
caught:

| Mutation | Tests that fail |
| --- | --- |
| No hold (the base chain) | 4 |
| Hold checked first, before every other reason | 8 |
| Hold checked before the allowance reason | 2 |
| An omitted field treated as not ready | 1 (the older-host test) |
| `choose()` ignores the hold; buttons still marked | 9 |

## Commands and results

Commands were run from the worktree root unless noted.
- **Dependencies:** the parent checkout's pinned `node_modules` are reached by normal
  resolution, since this worktree is nested inside that checkout. No symlink was made, and
  nothing was installed or upgraded. Both checkouts' `package-lock.json` SHA-1 is
  `e33abebd…cc944`.
- **Caches:** TypeScript build info and Vite caches went to this worktree's ignored
  `editor/ui/node_modules/`.

| Command | Result |
| --- | --- |
| `cd editor/ui && ../../../../node_modules/.bin/vitest run --maxWorkers=4` (baseline, clean `0ca2bcd`) | 43 files, 890 tests passed (`evidence/vitest-baseline.txt`) |
| `cd editor/ui && ../../../../node_modules/.bin/vitest run --maxWorkers=4 --reporter=verbose src/components/agent/AgentApprovalReadiness.test.tsx` on the base component | 4 failed, 8 passed (`evidence/vitest-before-fix.txt`) |
| same, final component | 12 passed (`evidence/vitest-after-fix.txt`) |
| review observation, before and after the fix | passed, then failed at line 55 (see above) |
| `node handoffs/0049-agent-approval-readiness/tools/mutations.mjs` | 5 of 5 mutations caught |
| `cd editor/ui && ../../../../node_modules/.bin/vitest run --maxWorkers=4` (final) | 44 files, 902 tests passed, exit 0 (`evidence/vitest-full.txt`). That is the baseline's 43 files and 890 tests, plus the new file's 12. |
| `cd editor/ui && ../../../../node_modules/.bin/tsc -b --noEmit --verbose` (build info removed first, so app, node and test projects are all fully checked) | exit 0, no errors; all three projects were rebuilt (`evidence/typecheck.txt`) |
| `cd editor/ui && ../../../../node_modules/.bin/tsc -b && ../../../../node_modules/.bin/vite build` | exit 0 (`evidence/build.txt`) |
| `node tools/build_bridge.mjs --check` | exit 0: the generated bridge is current, and was not edited (`evidence/bridge-check.txt`) |
| `git diff --check`, with the new files marked intent-to-add | exit 0 (`evidence/diff-check.txt`) |
| `node handoffs/0049-agent-approval-readiness/tools/capture.mjs after --root=…/artifacts/dist-after`, and the same for `before` | 8 captures and 2 flows each, no step errors (`evidence/capture-{after,before}.txt`) |
| `git diff --cached --check` over the staged commit, at commit time, in this session | first run: exit 2, flagging `evidence/typecheck.txt`. tsc's verbose output had CRLF endings on 4 lines and one trailing space. Only those whitespace bytes were removed from the committed copy; the raw log's SHA-256 is `6be5830a3713cdfb70da9daad402feb7e83d55e484c8e7c0ba99b9c0738a161e`. Rerun on the final staged commit: exit 0. |

**Final checks.** The five final checks (full suite, types, build, bridge, whitespace)
ran in the previous session, after the last source edit. That edit removed an unused
helper parameter from the test file, saved at 18:03:06. The typecheck started at 18:03:09;
the build, bridge and whitespace checks finished at 18:03:27; the full suite finished at
18:04:06. No source changed afterwards, so, as the finalization revision asks, they were
not rerun. The commit-time whitespace check covers this session's documentation edits,
which the earlier run could not see.

In the saved logs, this worktree's absolute path is replaced with `<worktree>`, and the
parent checkout's with `<checkout>`.

## Browser review

**This is browser evidence of fixture data, not native evidence.**
- **Browser:** headless Chromium 156.0.8078.4, from the Playwright cache that matches the
  pinned playwright-core 1.64.0.
- **Input:** real pointer and keyboard.
- **Server:** `vite preview` on a private 127.0.0.1 port.
- **Not used:** the network, native windows, the GPU, accounts or providers.

**Harness, a small extension of 0046's.**
- 0046's `tools/fake-host.js` and its `approval` scenario, unchanged;
- this handoff's `tools/readiness-host.js`, which holds `engine_history` and session
  reads, and records `engine_agent_decide`;
- the real `native.generated.js`, unchanged, relabelled as a fixture.

**Builds.** Two builds in ignored `artifacts/`:
- **before:** the base `AgentPanel.tsx` put back for the build, then restored and checked
  byte for byte;
- **after:** the final component.

The bundle text was checked: only "after" contains the new sentence.

**Which images I viewed.** All 11 fixed-build images, including pending and recovered at
both 280 and 380 px. Of the 11 base-build images I viewed 3: `pending-w1440-s380`,
`pressed-w1440-s280` and `recovered-w1440-s380`. The other 8 base-build frames are
described here from their measured report data, not from my viewing.

**Results on the fixed build, at both 280 and 380 px.**
- **Pending:** Undo is held. The card's footer shows the reason in the quiet note colour.
  Deny and Allow are both quiet: Allow loses its accent fill. Stop keeps the accent, the
  draft stays, and the header says Needs approval.
- **Presses while held:** pointer presses do nothing, and no refusal appears.
- **Reading:** the history reply has landed and the session read is held. The hold looks
  the same.
- **Recovered:** Allow takes its accent fill back, and the note goes away.
- **Keyboard flows:**
  - **Undo, 380:** Shift+Tab from the composer reaches Allow, with the 2 px focus ring.
    Enter and Space while `engine_history` is held send nothing; so does Enter while the
    read is held. Focus stays on Allow through recovery. Enter then sends exactly one
    `engine_agent_decide {turnId: 3, approvalId: 4, approved: true}`, and focus moves to
    the "Allowed" row.
  - **Redo, 280:** the same hold appears. Enter on Allow sends nothing. Enter on Stop
    sends `engine_agent_cancel {turnId: 3}`.
- **Clean:** axe found 0 violations in the Agent region (pending and recovered). There
  were no page errors, no external requests, nothing drawn past the pane, and no
  horizontal scrollers.

**Results on the base build.**
- During the barrier, Allow keeps its accent fill and is usable.
- A press, by pointer or by Enter, shows the red account error, and the bridge sends no
  IPC.
- That error stays after recovery.

**Visual note.** The 2-line note makes the sticky footer 83 CSS px tall instead of 45. At
the default pane height, that leaves less of the review visible above it. The review
still scrolls, and this is the same footer and growth as the existing allowance hold. No
layout was invented or changed.

## Screenshots

All are browser captures, described in `screenshots/browser/README.md`. Each folder also
holds its `report.json`.
- `screenshots/browser/after/`, the fixed build:
  - `{pending,pressed,reading,recovered}-w1440-s{280,380}-side.png`;
  - `flow-keyboard-held.png`, `flow-keyboard-decided.png` and `flow-redo-held.png`.
- `screenshots/browser/before/`: the same names, from the base build.

No native screenshots were taken, and none are claimed.

## Native acceptance (root, part of 0048)

I did not launch the native editor. Natively, the hold may last only milliseconds, so the
main native checks are that nothing regresses and that the error is gone:
1. **Undo and Redo while awaiting approval:** with an authored project allowance and a
   turn awaiting approval, press Undo or Redo (titlebar, ⌘Z or ⇧⌘Z), then Allow at once.
   - Allow either waits with "Waiting for the agent to update its session…" or works.
   - The account error never appears.
   - Once the pane is ready, Allow or Deny reaches the agent.
2. **Stop during the hold:** Stop works right after Undo.
3. **Precise reasons:** sign-out and allowance edits still show their own reasons.

## Limitations

- **No native or WebKit run.** jsdom and headless Chromium only. Playwright WebKit is not
  installed here, and I did not download it.
- **Simplified Undo/Redo fixture.** In the browser, Undo/Redo changes only History counts
  and the revision; the scene stays the same. The pending approval is left alone, as the
  native host leaves it when the allowance does not change: `check_active` in
  `editor/app/src/agent/worker.rs` compares only the project ID and the cap.
- **Not exercised:** whether the agent's patch later fails because its
  `expected_revision` is stale after an Undo. That is engine behaviour.
- **No announcements.** Nothing is announced when the hold starts or ends, and pressing a
  held button adds no message. This matches the existing holds. The reason is on screen,
  and it is in the card's accessible description.
- **No auxiliary review.** None is counted (see "Review"). The reviews relied on are the
  root's source review and my own visual review.
- **Partial view of the base build.** The base-build images are comparison evidence, and
  I viewed 3 of its 11 frames myself. The rest are described from measurements.
- **Existing wording, unchanged.** While the account is still being checked or signed in
  (`checking` or `connecting`), the existing chain says "Sign in again to decide." That is
  older wording outside this packet, which asked for the sign-out reasons to be kept. I
  observed it in the source and left it unchanged.

## Changed paths

**Edited:**
- `editor/ui/src/components/AgentPanel.tsx`;
- `editor/ui/DESIGN.md`, one sentence.

**New:**
- `editor/ui/src/components/agent/AgentApprovalReadiness.test.tsx`;
- `handoffs/0049-agent-approval-readiness/result.md`;
- `handoffs/0049-agent-approval-readiness/tools/capture.mjs`, `readiness-host.js` and
  `mutations.mjs`;
- `handoffs/0049-agent-approval-readiness/evidence/`: test, typecheck, build, bridge,
  diff, capture and mutation logs;
- `handoffs/0049-agent-approval-readiness/screenshots/browser/`: 22 PNGs, 2 reports and a
  README.

**Not edited:**
- the bridge, contract, generated bridge, styles and existing tests;
- 0046's tools;
- the review receipts;
- every path outside this worktree.

**Written by the root, left uncommitted and unchanged.** The root wrote two files in this
worktree:
- the finalization revision of `handoffs/0049-agent-approval-readiness/brief.md`;
- `handoffs/0049-agent-approval-readiness/review/root-consumer-review.json`.

They stay on disk for the root to commit under its own trailer. Earlier Claude commits do
not carry packet files: Astra's routing commits do.

**Local, ignored and not committed:**
- `editor/ui/node_modules/` (build info and caches);
- `editor/ui/dist/`;
- `handoffs/0049-agent-approval-readiness/artifacts/`: both review builds and the raw
  capture sets.

## Review

**Root source review.** `review/root-consumer-review.json`, by Astra root, recorded
2026-10-11T22:06:35Z.
- **Status:** `bounded_source_acceptance`, with no actionable findings.
- **Scope:** the native readiness P2 source correction. It claims no WKWebView or provider
  acceptance. It does not rely on the auxiliary review. The root will run the final
  combined UI suite, types and build after integration.
- **What it read:**
  - the `AgentPanel` consumer and its memoized dependencies;
  - the guarded `ApprovalCard.choose` path;
  - the native admission predicate;
  - all 12 regressions;
  - the before and after browser report data.
- **Pixel review:** it leaves that to this session.
- **Hashes.** It names SHA-256 hashes for six files. I compared each with the files in this
  commit, and all six match byte for byte:

| File | SHA-256 |
| --- | --- |
| `AgentPanel.tsx` | `513e583b…` |
| `AgentApprovalReadiness.test.tsx` | `a485650b…`, the final file, after the helper cleanup |
| `AgentTurn.tsx` | `11dea252…`, unchanged |
| `native.ts` | `fa299952…` |
| `contract.ts` | `ddacc748…` |
| `native.generated.js` | `fd67d6db…` |

**My visual review.** This session ran on Opus 5.5 at Max, as verified above. I viewed the
images listed under "Browser review" and compared each with its report data.
- **The hold:**
  - the reason sits in the card's existing footer note, in the quiet note colour, above
    the buttons;
  - Deny and Allow are both quiet, and Allow drops its accent fill;
  - Stop keeps the one accent;
  - the conversation, draft and "Needs approval" header stay.
- **No layout problems:** nothing is clipped, overlaps or overflows at 280 or 380 px, and
  no new layout was added.
- **Focus:** keyboard focus on a held Allow shows the standard 2 px ring, with the reason
  directly above it.
- **Recovery:** the note is removed and Allow's fill comes back, at both widths.
- **The base build shows the defect:** the accent Allow is usable during the barrier, a
  press shows the red account error, and that error stays after recovery.
- **Finding:** I found no visual defect in the fixed build.

**Auxiliary review.** I started a read-only review subagent in the previous session. Its
model and effort were never verified through ACP, and it reported nothing before that
session ended. It is not counted as a review, and no finding is attributed to it.

## Open questions

1. **For the root:**
   - integrate, and commit your own `brief.md` revision and review receipt;
   - run the final combined UI suite, types and build;
   - include the native checks above in the 0048 acceptance.
2. **For the director:** none.
