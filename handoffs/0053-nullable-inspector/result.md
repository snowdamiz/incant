# 0053 native nullable Inspector recheck: result

## Status

- **Clip field: pass.** In the native WKWebView editor, the rig's Animation component shows Clip as
  a text field named "Clip" with the value `0`. It uses the existing number field: its value box
  is pixel-identical to Seconds' (0 differing pixels), and against Speed only the 0/1 glyph
  differs. No unsupported-type notice or raw-JSON fallback appears anywhere in the window, in any
  of the nine state reads.
- **Adjacent labels and values: pass at 1440 × 875 and 1000 × 650.**
  - Labels share one column on a steady 34 pt pitch, and the Clip label now sits on its siblings'
    baseline.
  - At 1440 × 875, the Inspector column from Looping through the Script component matches
    0052's accepted capture, moved up 41 pt, to within ±1 code value. 0052's open agent pane
    covered the rows below that.
  - At 1000 × 650 (the window's minimum), the whole component is bit-identical to its 1440
    rendering.
- **Play, Pause and Stop: pass.**
  - ⌘P showed Playing about 1.0 s after the press, and ⇧⌘P paused at tick 10 about 0.6 s after
    its press. ⌘P from Paused returned to Live about 0.6 s after the press, with "Play stopped.
    Runtime changes were discarded."
  - The Runtime block appears above the unchanged Animation component and goes away on Stop.
  - Clip takes focus; Tab leaves it for Looping and ⇧Tab returns to it. It keeps focus through a
    Resume/Pause cycle.
- **Authoring unchanged: pass.** All 12 fixture files, including `project.json`,
  `project.journal.jsonl` and `animation.ts`, are byte-identical before launch and after quit.
  Their modification times are unchanged, and they still match the fixture manifest. History
  stayed at 2 entries. No authoring edit was made.
- **No remaining appearance defect in the corrected field.** Two behaviour observations for Astra
  came up beside it (below): one dropped focus in the Runtime block, and unconfirmed transient
  positions during Resume/Pause.
- **No source change.** No UI, bridge, Rust, style or fixture file was edited. No provider turn was
  sent, and no account dialog was opened.
- **Native and GPU ownership: released** at 01:56:59Z. The editor exited at 01:56:44Z, and no
  owned process remains.
- **No gate claim.** This makes no phase, platform, device, IME, frame-pacing or
  shipping-performance claim. The binary is a `ci` profile build.

## Model and effort

- **Model:** Claude Opus 5.5, model ID `claude-opus-5-5`.
- **Thinking effort:** Max.
- **How it was set:** the runner `tools/handoff/main.py run 0053-nullable-inspector` (PID 3880)
  started this session through `claude-agent-acp` (PID 3945). `tools/acp/config.json` pins
  `claude-opus-5-5` and `max`; it is `a911ce8f…` in both the parent checkout and this worktree.
  `configure_model_and_effort` raises before prompting unless ACP confirms both.
- **What I could not see:** the runner's confirmation line is not visible from inside the session.
- **No substitution, no routing exception, no auxiliary agent.**

## Packet

- **Revisions:** the brief has no priority revision and no new director feedback. It was applied
  as written.
- **Starting state:** the worktree was clean at `f230de0`, with no interrupted attempt.
- **Root's files:** `brief.md` and `build.json` were not changed.
- **Out of scope:** the missing Console logs observation, which Astra and Sol are implementing.

## Build, fixture and window

| Item | Value |
| --- | --- |
| Grant | `build.json`: `native_gpu_granted: true`, recorded 2026-10-12T01:24:20Z, exclusive to 0053. Rechecked by `launch.sh` before the launch. Fresh helper paths (`/tmp/incant0053`); 0052's released marker was not read or changed |
| Build manifest | `/tmp/incant-nullable-inspector-0053-bin-v1/manifest.json`, `67f60a8e…dd62`, source `2841c24`, profile `ci`. 1,040 of its 1,059 source hashes match this worktree and none differ. The other 19 are `editor/ui/dist/` build outputs, which are not in the worktree. The fixed `editor/bridge/native.ts` and `native.generated.js` are among the matches |
| Editor | `incant_editor` `677f3955970e5f63621dbc4a5ac0df09dce2c90d520b5278918da6ffcd3ae5f4`, 63,194,672 bytes. Hash rechecked at launch. It contains the generated bridge's `Array.isArray(raw.type)` branch, which the pre-fix generated bridge (`30eacbc^`) lacks. Not rebuilt or replaced |
| Play-time inputs | The editor resolves its TypeScript cook to the parent checkout, which is clean at `f230de0`. There, `tools/build_native_script.mjs`, `package.json` and `package-lock.json` equal the manifest's entries. Node v24.19.0 came first in the editor's PATH, as in 0052 |
| Fixture | `/tmp/incant-animation-native-review-v2/project.json`. The fixture manifest `069789be…56f2` matched all 12 files with no extras, before launch and after quit. The authored Animation value is `{"clip": 0, "looping": false, "playing": true, "seconds": 0, "speed": 1}` |
| Host | macOS 26.6 (25G72), Apple M5 Pro, built-in display 1512 × 982 pt at 2×. The visible desktop is 1512 × 875 pt |
| Window | The editor asks for 1440 × 900; macOS opened it at **1440 × 875**, the largest that fits. The AX resize back to 1440 × 875 returned 875, not 0052's 874. **1000 × 650** is the window's minimum, and it was returned as asked |

## Results

The numbers come from AX reads of PID 7560 (`evidence/states.jsonl`, `flows.jsonl`) and from
`imgtool` over the captures (`evidence/measurements.json`, which records each command). Each
transport time runs from the key press to the first AX read that shows the new state word.
`flow.py` logs each key 0.25 s after its press, and the wait loop polls every 0.1–0.3 s, so the
times are good to about 0.2 s.

| Check | Result |
| --- | --- |
| Clip value and presentation | **Pass.** Clip is an `AXTextField` named "Clip" (title and label) with the value `0`. Its box is 206 × 28 pt at the Seconds and Speed boxes' x, and its fill is `#1e2025` like theirs. Across the value side of the row (x 1200–1440 pt, 480 × 68 px), Clip's band is pixel-identical to Seconds' (both read 0). Against Speed, only the glyph differs: 183 px inside a 12 × 19 px box. Capture 01 |
| No fallback | **Pass.** No node in the window holds "not supported", "Unsupported field type", "Cannot show as" or "raw value". This held in all nine reads: Live, Playing and Paused, at window widths of 1512, 1440 and 1000 pt |
| Labels and adjacent rows, 1440 × 875 | **Pass.** All labels start at x 1092 pt on a 34 pt pitch (Clip 118, Looping 152, Playing 186, Seconds 220, Speed 254). The Clip label's ink spans 19–41 px of its row, like Looping, Playing and Speed; in 0052's tall notice row it sat 1 px higher. From Looping through the Script component, the column matches 0052's capture 01 moved up 41 pt, to within ±1 code value: 3,924 of 529,920 px differ by 1, and none by more than 8. The heading matches to ±1. Capture 01 and crop 04 |
| 1000 × 650 | **Pass.** The Inspector column stays 360 pt. Under the Runtime block, the Animation component's 720 × 440 px band is bit-identical to the 1440 × 875 capture: 0 of 316,800 px differ, and the largest difference is 0. The whole component is visible with the agent pane collapsed. In the Runtime block, only the tick number differs (10 or 195). Captures 02 and 03 |
| Play and Pause | **Pass.** ⌘P from the focused Hierarchy row showed Playing about 1.0 s after the press. ⇧⌘P showed Paused about 0.6 s after its press, at tick 10, with "Paused at tick 10." The Runtime block (Translation 0 0 0, Rotation 0 0 0 1, Scale 1 1 1) sits above the Animation component, whose rows are unchanged. Three more Resume/Pause pairs took about 0.8/0.7, 0.8/0.8 and 0.8/0.7 s. While Playing, the Runtime block keeps the last read's values (tick 118) until the next pause reads tick 195, as `RuntimeSection` documents. Capture 02 |
| Stop | **Pass.** ⌘P while paused showed Live about 0.6 s after the press. The status line reads "Play stopped. Runtime changes were discarded.", and the Runtime block is gone. Play is enabled; Step and Stop are disabled with their reasons. The Clip row is back at y 112 with value 0 |
| Clip usable alongside runtime state | **Pass.** While paused, an AX-located click focused Clip. Tab moved to Looping and ⇧Tab came back. Focus stayed on Clip for 3 s idle, 3 s after the keys and 2 s after a pointer move. With Clip focused, a full Resume → Pause cycle kept it focused, checked for 4 s after. When the window came forward again, WebKit restored focus to Clip. Play shortcuts work with focus in the read-only field, since `isTextEntry` is false for read-only inputs |
| Project, journal and behaviour source | **Unchanged.** `project.json` `5af5a8bb…`, `project.journal.jsonl` `f1ef9835…` and `animation.ts` `f16ead4a…` are identical before launch (01:29Z) and after quit (01:56Z). So are the other nine files, with no additions, removals or modification-time changes. All 12 also equal 0052's post-quit snapshot, including modification times |

## Observations for Astra

1. **Runtime-field focus dropped once.**
   - **What happened:** in the first pause after Play started (burst 3), ⇧Tab put focus on the
     Runtime block's Translation Y field (AX, 01:41:46.33Z). By 01:41:47.23Z, focus had fallen to
     the web area itself, so nothing was focused. My only input in between was a pointer move to
     the titlebar, and a pointer move does not clear focus in `inputModality.ts`. This is why
     capture 02 shows no focus ring.
   - **Not reproduced:** in a steady pause, the same field kept focus for 6 s idle and 3 s after
     Tab/⇧Tab. A field clicked straight after a later Resume/Pause kept focus for 4 s.
   - **Candidate cause, unverified:** `RuntimeSection` returns `null` while `feed.transport` is
     set, and `PlayContext` resets the read when `inspectable` flips. A re-render like that would
     unmount the Runtime fields and drop focus held inside them.
   - **Effect on the corrected field:** none. The authored Clip field never lost focus.
2. **Possible transient shift during Resume/Pause, unconfirmed.**
   - **What I saw:** in one cycle, AX gave Clip's box y 282, read 0.25 s after the Resume key,
     and y 305, read 0.25 s after the Pause key while the header still said Playing. Every other
     read gave 284, including the next cycle's reads at the same delays.
   - **Not confirmed:** no capture covers a transition. If the shift is real, the components under
     the Runtime block jump by up to 21 pt for a fraction of a second.
3. **The window changed before my first input.**
   - **What changed:** between launch (01:30:38Z, when `launch.sh` handed the front back) and the
     first burst (01:35:55Z), the window was filled to 1512 × 875 at x 0, and the agent pane was
     collapsed. When that burst started, the editor was in the background again.
   - **My input:** I sent none in that interval, only AX reads. The user was active then: the
     burst waited 254 s for 15 s without input.
   - **How the pane works:** its open state is per-launch React state (`App.tsx`). It starts open
     at heights of 720 pt and above and changes only through its toggle.
   - **What I did:** I restored the 1440 × 875 frame for the checks and left the pane collapsed. It
     does not change the Animation component. So all three captures show the pane collapsed, unlike
     0052's 1440 captures.

## Captures

There are three original window captures, the packet's limit, plus one crop sheet. All are in
`screenshots/native/`, each reviewed, and the README describes each one.

- `01-authoring-1440x875-clip-0.png`: Live, rig selected, Clip `0`.
- `02-paused-tick10-1440x875-runtime-and-animation.png`: Paused at tick 10, with the Runtime block
  above the Animation component.
- `03-paused-tick195-1000x650.png`: Paused at tick 195 at the minimum size.
- `04-crop-clip-before-after-1440-and-1000.png`: the Inspector column from 0052's capture 01
  (before), 0053's capture 01 (after, 1440) and 0053's capture 03 (after, 1000 × 650).

- **Redaction:** the titlebar account avatar is filled with the titlebar colour (`#0b0c0f`) in
  01–03. Each image differs from its raw capture in exactly 2,040 pixels, all inside the avatar's
  50 × 50 px box. The images keep the captures' Display P3 profile.
- **Raw captures:** they stay local in the git-ignored `artifacts/0053/raw/`. Hashes for both sets
  are in `evidence/hashes.txt`. 0052's capture 01 was read for the comparisons; its hash is
  unchanged (`cf79269d…`).

## Processes and cleanup

| PID | What | End |
| --- | --- | --- |
| 7560 | `incant_editor` (the granted hash) on the fixture, launched 01:30:33Z after a 70 s idle wait | ⌘Q at 01:56:43Z, exited 01:56:44Z; the editor log is empty |
| — | Play-start TypeScript cook children of the editor, short-lived and not inspected | none remain |
| — | `launch.sh`, 11 `burst.sh` runs, `winctl`, `flow.py`, `state.py`, `check.py`, `neutral.py`, `imgtool`, `fixture.py` | all exited |
| 5343 and children | Not mine: Astra's Codex app server. During the interval it ran `gh pr checks 111 --watch`, plus `cargo build -p incant_headless` and `cargo test … --features incant_assets_runtime/visual` in the `color-mip-roles` worktree, then `cargo test -p incant_runtime -p incant_script` | left alone. I did not check whether the `visual` tests use the GPU |

- **Final check:** after quit, `pgrep -f '^/tmp/incant-nullable-inspector-0053-bin-v1/'` finds
  nothing, and no helper is running.
- **Bursts:** each burst waited for 15 s without user input. Waits were 254, 50, 22, 150, 0, 206,
  24, 0, 0, 108 and 0 s. App 1866 was in front at every burst start and again at every end.
  - Bursts that raised the editor handed the front back and put the pointer back.
  - Burst 4a never raised the editor.
  - In burst 9, macOS returned the front to app 1866 when the editor quit. That burst moved no
    pointer.
  - `winctl` checked the front app before every event.
- **Release:** native and GPU ownership is released, recorded at 01:56:59Z in
  `/tmp/incant0053/released` and `evidence/processes.txt`. Afterwards, `cap.sh`, `burst.sh` and
  `launch.sh` all refused to run (exit 2), and no file was written.

## Slips and unsuccessful attempts, recorded

- **Coordinate click (burst 3):** I aimed the click at Clip's Live position (1318, 126). In
  Paused, the Runtime block pushes Clip down to y 284, so the click focused the Runtime Translation
  Y field instead. That field is read-only, so nothing changed. The Clip focus probe was redone
  with AX-located clicks in bursts 4a2 and 5.
- **Watch before raise (burst 4a):** my passive focus watch started before anything had brought
  the editor forward. The front-app guard stopped the burst with no input posted, and I reran it
  with `raise` first.
- **Read-only redo:** one zsh loop did not word-split its `imgtool sample` arguments, and I redid
  it with `${=xy}`.

## Changed paths

All are new, in `handoffs/0053-nullable-inspector/`:

- `result.md`;
- `evidence/`: `states.jsonl`, `flows.jsonl`, `measurements.json`, `hashes.txt`,
  `fixture-before.json`, `fixture-after.json`, `fixture-compare.json`, `images.txt`, `bursts.txt`
  and `processes.txt`;
- `screenshots/native/`: 4 PNGs and a README;
- `tools/native/`, with these files:
  - from 0052 with 0053's binary, hash, grant, scratch directory and a three-capture budget:
    `winctl.swift` and `imgtool.swift` (only their header comments changed), `launch.sh`,
    `burst.sh`, `cap.sh` and `neutral.py`;
  - extended: `flow.py` (`watch`), `state.py` (Animation rows with values, a window-wide fallback
    search and the status line) and `fixture.py` (`verify` against the fixture manifest);
  - new: `check.py`, which refuses a capture unless the state read matches.

Outside the worktree, I wrote only my scratch directory `/tmp/incant0053`: built helpers, logs and
the release marker. `artifacts/0053/` is ignored and local.

## Commands run

- **Before launch:** `shasum -a 256` on the binary, build manifest and fixture manifest. A Python
  check of all 1,059 manifest source hashes against the worktree. `fixture.py snap` and `verify`.
- **Launch:** `tools/native/launch.sh <worktree>/artifacts/0053/editor.log
  /tmp/incant-animation-native-review-v2/project.json`.
- **Each step:** `tools/native/burst.sh "<steps>"`. Inside a burst, `winctl` handled raise, click,
  frame, move and key, plus AX-located clicks. Then `flow.py` keys, wait and watch, `state.py`,
  `check.py`, and `cap.sh` for captures 01–03. Every burst's output is in `evidence/flows.jsonl`.
- **After quit:** `fixture.py snap`, `compare` and `verify`. `imgtool redact`, `crop`, `stack`,
  `diff`, `rdiff`, `ink` and `sample`. Every measurement's command is stored with its output.

## Checks not run

- **Source checks:** the UI suite, bridge tests, strict TypeScript, generated transport and
  production build were not re-run, because I changed no source. They are Astra's evidence, and I
  do not claim them.
- **Null Clip:** the null case ("Not set") was not exercised natively, because the fixture's Clip is
  0.
- **Agent pane open at 1000 × 650:** not rechecked, because the pane was collapsed when I started.
  In 0052, with it open, the rows below Clip sat below the fold.

## Limitations

- **Coverage:** one Mac, one display and macOS only; `ci` profile. Not a device, frame-pacing or
  shipping-performance measurement.
- **Paused and stopped states only:** all three captures are paused or Live states. Running
  playback and transitions were read through AX, not captured.
- **Network:** not monitored. The editor runs its provider `restore` action whenever its window
  gains focus, which every burst caused. No provider turn, account dialog, sign-in or sign-out
  took place.

## Open questions

1. **For Astra:** observation 1. Does `RuntimeSection` unmount after the first pause of a session,
   dropping focus held in its fields?
2. **For Astra:** observation 2. Do the components under the Runtime block jump while Resume or
   Pause is in flight?
3. **Data binding, carried from 0052:** should the Inspector name the clip, for example
   "0 · move"? That needs the cooked model's clip table in the snapshot.
4. **For the director:** none.
