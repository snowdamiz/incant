# 0061 result: Pearl Cove, the playable 2D sample presentation

## Status

- **Offscreen scope: complete.** Original art, the builder, the presentation
  guide and offscreen review through Incant's actual renderer are done. The
  review covered the authoring renderer and the native retained runtime renderer.
- **Native-window review: pending.** 0060 still held the desktop when this work
  was ready (`~/.codex/incant-evidence/0060/released` was absent at 20:45Z and
  0060 was logging bursts). I opened no window and captured no screen. The
  precise pending item is below.
- No feature, device or phase gate is approved by this handoff.

Model: **Claude Opus 5.5 (`claude-opus-5-5`)**. Thinking effort: **max**. This
was one ACP handoff session; the root holds the ACP-side verification record.
Brief applied: `handoffs/0061-planar-sample-presentation/brief.md`, SHA-256
`e30ae96ddd1cec17dea4b5c6fb7be97abafc60db23940ce9cdceb4474fdadeed`. It had no
appended revisions during the session, and it remains unchanged and uncommitted
by me.

## What was built

A small low-tide cove in 16 px pixel art. A beachcomber in a red raincoat
collects three pearls and reaches the moored rowboat. Sea urchins sting. The
visual rules are:

- Walkable sand has the highest values. Solid moss-topped basalt, stone
  boulders, sea and the tide pool are darker and outlined.
- Only pearls are white, only urchins are dark purple and spiky, and only the
  player is saturated red. The boat shares that red as the goal.
- Every color stays in the tone map's identity range (≤ 231 per channel), so
  rendered pixels equal the palette.
- The camera is a fixed `pixel_perfect` 320 × 180 at 16 px/m, integer-scaled
  and letterboxed.

Iteration was driven by actual renders: `screenshots/14-…` is the first engine
render and `15-…` a mid-point. I fixed these defects:

- a bright lawn that out-valued the beach;
- cliff faces that read as masonry, then as glyphs;
- boulders that looked like concrete blocks;
- a square "swimming pool";
- a pier that read as a ladder;
- a clam that read as a jar;
- the editor placeholder cube on the controller;
- a moss motif repeating on the 16 px grid;
- a pearl twinkle hidden on white.

| Path | Purpose |
|---|---|
| `samples/planar/art/` | `palette`, `image` (canvas, PNG writer, seamless noise), `terrain` (neighbour-aware autotiles), `sprites`, `cove` (map, IDs, cell assignment), `build` (sheets and atlas descriptor) |
| `samples/planar/scene.py` | Public import plus three RPC command transactions; Undo/Redo, save, journal reopen, validate; `--play` |
| `samples/planar/playtest.py` | Seven native-play scenarios on this layout |
| `samples/planar/PRESENTATION.md` | Direction, layers, sprites, IDs, controls, integration contract, budget |
| `samples/planar/README.md` | One paragraph pointing at the presentation; open items stay listed |
| `tools/tests/test_planar_presentation.py` | Identities, art bounds and tone-map range, layout and route clearance, controller contract |
| `handoffs/0061-…/tools/` | `guard.py` (lease), `package.py`, `review.py`, `collect.py`, `mutate.py`, `pngtool.py` |
| `handoffs/0061-…/screenshots/`, `evidence/` | Outputs below |

`gameplay.ts`, `verify.py`, `export_verify.py`, `tsconfig.json`, CI, engine,
ABI, save, solver and compiler code are untouched.

## Implementation contract for Astra

- **Authoring.** `scene.py` creates everything through `incant_headless import`
  and RPC `command.execute`. The history is `Import cove.atlas` (import origin),
  then three `editor` transactions: controller and input (3 commands), terrain
  and camera (2), actors and controller (13).
- **IDs.** IDs are readable canonical ULIDs (`01PSEN…` entities, `01PSTM…`
  layers, `01PSSC…` script). Atlas frames are `01PSFR` plus a hash of the frame
  name, so they stay stable. The importer assigns the texture asset ID; each
  build records it.
- **Controller props.** Only the source's props are used: `player`, `exit`,
  `pickups` (3), `hazards` (3) and `speed: 3`. There is no camera prop, by design.
- **Roles.** The player is a dynamic, zero-gravity body with `Collider2D` r 0.30
  and `Velocity`. Pearl sensors are r 0.35, urchin sensors r 0.40, and the boat
  exit sensor r 0.40, placed on the pier's seaward edge. All roles are root,
  unit scale, and pearls have no children. The water and rock tile layers are
  solid; the pier layer is not.
- **Input.** `move` is axis2: WASD, the arrow keys, the left stick with
  `invert_y` (up is positive) and the d-pad.
- **State frames.** These are ready now, with no engine change. The player has
  a paused flipbook: 0 default, 1 stung, 2 cheering. The boat has one: 0
  lantern dark, 1 lit. Seek them with `context.commands.controlFlipbook(id,
  'seek', n)` on these transitions: all pearls collected → boat 1; won → player
  2; lost → player 1. Frame 0 is exactly what the current controller shows.
- **HUD.** It is not authored. A scene with UI nodes cannot run in headless
  native-play, and no script API mutates widgets. PRESENTATION.md specifies
  widget IDs `01PSUI…01`–`04`, string table `01PSST…01`, five `en` strings and
  the exact transition table, for binding once a bounded text-binding command
  exists.
- **Restart.** Add a `restart` button action (KeyR, Enter, gamepad Start and
  South), accepted only when won or lost. It needs a session reset that
  rebuilds the world from the cooked scene at the next sync point. The reset
  clears `context.state`, respawns pearls and resets the flipbooks.

## Commands and results

Every build and run went through `handoffs/0061-…/tools/guard.py`. The guard
used a packet lease (`evidence/lease.json`), `./tools/cargo --locked --profile
ci -j 2`, Rust 1.99.0 and `CARGO_TARGET_DIR=/Users/andreyyurlov/.codex/incant-core-presentation-target`.
`evidence/ledger.jsonl` holds 105 entries with each argv, exit status and input
hashes.

| Step | Command | Result |
|---|---|---|
| Headless build | `guard.py cargo build -p incant_headless --bin incant_headless` | exit 0, 1m26s; `incant_headless` `e60b5132…0f909bba` |
| Frames tool | `guard.py cargo build -p incant_render --example native_frames` | exit 0, 2m41s; `8786a91d…3b2dbc16` |
| Review player | `handoffs/0061-…/tools/package.py` (`-p incant_game_player`, features `project,native-window`, `INCANT_NATIVE_PROJECT`) | exit 0, first build 3m12s; startup check passed; final player `569c6973…b96926d9` |
| Full review | `python3 -B handoffs/0061-…/tools/review.py artifacts/0061/review-5` | exit 0; collected by `collect.py` |
| Unit tests | `python3 -B -m unittest tools/tests/test_planar_presentation.py` | 4 tests OK (also on Python 3.13; syntax parses for 3.9) |
| Mutations | `python3 -B handoffs/0061-…/tools/mutate.py` | 5 of 5 detected (urchin on the route, pearl in a boulder, a color above 231, player gravity, solid pickup); unmutated suite passes; every file restored byte-exact |
| Tool suite | `python3 -B -m unittest discover -s tools/tests` | 249 tests OK, 4 skipped |

Inside `review.py`, run on HEAD `2b6eabc4`:

- **`scene.py --play`.** The import, three transactions, three undos (back to
  the initial project, with no `gameplay.ts` on disk) and three redos all match.
  Save, journal reopen (identical `project.read`) and `validate` (valid) pass.
  Seven scenarios pass:

  | Scenario | Ticks | Result |
  |---|---|---|
  | keyboard, arrows, left stick and d-pad wins | 781 each | pearls A, B, C in order; one win; stopped at (10.5, 2.6); logs, observations, PCM and entity count identical |
  | urchin loss | 201 | lost at (13.5, 4.1) |
  | locked boat | 181 | no win at (10.5, 2.3) |
  | shore then cliff | 221 | stopped at (6.5, 9.7) |

  Authored file hashes are identical before and after Play
  (`play_left_project_unchanged: true`).
- **Renders.** `incant_headless screenshot` at 320×180, 1280×720, 1920×1080,
  1280×960 and 1920×887 on adapter `Apple M5 Pro`. Geometry: 1 tilemap, 455
  tile cells, 12 sprite entities, 0 diagnostic entities.
- **Parity.** Authoring 960×540 against the native retained frame at tick 0:
  **0 of 518,400 pixels differ**.
- **Native sequence.** `native_frames` captured 32 ticks of flipbook motion and
  hook seeks at ticks 1–2, which selected player frame 1, then player 2 and
  boat 1 (`evidence/native/hooks-report.json`).
- **Observed state.** A truncated keyboard-win run asserted pearls A and B
  collected and C present at tick 450. The player was at (16.95, 8.5).

Two development runs failed and were fixed, both recorded in the ledger. The
first `scene.py` assumed the importer keeps the descriptor's atlas ID; it
assigns its own. `review-1` requested a 2532 px screenshot; the tool's limit is
1920 × 1080.

Not run: `tsc` (`gameplay.ts` is unchanged) and `cargo fmt`/`clippy` (no Rust
changes). No device, window, capture or performance measurement was made.

## Screenshots

All are actual Incant renderer output on adapter Apple M5 Pro, offscreen.
"Authoring" means `incant_headless screenshot`; "native" means the retained
native runtime path (`native_frames`, cooked package data).

| File | Content | SHA-256 |
|---|---|---|
| `screenshots/01-cove-1280x720.png` | Authoring, 4×, initial state | `8e92b4e9…32b4db` |
| `screenshots/02-cove-logical-320x180.png` | Authoring, 1× logical frame | `377f7d97…5cda96` |
| `screenshots/03-window-4x3-1280x960.png` | 640×480 pt window shape: 4× with backdrop bars | `350ab4ca…fa5ed8` |
| `screenshots/04-phone-landscape-aspect-1920x887.png` | iPhone 13 landscape aspect at the tool's cap (the device itself computes 6×) | `4777408a…229909` |
| `screenshots/05-native-runtime-tick0-960x540.png` | Native runtime frame, identical to authoring | `dbccedc6…aa631a` |
| `screenshots/06-native-hook-stung-960x540.png` | Native: player flipbook sought to "stung" | `507cd015…9b0f8d` |
| `screenshots/07-native-hook-cheer-lantern-960x540.png` | Native: "cheering" plus the boat lantern lit | `c8cc89cd…c60d33` |
| `screenshots/08-state-two-pearls-1280x720.png` | Observed tick-450 state | `b31e5080…5e3811` |
| `screenshots/09-state-won-with-hook-frames-1280x720.png` | Observed win position with hook frames | `bf2924fb…858d0a` |
| `screenshots/10-state-lost-with-hook-frame-1280x720.png` | Observed loss position with the hook frame | `2d6f877e…ee412a` |
| `screenshots/11-native-motion-pearl-urchin-ticks.png` | Native ticks 0–62: pearl twinkle and urchin sway | `2cdb9f76…c25b70` |
| `screenshots/12-art-sprites-x6.png`, `13-art-tiles-x3.png` | Source sheets (checkerboard means transparent) | `fdd23638…ecb3fe`, `38698ee5…112dd8` |
| `screenshots/14-iteration-v1-first-engine-render.png`, `15-iteration-v4-before-moss-and-sparkle-fixes.png` | Earlier iterations, for comparison | `01d800e9…c98bf5`, `69248ba5…9dd082` |

Screenshots 08–10 are authoring stills of observed states, reconstructed on
scratch copies through public RPC commands. A flipbook's first frame overrides
the sprite region, so those copies drop the flipbook to select a frame. They
are not live frames, and the hooks are not implemented in the controller.

## Measured budget (desktop, not reference-phone)

- One 200 × 144 RGBA8 sRGB atlas page with 8 mips and 88 frames (73 tile, 15
  sprite), 153,972 B KTX2 per color role.
- Source sheets: 3,317 B and 997 B PNG.
- 455 tile cells in 5 layers, 12 sprite entities, 79 flipbook frames.
- `world.scene` 14,136 B; 331,063 B of package data in total.

## Leases, processes and cleanup

- **Lease.** `artifacts/0061/lease.json` (copy in `evidence/lease.json`) ran
  from 2026-10-12T20:05:13Z with a 12-hour expiry. It was released at
  **20:47:05Z** (`evidence/release.json`). After release the guard refused
  `check`, `cargo` and `run` (exit 3).
- **Target.** The warm target remains as cache; nothing was deleted from it. My
  build replaced its `ci/incant_headless`. The previous binary's hash,
  `d1855c6a…82fa5e`, was built from the parent checkout and is in
  `evidence/warm-binary-before.txt`.
- **No capture lease.** No window was opened and nothing was captured.
- **Processes.** No owned process remained. Every child ran to completion
  under `subprocess.run`, and `pgrep` showed the target idle before release.
- **Scratch.** The worktree-root `node_modules` link I created for the
  TypeScript cook (lock hashes verified identical first) is removed. Scratch
  stays in the ignored `artifacts/0061/`, including the `review-5` window
  package for the pending item.

## Pending native-review item

When `~/.codex/incant-evidence/0060/released` exists:

1. Acquire a fresh packet lease and capture lease; the old guard refuses by design.
2. Check that the session is unlocked and has had at least 20 s of HID idle.
   Record the frontmost app.
3. Rebuild the package with `python3 -B handoffs/0061-…/tools/package.py
   <scene-output>/project/planar.json <new> --camera 01PSEN00000000000000000003`.
4. Launch `incant_game_player --package <new>/player/data --window` as a
   supervised child.
5. In short focused bursts, drive the `playtest.WIN` route with key events.
   Take window-only captures (`screencapture -l <window>`) at:
   - startup;
   - after the west pearl;
   - the east alcove (two pearls gone);
   - the boat.
6. Resize to a 16:9 and a 4:3 frame to confirm integer scaling and letterbox
   bars.
7. Close through the AX close button to get the run report. Restore focus,
   reap, and release both leases.

Acceptance means:
- pixels match screenshots 01, 03 and 08 at equal scale, allowing for Display
  P3 capture tagging;
- flipbooks animate;
- the player stops at rock and shore;
- the win stops the player at the pier end.

## Remaining gates and limitations

- Native-window acceptance, above, is pending.
- The HUD, state-frame seeks and restart need Astra's controller and engine
  hooks. Until then the game has no on-screen text, and a win or loss is shown
  only by the player stopping.
- Results are offscreen on a local Mac. Device and performance gates (iPhone 13,
  Pixel 6, M1 Air, Windows laptop) are untouched. The review player is
  ci-profile, not the audited shipping export, which Astra runs separately.
- Phone scales are computed from `PixelGrid::fit`, not rendered at device size.
  Portrait phones letterbox to 3×.
- There is no audio. The follow camera is unused by design. Physics save/load
  is not claimed.

## Open questions

1. Should the HUD bind through a native `setUiText`-style command, or through
   declarative UiText-to-string-table bindings?
2. Should restart be a scene-level host reset (my preference) or a
   script-level respawn API?
