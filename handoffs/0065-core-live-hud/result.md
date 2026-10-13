# 0065 Core Sample live HUD and restart presentation — result

## Status

**Done for the Mac offscreen presentation scope.** The restart action is now
discoverable, and the live HUD was reviewed at rendered pixels on the real
controller routes.

- **Restart hint.** The controls legend reads "WASD move · Space jump · R play
  again" (Arabic «… · إعادة اللعب R», Chinese "… · R 再玩一次", pseudo-locale
  derived). It ships through the existing string table and the public
  `scene.py` / `incant_cmd` path. No technical text, IDs or phases appear in
  the UI.
- **Narrow widths.** When status and progress no longer fit on one row, the top
  bar wraps progress onto a second row instead of wrapping labels inside their
  columns.
- **Live review.** Offscreen captures on Apple M5 Pro / Metal ran the
  build-embedded controller on the accepted numerical routes. They cover:
  - pickup and damage changes;
  - the locked exit;
  - win and loss;
  - return to the initial HUD after both R and gamepad Start;
  - Arabic, Simplified Chinese and pseudo-locale.

  The frames after R and after Start are **bit-identical** to a fresh start.
- **Not claimed.** No phase gate, device, minimum-OS, performance or
  other-backend result. Root reviews the numerical evidence separately.

## Model and effort

Claude Opus 5.5 (`claude-opus-5-5`), thinking effort **max**, for the whole
session. Root's ACP record `~/.codex/incant-evidence/claude-0065-acp.log`,
line 3, reads "Claude session verified: Opus 5.5; thinking: max". I cannot
inspect the ACP transport myself.

## Brief

Brief SHA-256 `667c75db…6123b779`. It has no priority revisions and was
unchanged at lease time, before the first frames, and before commit. It is
root's file and I did not edit it.

## What changed

| File | Change |
|---|---|
| `samples/core/ui/strings.json` | `hud.controls` gains the restart item in en, ar and zh-Hans |
| `samples/core/ui/hud.json` | Top bar (2) `wrap: true`, gap `[24, 6]`; status (3) and progress (6) `grow: 1`; one sentence added to `about` |
| `samples/core/art/fonts/CoreSampleSC-Regular.ttf` | Regenerated subset: 42 glyphs (adds 一 再 次 玩), `7aeb899a…` → `8d78a4c4…`, 13,232 bytes |
| `samples/core/art/fonts/provenance.json` | New strings, font and deterministic-rerun hashes and size |
| `samples/core/PRESENTATION.md` | Legend, wrapping rule, glyph count, integration-contract line, measured limits |

Strings (before → after):

```
en       WASD move · Space jump               → WASD move · Space jump · R play again
ar       الحركة WASD · القفز Space            → الحركة WASD · القفز Space · إعادة اللعب R
zh-Hans  WASD 移动 · 空格键 跳跃               → WASD 移动 · 空格键 跳跃 · R 再玩一次
```

Unchanged, with SHA-256 equal to HEAD (35 files, `evidence/sources.json`):

- `gameplay.ts`, bindings, oracle, playtest, verify and test helpers;
- routes, `scene.py`, `hud.py` and the art generators;
- the bindings spike and STATUS.md;
- every crate (`git status -- crates` clean).

Widget, entity and asset IDs and the binding structure are unchanged: still 12
widgets with the same roles.

## Design decisions

**Wording.** R and Start restart only after a result; the controller ignores
them while playing. "Restart" in an always-visible legend would promise a
mid-run restart that does nothing. "Play again" names exactly what the key
does when it works, and stays concise:

- Arabic uses the verbal noun «إعادة اللعب», matching «الحركة»/«القفز». The
  paragraph still starts with Arabic, so it resolves RTL.
- Chinese uses 再玩一次.
- The legend stays keyboard-only, like move and jump; there is no
  input-device switching.

**Placement.** The legend keeps its 0060 place in the top-right deep-sky band.
There, every one-row top-bar label in every reviewed frame measures 4.61:1 or
better (68 measurements). A prompt under the result title would sit where the
eye goes at the end of a run, but it cannot be built today:

- the controller only reparents the won/lost labels;
- labels cannot have children;
- a new label breaks the accepted oracle (`verify_hud` sees an extra
  declaration, and `ship` asserts 12 widgets).

See the binding request below.

**Narrow widths.** The restart item makes the English legend 224 pt instead of
147 pt, so the one-row top bar now needs about 491 pt (pseudo-locale about
700 pt).

- **Before:** an overflowing row shrank both columns and wrapped labels
  internally. That produced the orphaned-word defect I fixed in 0060.
- **Now:** the bar wraps, and both columns grow, so the one-row look is
  identical when it fits. When it doesn't, the whole progress block drops to a
  second row, still end-aligned, and every label stays on one line.
- **Coverage:** checked in every locale at 960, 640 and 420 pt (12 layout
  snapshots). Arabic and Chinese stay on one row at 420 pt.
- **Trade-off:** see Limitations.

**Two-line legend, considered and rejected.** It would avoid stacking, but
labels have no text alignment. The second line ("R play again") would hang
inset from the right margin in the default view. I kept the default view clean
and put the trade-off at narrow widths.

**Font.** I regenerated the subset with the unchanged recipe, the pinned
upstream (`a3041811…`) and fonttools 4.60.1 (wheel `a884aef0…`) in an offline
venv.

- Two runs give the same `8d78a4c4…`.
- As a control, the same environment reproduces the committed `7aeb899a…`
  exactly from HEAD's strings (`evidence/font.json`).

## Rendered review

**Route.** `tools/live_hud.rs`, a packet-only adapter, is copied into
`crates/incant_game_player/examples/` only while it builds. The copy is
hash-checked against the frozen source `ed75e614…` and removed, and `crates/`
is verified clean.

It runs each variant's build-embedded program on the composed game with the
GpuRenderConsumer offscreen (Apple M5 Pro, Metal, `Rgba8UnormSrgb`, scale 1).
It follows the ordinary player's offline route:

- recordings are indexed by host step;
- a restart request is accepted after the complete tick through the public
  `HostedOfflineGameSession::restart_from` with fresh host settings;
- held input crosses the replacement under a mirror of the player's
  crate-private `restart/input.rs` fence (source hash recorded).

It only reads the public snapshot: UI owner, render input, consumer stats,
logs and world tick. It never edits gameplay, the HUD or input.

**Numerical identity of the captured runs** (`tools/route_check.py`):

- **Routes.** All 8 routes are generated by `binding_oracle` and are
  byte-identical to the inputs in Astra's accepted `shipping-003` proof.
- **Logs.** In all 19 dry and frame runs, each generation's game logs equal
  the accepted run, or its exact prefix where a capture run stops early.
- **PCM.** The SHA-256 over every host step equals the accepted PCM bytes for
  the same steps. Restart boundaries equal the accepted session record (host
  step 2156, generation 1, 1,724,800 frames).
- **Labels.** Every captured label equals `binding_oracle.labels()` for the
  logged state, with the engine-localized text. Won/lost sit in Result only in
  their own phase.
- **Restarts.** Every post-restart snapshot equals the fresh snapshot at the
  same simulation tick (widgets, layout, drawn text).
- **Pixels.** Fresh start (`review-en-start`, host 2), after Start
  (`frames-en-route-a`, host 2158) and after R (`frames-en-route-b`, host 902)
  are all `20ec7c5c…`, from three separate processes.
- **Checker sensitivity.** Ten mutations (stale hint text, wrong title parent,
  shifted or dropped log, PCM, restart mismatch, two-line label, off-edge,
  missing glyph, overlap) are each detected. The unmutated copies pass.

**Critique, state by state** (frames in `screenshots/`, unedited):

- **Start** (01, 08). The top bar reads cleanly in the sky band at 960×540
  and at the default 640×480. The new item is legible at 1×; see the 4× crop.
  At 16:9 the right canyon rim rises behind "again", but it is darker than the
  sky, so contrast holds (5.18:1).
- **Pickup** (02). The counter changes to 1/4 the tick after collection, and
  the stand beside the robot is empty.
- **Damage** (03). The pips drop to ● ● ○ as a mite touches the robot. The
  hollow pip is clearly distinct at 22 px. The hit has no in-world feedback;
  the controller has no flash or knockback.
- **All collected** (04). "Samples 4/4" and "Return to the lift" both change
  on the fourth pickup.
- **Locked exit** (07). The robot stands on the lift pad, alive, with 3/4 and
  the objective still "Collect the core samples". There is no title, so the
  HUD reads truthfully as "not yet".
- **Won** (05, 10, 12, 14). The weakest moment. The 40/48 title sits on the
  pale horizon haze, and the headframe's dark beams cut through it (2.72–3.11:1
  by estimate). The objective still says "Return to the lift" beside "Samples
  secured", because the controller keeps the exit objective after the win.
- **Lost** (06, 16). "Surveyor down" with empty pips. The legend's "R play
  again" is visible in the result state, but away from the title where the
  player looks.
- **After R / Start.** These frames are pixel-identical to 01, so they are
  not duplicated.
- **Arabic** (09, 10, crop). The HUD mirrors fully. The legend shapes
  correctly and orders each key after its Arabic action in RTL reading.
  Digits stay Latin, as in 0060.
- **Chinese** (11, 12, crop). 再玩一次 renders from the new subset. At 1×,
  12 px complex Han glyphs (键) are near the practical floor; the 2× display
  renders them crisply. The size is unchanged from 0060.
- **Pseudo-locale** (13, 14). The text is complete with accents and brackets.
  At 640 pt the progress block takes its own row (13). After the fourth
  pickup the shorter exit objective fits one row again (14).
- **420 pt** (15, 16). The stacked layout keeps every label on one line. Its
  second row leaves the sky band and sits on haze and hoodoos.

**Contrast estimates** (`tools/contrast.py`): WCAG ratio of the theme text
(236, 238, 242) against the median luminance of each label's box. Glyphs cover
a minority of each box. This is an estimate for comparing placements, not
conformance.

| Placement | Range | Count |
|---|---|---|
| One-row top bar, all locales | 4.61 – 12.88:1 | 68 |
| Stacked first row | 4.00 – 12.53:1 | 8 |
| Stacked second row (samples, legend) | 2.65 – 3.93:1 | 8 |
| Result titles over the horizon | 2.72 – 3.17:1 (four of six below 3:1) | 6 |

## Commands and results

| Command | Result |
|---|---|
| `tools/lease.py acquire` | Lease on grant `e333ab28…` at 02:00:27Z, nothing on the target |
| `lease.py cargo build --profile ci -p incant_headless --bin incant_headless` | exit 0, 58.5 s; frozen `410ce655…` |
| `scene.py OUT --binary … --art 0060/v13 --locale L --state playing` × 4 | First en attempt exit 1 (no root `node_modules`; retained). Then en, ar, zh-Hans, en-XA: valid, 0 diagnostics, Undo/Redo/reopen, full glyph coverage |
| `(cd samples/core && python3 -B -m unittest test_bindings test_playtest)` | 16 tests OK |
| `node node_modules/typescript/bin/tsc --noEmit -p samples/core/tsconfig.json` | exit 0, no output |
| `verify_bindings.py prepare builds/en-playing checks/bindings-en --binary …` | passed. 15 native processes, 3 public refusal controls, cue isolation, live HUD readback of the new hint at tick 1 in all four locales. win/lose/locked/idle/win-tail PCM, logs and observations equal Astra's accepted `native-003` |
| `playtest.py builds/en-playing checks/playtest-en --player packages/en/player` | passed: win 2066, loss by contact, locked exit alive 3/4 for 30 ticks, authoring unchanged |
| `tools/build_variant.py VARIANT PROJECT OUT [--clippy]` × 4 | First attempt refused before any child (system Python 3.9; retained). Then all four exit 0. Clippy `-D warnings` on the adapter exit 0. Startup checks pass. `crates/` clean |
| `tools/routes.py routes --accepted …/shipping-003` | 8/8 byte-identical |
| `tools/capture.py` layout ×12, dry ×7 | 0 frames; first layout run exit 1 (missing `captures/` parent; retained) |
| `tools/capture.py` review ×4, frames ×8 | 21 frames, ≤ 960×540, slowest route 4.05 s, slowest capture 0.017 s |
| `route_check.py` / `layout_check.py` / `contrast.py` / `mutate_checks.py` | all pass; 87 layout states; 10/10 mutations detected |
| `lease.py release` | 02:20:00Z: 49 children (32 GPU), no timeouts, every group reaped, no owned process or target user |
| After release: `acquire`, `cargo`, `run`, `capture.py`, `build_variant.py` | each exit 3 "lease was released", nothing created |

Every child, its argv, exit status, timing and log hashes are in
`evidence/ledger.json` (ledger `5ae8d09f…`).

## Identities

| Item | Identity |
|---|---|
| Source | HEAD `d58c45b5` (packet commit over `a9e26999`), tree `a5f55d58`, `Cargo.lock` `c6831578…` |
| Host | macOS 27.0.1 (26A434), Mac17,9, Apple M5 Pro |
| GPU | Apple M5 Pro, Metal, IntegratedGpu, `Rgba8UnormSrgb`; offscreen only |
| Toolchain | rustc/cargo 1.99.0, `ci` profile, `-j2`, `--locked` (`--offline` for packages); Python 3.14.8 and 3.9.6; Node 22.23.3; TypeScript 5.9.3 |
| Target | `~/.codex/incant-core-presentation-target`, exclusive under the grant, left as warm cache |
| `incant_headless` | `410ce655…`, 60,497,696 bytes |
| Packages (`project,render-gpu`, ci) | per-variant player, adapter and embedded-program hashes in `evidence/packages.json`. The gameplay bytecode, bundle `68ca3b91…` and `gameplay.ts` `ff6158f6…` (Astra's accepted controller) are identical across variants |

## Resource use and release

- **Grant and lease.** I verified the grant chain first: 0064's
  `gpu-released.json` and `target-lease.json` hash to the values in
  `review-ready.json`. Astra's file was only read. The lease rules are in
  `evidence/identities.json`.
- **Bounds.** Every GPU child held an exclusive lock, so groups were serial.
  The route bound was 300 s (the slowest took 4.05 s), and every capture
  finished far under 120 s. 21 of 24 frames were retained, each at most
  960×540. There was no native window, desktop or browser capture, device
  audio, credential or account.
- **Release.** At 02:20:00Z. Every tool refuses afterwards. The worktree-root
  `node_modules` link (to the parent checkout's, identical
  `package-lock.json`) was removed.
- **Kept, not deleted.** The raw frames, builds, packages, binaries, logs and
  the offline venv stay in `~/.codex/incant-evidence/0065`. The 0060 evidence
  (art v13, font inputs) was only read.
- **Retained failures:**
  1. `scene.py` without the root `node_modules` link (exit 1);
  2. `build_variant.py` under system Python 3.9 (refused before any child);
  3. the first layout capture (exit 1, `captures/` parent missing).

  After release I made `capture.py` and `build_variant.py` check the guard
  first: `capture.py` had refused on "output exists" before reaching it, and
  `build_variant.py` would have copied the adapter into `crates/` before its
  first guarded call.

## 0060 records

The historical 0060 offscreen-helper discrepancy (`sources.json` claims
`d0863272`, the committed source hashes to `9f453fd0`) and the old false
locked-route result stay as they are; this packet does not relabel them. New
acceptance comes only from this packet's execution:

- the adapter source was frozen at first build (`evidence/freeze`) and
  hash-checked before every build;
- every adapter binary is checked against its package record before every run;
- the locked route is the corrected `routes/locked-input.json` (alive, 3/4,
  30 ticks in the exit).

## Findings and requests for Astra (not edited)

1. **Binding request: a result-state prompt.** For "R play again" to appear
   under the title at the moment it applies, the controller would need an
   optional HUD role (for example `prompt`) reparented with the result titles.
   The oracle's label set and the `ship` widget count would need the matching
   update.
2. **Objective after a win.** The objective keeps "Return to the lift" beside
   "Samples secured" (05). Clearing or switching it at the terminal phase
   would read better.
3. **Titles over the horizon.** They measure 2.72–3.17:1. Layout alone cannot
   fix this: raising the title crowds the objective, and lower is brighter
   sand. A theme-level text outline or scrim in the renderer would (as 0060
   finding 8 noted).
4. No runtime defect was found. The GPU-hosted routes reproduced the accepted
   CPU logs, PCM and restart boundaries exactly.

## Limitations

- **Narrow widths.** Below about 491 pt (English) or 700 pt (pseudo-locale)
  the stacked second row leaves the sky band and measures 2.65–3.93:1. Between
  about 442 and 491 pt, English returns to one row once the shorter exit
  objective appears, a one-time reflow at the fourth pickup.
- **Keyboard-only legend.** Gamepad Start works but is not named.
- **Untested here:**
  - the source-free shipping export (`verify_bindings.py ship`; this change
    alters only copy, layout and font bytes);
  - the native window;
  - audible device output;
  - other backends;
  - minimum OS, phone and performance.
- **Contrast figures** are estimates against each label's median background,
  not conformance measurements.
- **Scope.** Mac presentation review only. The capture adapter is a packet
  tool, not an engine interface.

## Changed paths

- `samples/core/ui/strings.json`, `samples/core/ui/hud.json`,
  `samples/core/art/fonts/CoreSampleSC-Regular.ttf`,
  `samples/core/art/fonts/provenance.json`, `samples/core/PRESENTATION.md`
- `handoffs/0065-core-live-hud/tools/`:
  - `lease.py`, `build_variant.py`, `capture.py`, `plans.json`, `routes.py`;
  - `live_hud.rs`, `png.py`;
  - `route_check.py`, `layout_check.py`, `contrast.py`, `mutate_checks.py`,
    `collect.py`.
- `handoffs/0065-core-live-hud/evidence/` (9 JSON files),
  `handoffs/0065-core-live-hud/screenshots/` (16 frames and 3 crops),
  `handoffs/0065-core-live-hud/result.md`

## Screenshots

All are unedited PNGs of the retained RGBA (`tools/png.py convert`). Sizes,
hashes and source frames are in `evidence/screenshots.json`.

- `01-en-960x540-start.png`: fresh start; identical to the frames after Start
  and after R
- `02-en-960x540-pickup-1.png`: first sample, 1/4
- `03-en-960x540-damage.png`: first hit, ● ● ○
- `04-en-960x540-collected.png`: 4/4, Return to the lift
- `05-en-960x540-won.png`: win at the lift
- `06-en-960x540-lost.png`: loss, empty pips
- `07-en-960x540-locked-exit.png`: alive at the lift with 3/4, no win
- `08-en-640x480-start.png`: default window size
- `09-ar-640x480-pickup-1.png` and `10-ar-640x480-won.png`: Arabic
- `11-zh-Hans-640x480-pickup-1.png` and `12-zh-Hans-640x480-won.png`:
  Simplified Chinese
- `13-en-XA-640x480-pickup-1.png` and `14-en-XA-640x480-won.png`:
  pseudo-locale, stacked and one-row
- `15-en-420x320-start.png` and `16-en-420x320-lost.png`: 420 pt
- `crop-*-progress-x4.png`: 4× nearest-neighbour review crops of the legend
  in en, ar and zh-Hans. They are labelled as crops and never replace a frame.

## Open questions

- Should the controller gain a result-state prompt role (finding 1), or is
  the legend the intended home for session controls?
- Should the game-UI theme gain a text outline or scrim? It would fix both
  the result titles and the narrow-width row.
- Should the legend name gamepad controls when a pad is connected? That
  would need input-device-aware copy.

## Publication correction by Astra

After GPU release, Astra restricted the runner and collector to an explicit
build-variable allowlist. The published ledger is a sanitized derivative of the
private original, whose hash is retained. No execution, source adapter, numeric
result or screenshot changed. The original unpublished packet commit is excluded
from publishable ancestry. See `evidence/environment-redaction.json`.
