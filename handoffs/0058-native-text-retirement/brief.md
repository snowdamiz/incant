# 0058 — Native text-input retirement regression

Required model: Claude Opus 5.5 over ACP, thinking explicitly Max. Root/Astra
owns correctness; you own native interaction and rendered review. Read CLAUDE.md.
This is a focused follow-up to the actual focus-loss crashes you reported in0057.
Do not perform a new whole-app redesign or broaden the frozen source repair.

## Source and question

Base contains root fix f830f496 on actual main147. The managed Cocoa client now
inherits NSObject, not NSView; its protocol, weak window and immutable session
fencing are unchanged. The exact invisible AppKit regression aborts with the
old source and passes 768 contexts/256 stale-owner checks with the fix. All126
affected tests, Clippy and independent source review pass. See
`docs/spikes/macos-text-retirement.md` and its evidence packet.

Verify the ordinary composed game player can type, compose with the available
input method, blur and refocus repeatedly, move between fields, and close without
the old crash. Verify text delivery stays with its original owner, not a newly
focused field. Distinguish observed callbacks/behavior from inferred results.
No source-only or hidden-window check counts as this native acceptance.

## Reference material, read only

Your completed0057 packet lives at:
`/Users/andreyyurlov/.codex/worktrees/responsive-project-loading/incant/.worktrees/0057-native-window-game-ui/handoffs/0057-native-window-game-ui/`.
Read its result, native-runs/crash receipts and fixture/tool sources. Its old
lease has been released: never execute those old guarded tools or modify that
checkout. Adapt a small subset in this packet with a fresh0058 lease and own
process ledger. Reuse the public-command fixture design and its known Name,
Arabic and CJK fields; public command bus only for project authoring. Do not
claim injected Unicode commits are live Arabic/CJK IME conversion.

## Acceptance

- Build the ordinary `incant_game_player` with `project,native-window`, locked
  Rust1.99.0, ci profile, at most2 jobs. Use a freshly authored/cooked fixture;
  record exact source/player/package identities. No internal replacement player.
- At least three actual processes: repeated Latin edit/blur/refocus and field
  switches; available live dead-key preedit/commit across retirement; Unicode
  RTL/CJK commit and close. Exercise at least ten retirements overall. Each
  process must close cleanly and retain its native report. On a crash, retain
  the exact failure and return it for root repair; do not obscure it with retry.
- Briefly verify caret/selection/composition remain visible as before, and
  no text reaches a newly focused field from an old context. The known field
  click-submit, logical-focus restoration and overflow scrolling limitations
  remain separate root work; document them, do not silently count them fixed.
- No Linux acceptance is claimed by this task. The0057 shader/reference patch is
  a separate candidate; do not merge it into this fixed source as part of setup.
- No unrelated app changes, account changes, credentials, publishing or remote
  branch operations. No GPU/visual edits unless a real regression in this exact
  patch requires them; report a proposed change first in result if outside scope.

## Resource and desktop rules

All root builds in `~/.codex/incant-native-target` are terminal/reaped. That warm
target is now exclusively available to this handoff until release. Other agents
own their separate caches; do not stop or inspect their processes unnecessarily.
Evidence goes under `~/.codex/incant-evidence/0058`, never volatile /tmp.
Use short bursts after at least20seconds of actual user idle time; require an
unlocked awake display, check foreground owner before every event, stop/yield
on human input, and restore previous front app/pointer. Never fight user input.
At most12 window-only captures, no desktop capture. A time limit is not permission
to keep driving a stale window. Use a fresh explicit lease, expiry and release
marker; guard every build/run/capture/event tool and verify guards refuse after
release. End every owned player/probe, reap all builds, retain the process ledger.

## Deliverable

Commit scope-owned sources/tools/evidence with Built-by: claude. Leave root's
brief unchanged. Write result.md with exact model/effort, source and binary
identities, per-process events/outcomes, screenshots, limitations and all owned
process handles reaped. This is native text correctness/rendered acceptance only,
not Game UI, device performance or a phase gate.
