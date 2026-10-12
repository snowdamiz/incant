# 0058 native text-input retirement regression — result

## Status

**Passed on the actual composed player.** The macOS text-client fix in f830f496
holds in the ordinary `incant_game_player` window:

- **Processes.** Across three counted processes, the player typed, composed with
  the live dead key, blurred and refocused, switched fields, and closed without
  a crash.
- **Retirements.** 21 in total, 3 of them with live marked text active. Every
  process closed through the AX close button with exit status 0 and printed its
  native report. No crash report was written.
- **Ownership.** Text never reached a newly focused field from a retired context.
- **Rendering.** Caret, selection and live composition render bit-identically
  to the 0057 captures of the same states.

**One extra attempt failed, and I caused it.** A fourth process, p1-latin, ended
with exit 1 ("Error: Failed") after my `heap` probe suspended it. No input had
been sent. It is retained and reported below, and it is not counted.

This is native text correctness and rendered acceptance only. It is not Game UI
completion, device performance or any phase gate.

## Model and effort

Claude Opus 5.5, model ID `claude-opus-5-5`, thinking effort **max**.

Root's ACP record is `~/.codex/incant-evidence/0058/acp.log`: "Claude session
verified: Opus 5.5; thinking: max". I cannot inspect the ACP transport myself.

## Brief

Applied brief SHA-256: `a7e99f03…8de9527a`.

- It has no priority-revision section, and it did not change during the session
  (rechecked before release and before commit).
- `brief.md` is root's and is unchanged. No root review receipts were present.

## Identities

| Item | Identity |
| --- | --- |
| Source | HEAD `b583c82c` (main147 + root fix `f830f496` + routing), tree `549885bf`; clean outside this handoff at build |
| Fixed file | `third_party/tao/.../macos/text_input.rs` `84135a47…e1e623c`, equal to the spike's fixed hash; `super = NSObject` |
| Not included | the 0057 shader/reference candidate `e2551dd2` and caret accessor `ca111ccf` |
| Toolchain | rustc 1.99.0 (b940084d7), cargo 1.99.0; `Cargo.lock` `ac059d1a…abcac7f` |
| Player | ordinary `incant_game_player`, features `project,native-window`, `ci` profile, `--locked --offline -j2`: `38ee8b96…d6cebf`, 31,400,048 bytes |
| Fixture author | `incant_headless` (ci, -j2, locked) `043e1aa8…645e06b` |
| Fixture | `project.json` `6c50ea75…410548d0`, revision 3; script `69edca6c…`; fonts identical to 0057 |
| Package | the exporter's `data_package` (1,334,534 bytes); CPU startup check passed (18 widgets, 99 glyphs) |
| Host | Apple M5 Pro (Mac17,9), Metal, macOS 27.0.1 (26A434) |
| Window | every run opened on the external 1x display at (-2040,-158), 640×512 pt, viewport 640×480 |

How each piece was made:

- **Fixture.** `tools/fixture.py` authored a fresh project through public commands
  only: `init`, `import` and one atomic `command.execute`. Undo/Redo and a journal
  reopen were checked. It keeps 0057's design (Name, Arabic and CJK fields) with
  new `…58xx` identities.
- **Player.** `tools/build.py` follows the exporter's window build. The project
  is cooked by the player's own build script, so this is the shipping code path,
  not an internal replacement player.
- **Full record.** See `evidence/identities.json`.

## Native runs

**Input mechanisms:**

- **Keys:** CGEvent keys with US key codes at `kCGHIDEventTap`; Shift and Option
  are real modifier events.
- **Live composition:** the U.S. Option-E dead key, which produces real marked
  text, then a commit.
- **Arabic and CJK:** Unicode-string key events committed through the managed
  `NSTextInputContext`. This is not live Arabic or CJK IME conversion.
- **Pointer:** one pointer click.
- **Blur and refocus:** System Events activation of the previous front app, then
  of the player.
- **Close:** the AX close button.
- **Captures:** window-only `screencapture -l`.

The enabled input sources were U.S., Character Palette and PressAndHold.

| Run | PID | Exercised | Retirements | Outcome |
| --- | --- | --- | --- | --- |
| r1-latin | 28900 | Tab to Name; caret; " Byron"; selection; four Tab/Shift-Tab switches with one letter per field; four blurs, each with a different field focused; close with Name focused | 9 (4 switches, 4 blurs, 1 close) | exit 0; report: tick 1219, 1219 accepted frames, `discarded_stale_text_events` 0, `ui.focus` Name |
| r2-deadkey | 30776 | live Option-E marked "´"; blur while marked; E in Arabic; switch to Name; marked again, then pointer click into CJK; E in CJK; two switches; live "é" commit; marked again, then close | 6 (1 blur while marked, 1 pointer switch while marked, 3 switches, 1 close while marked) | exit 0; tick 770, 770 frames, stale 0, `ui.focus` Name |
| r3-unicode | 32684 | Arabic " مرحبا"; Tab; CJK "日本語"; blur with CJK focused; Arabic " نعم"; Tab; "一"; select two graphemes; Shift-Tab, Tab; close with CJK focused | 6 (4 switches, 1 blur, 1 close) | exit 0; tick 665, 665 frames, stale 0, `ui.focus` CJK |

Details:

- **Timing.** After each blur that ended a burst, the process kept running for
  20–38 s until the next burst. The two blurs inside burst p1b were followed by
  refocus and successful typing about 1.5 s later. In 0057 every blur with a
  focused field killed the player within 6 s (4 of 4).
- **Logs.** The exact steps are in `tools/plans/*.sh`, the per-run record in
  `evidence/native-runs.json`, the exits and reports in `evidence/runs/*/`, and
  the ledger in `evidence/processes.txt`.

### Observed versus inferred

**Observed** (on the ordinary player, macOS or in window captures):

- **Lifetimes.** Each process stayed alive and exited 0, as recorded by a
  supervising parent that waits on it. Each printed its JSON report. No new
  `incant_game_player` report appeared in `~/Library/Logs/DiagnosticReports`.
- **Field contents after the retirement sequences.** Each field received exactly
  the text typed while it was focused:

  | Run | Name | Arabic | CJK |
  | --- | --- | --- | --- |
  | r1 | "Ada Lovelace Byronef" | "سلام" + "bdg" | "中文ch" |
  | r2 | "Ada Lovelace Byroné" | "سلام" + "e" | "中文e" |
  | r3 | unchanged | "سلام مرحبا نعم", joined and right-aligned | "中文日本語一" |

- **Dead key across retirement.** After the dead key was retired by blur, and
  again by a pointer focus change, the next E produced a plain "e" in the newly
  focused field. No "´" or "é" leaked from the old context, and the cancelled
  preedit never reappeared in Name.
- **Live composition.** The marked "´" was drawn with its underline and IME
  caret, and a later live dead key committed "é" into the field that owned it.
- **Report counters.** `discarded_stale_text_events` was 0 in all three runs.
  `ui.focus` at exit matched the field that was focused when the window closed.
- **Focus not restored on refocus.** "g" and "h" landed in Arabic and CJK after
  Tab ×5 and ×6 from no focus. This matches the known limitation that refocus
  does not restore focus.

**Inferred** (from source, not observed directly):

- **One client per retirement.** Each trigger above retired and dropped exactly
  one managed client. The mechanism is `view.rs set_text_input_session` (one
  `managed_text`, `old.retire()` then drop) and `window/ime.rs`. Typed text can
  only arrive through the managed client on macOS, so each committed letter
  shows that its field had a live session at that moment.
- **Deallocation not observed.** I did not watch the retired clients deallocate.
  The `heap` census that could have shown it suspended the player and ended it
  (below), so I stopped using it. Surviving 21 retirements, including through
  close, is behavioural evidence that the formerly fatal dealloc path is fixed.
  It is not an object count.
- **Late callbacks.** Callbacks reaching a retired client are dropped inside the
  client and cannot be seen from outside. The stale counter only covers events
  that reached the active player.

### Rendered review (caret, selection, composition)

Compared by stored code values at tolerance 0, using 0057 run 2 on the same 1x
display as the "before" reference (`evidence/pixel-comparisons.json`):

| State | Region | Differing pixels |
| --- | --- | --- |
| Caret after "Ada Lovelace", focus ring | UI rows, 330×176 | 0 of 58,080 |
| "Byron" selected | UI rows | 0 of 58,080 |
| Live marked "´" (underline, IME caret) | UI rows | 0 of 58,080 |
| Committed "é" with caret | Name row, 320×34 | 0 of 10,880 |
| Title, body and buttons | rows 33–119 | 0 of 28,710 |

The controls do register differences:

- caret versus selection: 592 pixels, all inside "Byron";
- a 1 px shift: 5,625 pixels;
- the Arabic and CJK rows with the extra "e": 86 pixels.

By eye: the caret is crisp, the selection highlight sits under the text, the
composition underline spans exactly the marked glyph, and RTL text is joined and
right-aligned.

## Findings for root (not edited)

1. **A short process stall ends the window session, and the first error is
   lost.**
   - **What happened (observed).** `/usr/bin/heap` suspended the background
     player (p1-latin, PID 27789) for about 1.7 s, at a moment when it had no
     text session. Two seconds later the player exited with status 1. stderr
     held only `Error: Failed`; there was no report and no crash report.
   - **Unified log.** "window … finishing close", then CoreAudio
     `HALC_ProxyIOContext::IOWorkLoop: skipping cycle due to overload`
     (`evidence/runs/p1-latin/`).
   - **Why only "Failed" (inferred).** `window/mod.rs` assigns every callback
     error to `failure`. Once the first error has dropped the live session, the
     end-of-callback text-session sync calls `game.host()?`, which yields
     `GameError::Failed` and overwrites the original. The real first error
     (probably an audio or device check) is therefore unknown.
   - **Proposed.** Keep the first error (`failure.get_or_insert(error)`).
     Separately, decide whether a transient audio overload should be fatal.
   - **Scope.** Not a text-retirement failure, and only one occurrence.
2. **The click-submit limitation did not appear in one case.** In r2 the pointer
   click into CJK while Name had marked text focused CJK without submitting it
   (the crate stayed down).
   - **Why (inferred).** Losing focus cancels Name's preedit, which changes its
     display text and advances the layout generation. `UiInput` then cancels
     the captured press, so the release is suppressed.
   - **Not fixed.** A plain click into an unfocused field still submits (0057);
     that path is unchanged in source.

## Known limitations, still root work

- **Click-submit:** a pointer release on a focused field still calls `activate`
  and submits it.
- **Focus restoration:** refocus does not restore logical focus. This was
  observed again here.
- **Overflow scrolling:** single-line fields still have no horizontal scroll.
  This was not re-tested here.

None of these is counted as fixed.

Also not covered here:

- live CJK or Arabic IME conversion, candidate windows and clause handling
  (only U.S., Character Palette and PressAndHold sources are enabled);
- the press-and-hold picker;
- Linux, Windows or any CI;
- performance or phone results;
- 2x DPI typing (all three runs were at 1x).

Tool-check limit: `winctl` resolves the account home, not `$HOME`, so the
fake-home tests exercised only its "released" branch. Its expiry and
foreign-packet branches were not tested separately.

## Changed paths

Only `handoffs/0058-native-text-retirement/`; no engine source was edited.

- **`tools/`** (adapted from 0057 with a fresh 0058 lease guard):
  - `guard.sh`, `winctl.swift`, `setup.sh`, `fixture.py`, `build.py`;
  - `launch.sh`, which launches the player as its own child and records the real
    exit status;
  - `burst.sh`, `cap.sh` (12-capture budget), `heap.sh` (retired after its one
    use);
  - `pngtool.py`, `evidence.py`, `check_release.sh` and `plans/p1a–p3b.sh`.
- **`evidence/`:**
  - `identities.json`, `native-runs.json`, `pixel-comparisons.json`;
  - `processes.txt` (the ledger plus the burst and capture logs);
  - `release.json`, `checks/guards-after-release.txt`;
  - `runs/<run>/{exit.json,player-log.txt}`, plus the p1-latin log and heap
    excerpts;
  - copies of `fixture.json`, `package.json` and `headless.json`.
- **`screenshots/`** and this `result.md`.

## Commands and results

All runs are local on Apple M5 Pro and macOS 27.0.1, with
`CARGO_TARGET_DIR=~/.codex/incant-native-target`.

| Command | Result |
| --- | --- |
| `tools/build.py headless` (`build --locked --offline -j2 -p incant_headless --profile ci`) | exit 0 in 87 s |
| `tools/fixture.py` | public-command fixture; Undo/Redo and reopen checks passed |
| `tools/build.py player` (`--features project,native-window --profile ci -j2 --locked`) | exit 0 in 54 s (tao recompiled from this source); startup check exit 0 |
| `tools/launch.sh` + `burst.sh` plans p1a–p3b | 7 bursts, all exit 0; 6 window captures of 12 |
| `tools/check_release.sh` | every tool refused after release with nothing changed; 0 failures |

Not run: engine tests and Clippy. No engine source changed, and root reports 126
passing tests and clean Clippy for this fix.

## Screenshots

Window-only captures, colour-managed into the display profile.

- `screenshots/native-1x-r1-caret.png`, `native-1x-r1-selection.png`: caret and
  selection, both bit-identical to 0057.
- `screenshots/native-1x-r2-deadkey-preedit.png`: live marked "´".
- `screenshots/native-1x-name-caret-selection-preedit-commit-4x.png`: the Name
  row in all four states (4× zoom).
- `screenshots/native-1x-r1-after-nine-retirements.png`: all fields after r1's
  retirements.
- `screenshots/native-1x-r2-after-composition-retirements.png`: all fields after
  r2's retirements.
- `screenshots/native-1x-r3-rtl-cjk-selection.png`: RTL text and the CJK
  selection.
- `screenshots/native-1x-fields-after-retirements-r1-r2-r3-3x.png`: the three
  final field sets, zoomed 3×.

## Resources, lease and cleanup

- **Lease.** `~/.codex/incant-evidence/0058/lease.json` ran from 16:20:57 UTC
  (expiry 21:20:57) and was released at 16:38:42 UTC with the `released` marker
  (copy in `evidence/release.json`).
- **Guards.** After release, every build, run, capture, probe and `winctl` command
  refuses, and the evidence directory was unchanged.
- **Burst rules.**
  - Every launch and burst waited for an unlocked, awake display and 20 s of
    idle time.
  - Every event rechecked that the player was frontmost.
  - The previous front app and pointer were restored after each burst. A person
    switched apps between bursts, and that was respected.
- **Processes.** All owned handles are reaped:
  - 4 players: 3 closed with exit 0, and p1-latin exited 1 after my probe;
  - their supervisors and all 7 bursts finished;
  - 2 cargo builds and the CPU startup check finished with exit 0, their process
    groups reaped by `native_process`.

  Nothing owned is running.
- **Left in place.**
  - The worktree-root `node_modules` link, which the TypeScript cook needed, is
    removed.
  - The warm cache `~/.codex/incant-native-target` is kept, with nothing deleted.
  - 0057's files were only read.
  - No other agent's process was touched.
