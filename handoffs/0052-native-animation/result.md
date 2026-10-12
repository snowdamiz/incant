# 0052 native skeletal playback acceptance: result

## Status

- **Skeletal playback in native Play: pass.** In the actual WKWebView editor, native Play on the
  Stationary camera shows the joint moving the triangle in the move clip and rotating it about
  the world origin in the rotate clip. At every paused tick (39, 49, 139, 209) the rendered
  triangle matches the fixture's analytic pose within about 1 px at its centroid. The Runtime
  block reads the entity at identity each time, so this is joint articulation, not an entity
  transform or a text counter.
- **Pause, Step, Resume, Stop and restart: pass.**
  - Paused frames 70 s apart are bit-identical in the viewport.
  - Every Step advances exactly one tick, and ten steps move the triangle by the matching amount.
  - Resume animates again.
  - Stop returns to the authored preview pose.
  - A fresh run's tick-209 frame is bit-identical to the first run's.
  - Every transport action worked from the keyboard, and focus stayed usable.
- **Resize: pass.** Play kept running through resizes to 1000 × 650 and back to 1440 × 874. The
  poses at both sizes match their ticks. Step, the transport and the Inspector stay usable at
  1000 × 650.
- **Authoring unchanged: pass.** All 12 fixture files, including `project.json`,
  `project.journal.jsonl` and `animation.ts`, are byte-identical before launch and after quit,
  with unchanged modification times.
- **One presentation defect, for Astra.** The Animation component's **Clip** row shows "This
  field’s type is not supported yet; raw value: 0". The cause is in the bridge's schema
  normalization, not in styling (details below). Its layout is otherwise intact at both window
  sizes.
- **No source change.** No UI, bridge, Rust, style or fixture file was edited. No provider turn
  was sent, and no account dialog was opened.
- **Native and GPU ownership: released** at 01:09:47Z. The editor exited at 01:09:11Z, and no
  owned process remains.
- **No gate claim.** This makes no phase-gate, reference-phone, shipping-performance or
  cross-platform claim.

## Model and effort

- **Model:** Claude Opus 5.5, model ID `claude-opus-5-5`.
- **Thinking effort:** Max.
- **How it was set:** the runner `tools/handoff/main.py run 0052-native-animation` (PID 23188)
  started this session through `claude-agent-acp` (PID 23268). `tools/acp/config.json` pins
  `claude-opus-5-5` and `max`. `configure_model_and_effort` raises before prompting unless ACP
  confirms both.
- **What I could not see:** the runner's confirmation line is not visible from inside the
  session.
- **No substitution, no routing exception, no auxiliary agent.**

## Packet

- **Revisions:** the brief has no priority revision and no new director feedback. It was
  applied as written.
- **Starting state:** the worktree was clean at `f74379e`, with no interrupted attempt.
- **Root's files:** `brief.md` and `build.json` were not changed.

## Build, fixture and window

| Item | Value |
| --- | --- |
| Grant | `build.json`: `native_gpu_granted: true`, recorded 2026-10-12T00:37:29Z, exclusive to 0052. Rechecked by `launch.sh` before the launch |
| Build manifest | `/tmp/incant-native-animation-0052-bin-v1/manifest.json`, `eba86ebe…c799f`, source `edfc443`. All 934 source hashes match this worktree. Its dist entries are build outputs, which are not in the worktree |
| Editor | `incant_editor` `4c1bf1f00e14fd5a2a539b08f2cead1787c2da2d4dbf6cbefd47090e6e0b39a1`, 63,137,680 bytes. Hash rechecked at launch. Not rebuilt or replaced |
| Headless | `02075c09…31cf`, 53,022,992 bytes. Verified, never launched |
| Play-time inputs | The editor resolves its TypeScript cook to the parent checkout, which is clean at `f74379e`. There, `tools/build_native_script.mjs` and `package-lock.json` equal the manifest's entries. Node v24.19.0 came first in the editor's PATH, as in 0050 |
| Fixture | `/tmp/incant-animation-native-review-v2/project.json`. The fixture manifest `069789be…56f2` matched all 12 files with no extras. Scene Main, entity `…02`, camera `…03`, script `…01`, asset `01M4PRJ7250N1KDH27C3D2DDVV`. v1 was not used |
| Host | macOS 26.6 (25G72), Apple M5 Pro, built-in display 1512 × 982 pt at 2×. The visible desktop is 1512 × 875 pt |
| Window | Launched at **1440 × 875**, the largest that fits; 1440 × 900 does not fit. Resized to **1000 × 650** during Play. macOS returned **1440 × 874** for every later request (875 and 900 were both asked), so captures 06–08 are at 874 |

**What the fixture should show** (`tools/native/predict.py`):

- **Geometry:** one triangle, (0,0,0), (1,0,0) and (0,1,0), facing +Z. Every vertex is fully
  weighted to the single joint, with an identity inverse bind.
- **Clip 0 "move":** the joint's X goes 0 → 2 over 2 s.
- **Clip 1 "rotate":** the joint turns 0 → 180° about Z over 2 s.
- **Timeline:** `animation.ts` selects clip 0 at tick 1 and clip 1 at tick 121 of every 240-tick
  cycle. So at tick N the joint is at X = (N−1)/60 in the move phase, and the rotate phase turns
  1.5° per tick.
- **Sampling:** the runtime samples each frame from the rest pose
  (`AnimationPlayer::sample`), so the rotate clip pivots on the origin.

## Results

The numbers come from AX reads of PID 62948 (`evidence/states.jsonl`, `flows.jsonl`) and from
`imgtool blob` over the native viewport rectangle (`evidence/measurements.json`).

| Check | Result |
| --- | --- |
| Rig in the authoring preview | **Pass.** A lit grey triangle (`#8b8b8b` in the P3 capture) on the preview background (`#141519`). Its corners are within 0.4–2.0 px of the authored pose (clip 0 at 0 s, the rest pose) as the authoring preview projects it. That preview always draws from the renderer's fixed eye at (6, 5, 9), 50°, so it is oblique; the Play view uses the scene camera. Capture 01 |
| Scene and camera | One scene and one camera exist, so no picker is shown; DESIGN.md says they are used without asking. The Play images match the Stationary camera's projection (0.5, 0.5, 8), 60°, at every tick |
| Native Play, move phase | **Pass.** ⌘P from the focused Hierarchy row showed Playing about 1 s after the press, and ⇧⌘P paused at **tick 39**. The joint is at +0.633: the centroid is within 0.86 px of tick 39 (1.44 px for tick 38, 2.62 px for tick 40), and the area ratio is 1.015. The Runtime block reads the entity's Translation 0 0 0, Rotation 0 0 0 1 and Scale 1 1 1. Capture 02 |
| Paused pose across redraws | **Pass.** Capture 03 came 70 s after 02. In between, the editor spent time in the background, came forward and was held in front for 5 s. Paused frames redraw from the committed world (`game.refresh()`). The viewport is **bit-identical**: 0 of 1,779,288 pixels differ. The only other change is the status line's "Paused at tick 39." expiring |
| Step | **Pass.** Nine ⌥⌘P presses gave ticks 39 → 48, each exactly +1 in the header's paused tick; the Runtime block then read tick 49 with the entity at identity. Tab (row → separator → Resume → Step) and then Space on Step gave 49. The triangle moved 19.67 px, which is 0.170 units; 10 ticks is 0.167. The centroid is within 0.35 px of tick 49. Further single steps were 138 → 139 at 1000 × 650 and 43 steps from 166 to 209. Capture 04 |
| Resume, rotate phase | **Pass.** After resuming, the poses at tick 139 (27.0° expected, 28.1° measured, at 1000 × 650) and tick 209 (132.0° expected, 131.5° measured) pivot on the world origin. The joint's translation returned to rest, with no stale move offset. Captures 05 and 06 |
| Stop | **Pass.** ⌘P while paused showed Live about 0.7 s after the press. The viewport is the authoring preview again, with the authored pose (corners within 0.25–1.87 px). The area is 3,760 px, and the 1 pt shorter viewport predicts 3,759 from 01's 3,773. The Runtime block is gone, and the status line reads "Play stopped. Runtime changes were discarded." History stayed at 2 entries. Capture 07 |
| Fresh run | **Pass.** I Tabbed to Play and pressed Space; Playing showed about 1.1 s after the press. Space on the same button (now Pause) paused at tick 166, and 43 steps reached 209. The viewport is **bit-identical** to the first run's tick 209: 0 of 1,775,956 pixels differ. Capture 08 |
| Keyboard and focus | **Pass.** ⌘P plays and stops, ⇧⌘P pauses and resumes, and ⌥⌘P steps, with focus on the Hierarchy row, a transport button or the agent panel; focus stayed where it was. Space activates Play, Pause, Step and Resume when they hold focus. Focus stays on the transport button as it changes name (Play → Pause → Resume → Play). Unavailable buttons stay focusable and give their reason ("Step works while Play is paused.", "Play is not running.") |
| Resize | **Pass.** While running, a resize to 1000 × 650 kept "Playing", and the session kept running across the resize back to 1440 × 874 (tick 139 → 209). At 1000 × 650 the viewport is 393 × 309 pt, and the transport sits at the header's trailing edge with correct enabled states. ⌥⌘P stepped there. With the agent pane collapsed, the Runtime block and the whole Animation component are visible. With it open, the Inspector body is 256 pt tall, so the Animation rows below Clip sit below the fold (scrolling was not tested). Captures 05 and 06 |
| Animation component | **No layout regression.** The rows align in the label and value columns at both sizes, nothing clips, and the Clip notice wraps to two lines. The Inspector column is 360 pt at both sizes, so the component is identical. **Defect:** Clip shows the unsupported-type notice (next section). Capture 10 |
| Missing or clipped content, culling, flicker, vanishing triangles, stale pose | None seen. The triangle is complete in all eight captures: the area ratio is 0.998–1.015 in Play. The front face is visible at every pose. No triangle vanished. No stale pose appeared after Stop or in the restarted timeline. Flicker during running playback was not frame-sampled (see Limitations) |
| Project, journal and behaviour source | **Unchanged.** `project.json` `5af5a8bb…`, `project.journal.jsonl` `f1ef9835…` and `animation.ts` `f16ead4a…` are identical before launch (00:48:06Z) and after quit. So are the other nine files, with no additions, removals or modification-time changes. No authoring edit was made, so no copied fixture was needed |

## Defects and observations for Astra

1. **Animation Clip shows as unsupported.**
   - **Source:** `schemas/Animation.schema.json` declares `clip` as
     `"type": ["integer", "null"]`, which is how schemars writes `Option<u16>`.
   - **Cause:** `inspectorField` in `editor/bridge/native.ts` resolves a nullable only in the
     `anyOf: [T, {type: "null"}]` form (lines 104–110). Any non-string `type` returns
     `unsupported` (line 127), so the Inspector falls back to its notice and the raw JSON.
   - **Likely fix:** treat a two-member `type` array that contains `"null"` like the `anyOf`
     branch: the non-null type, plus `nullable: true`. Clip would then be an integer value box,
     and "Not set" when null, as DESIGN.md describes unset values.
   - **Why I did not change it:** this is bridge normalization, which is nonvisual. It also needs
     a regenerated `native.generated.js`, a bridge test and a newly granted binary.
2. **Script logs are not in the Console.** The fixture logs `skeletal:<tick>:<clip>:<seconds>`
   every 30 ticks, yet the Console stayed at its one startup entry through both runs.
   `crates/incant_play/src/control.rs` exposes `logs()`, but `editor/app` never reads it. Whether
   that is intended for now is your call. Here, the timeline came from the paused tick and the
   pixels.
3. **Out of scope, recorded only:**
   - **Agent panel focus ring.** After a pointer click on the agent panel's disclosure, focus
     landed on the panel `<section>` (`tabIndex=-1`), whose focus ring then stayed visible
     (06, 07). Likely cause: WebKit does not focus a clicked button. This is unrelated to Play.
   - **Window height.** macOS clamps AX resizes of this window to 874 pt high.

## Captures

There are eight original window captures, the packet's limit, plus two crop sheets. All are in
`screenshots/native/`, each reviewed, and the README describes each one.

- **Redaction:** the titlebar account avatar is filled with the canvas colour in 01–08. Each
  image differs from its raw capture in exactly 2,040 pixels, all inside the avatar's box. The
  images keep the captures' Display P3 profile.
- **Raw captures:** they stay local in the git-ignored `artifacts/0052/raw/`. Hashes for both
  sets are in `evidence/hashes.txt`.

## Processes and cleanup

| PID | What | End |
| --- | --- | --- |
| 62948 | `incant_editor` (the granted hash) on the fixture, launched 00:50:44Z after a 150 s idle wait | ⌘Q 01:09:05Z, exited 01:09:11Z; the editor log is empty |
| — | Play-start TypeScript cook children of the editor (`crates/incant_cook`), short-lived and not inspected | none remain |
| — | `launch.sh`, 12 `burst.sh` runs, `winctl`, `flow.py`, `state.py`, `neutral.py`, `imgtool`, `fixture.py` | all exited |
| 24076, 24114 | Not mine: Astra's CPU-only `tools/probes/native-animation.py` and its cargo build, seen at 00:40Z | gone before my launch (absent at 00:48:06Z) |

- **Final check:** after quit, `pgrep` finds no process from the 0052 binaries and no helper.
- **Bursts:** each burst waited for 15 s without user input and handed the front and pointer
  back. `winctl` checked the front app before every event. Waits were 196, 70, 24, 36, 14 and
  40 s for six of the bursts.
- **Interrupted burst:** one burst was cut short when the user switched apps (see Slips).
- **Release:** native and GPU ownership is released, recorded at 01:09:47Z. My `launch.sh`,
  `burst.sh` and `cap.sh` now refuse to run; a post-release `cap.sh` call was refused.

## Slips and unsuccessful attempts, recorded

- **Interrupted hold:** the first Play burst's 5 s hold stopped when the user brought another app
  forward. The guard ended the burst after capture 02, before 03. Capture 03 was taken in the
  next idle burst instead, which added a background/foreground round trip to the stability check.
- **Header-word scoping:** my first state read counted the Animation field label "Playing" as a
  Play state word. `state.py` and `flow.py` were fixed before any Play input to read only the
  Viewport header line.
- **Window height:** two requests for 1440 × 875 (875 and 900) both came back 874. The fresh-run
  comparison therefore used tick 209 against 06, at the same size.
- **Agent panel:** the agent panel was collapsed for capture 05 and expanded again straight
  after.
- **Read-only redos:** two calls were redone, with no input posted. One was a zsh
  non-word-splitting `imgtool sample` call. The other was the evidence assembler choking on my
  own 200-character display echo.

## Changed paths

All are new, in `handoffs/0052-native-animation/`:

- `result.md`;
- `evidence/`: `states.jsonl`, `flows.jsonl`, `measurements.json`, `hashes.txt`,
  `fixture-before.json`, `fixture-after.json`, `fixture-compare.json`, `images.txt`,
  `bursts.txt` and `processes.txt`;
- `screenshots/native/`: 10 PNGs and a README;
- `tools/native/`, with these files:
  - adapted from 0050: `winctl.swift` (unchanged behaviour), `launch.sh` (0052 hash, grant
    and release checks, Node PATH, no network sampler), `burst.sh`, `cap.sh` (eight-capture
    budget, front check), `flow.py` (wait, hold and steps) and `state.py` (header-scoped
    state, Runtime block and Inspector texts);
  - extended from 0050: `imgtool.swift` (`rdiff`, `blob`);
  - new: `fixture.py` and `predict.py`.

Nothing outside this worktree was changed. `artifacts/0052/` is ignored and local.

## Checks not run

- **UI suite, strict TypeScript and production build:** no UI source changed.
- **Not re-run, not claimed:** the earlier CPU and numerical GPU tests (native clocks, committed
  TypeScript snapshots, atomic controls, source-free startup, skin deformation, single-sided
  reflection and shadows) are Astra's evidence. I did not re-run them and do not claim them.

## Limitations

- **Coverage:** one Mac, one display and macOS only; desktop profile. 1440 × 900 does not fit, so
  the large-window checks ran at 1440 × 875 and 1440 × 874.
- **Paused states only:** all eight captures are paused or stopped states, so ticks are
  deterministic. Running playback was not frame-sampled for flicker or pacing, and the UI does
  not expose a presented-frame count.
- **First frame after restart:** it was not captured. The fixture re-selects clip 0 at 0 s on
  tick 1 of every run, which would overwrite stale animation state from tick 1. The identical
  tick-209 frame proves the timeline restarts, not that the first frame was fresh.
- **Fixture scope:** one joint and one rigid triangle, with no bending, multi-joint weights,
  material or shadow (`cast_shadows` is false). This covers articulation, clip switching, the
  transport and resize, not deformation quality.
- **Measurement precision:** about 1 px at the centroid, which is about ±0.5 tick in the move
  phase. At 1000 × 650, about ±1° in the rotate phase.
- **Network:** not monitored. The editor runs its provider `restore` action whenever its window
  gains focus (`editor/app/src/main.rs`), which every burst caused. No provider turn, account
  dialog, sign-in or sign-out took place.

## Open questions

1. **For Astra:** should the bridge accept `type: [T, "null"]` (defect 1)?
2. **For Astra:** should native Play forward script `log()` output to the Console (observation 2)?
3. **Data binding:** should the Inspector name the clip, for example "0 · move"? That needs the
   cooked model's clip table in the snapshot, which does not exist today.
4. **For the director:** none.
