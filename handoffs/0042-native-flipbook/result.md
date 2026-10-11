# 0042 native flipbook playback appearance review

## GPU slot

**All capture processes of this packet have ended. The last one ended at 2026-10-11
01:20:51.630 −04:00 (05:20:51.630 UTC).** That was `subset-source-free`, the 19th and final
`capture.py` invocation; every invocation exited 0. None of my processes remain.

- Astra's `artifacts/tools/gpu-review-release.json` (verified 05:23:00 UTC) records the
  release: no active tool processes, gate removed under the capture lock. The gate file is
  now absent.
- Everything after 01:20:51 −04:00 ran on the CPU against existing images: analysis,
  figures, evidence and this report.
- At 05:28:58 UTC, `ps` showed `tools/probes/native-game.py … --gpu` (PID 21741). This
  packet did not start it, and it began after the release. I did not interact with it.

## Verdict

**Accepted for flipbook-specific appearance, on one adapter (Apple M5 Pro) and one path
(the bounded `native_frames` helper). I found no flipbook-specific defect.** This is not a
performance, phone, shipping, interactive-player, editor or phase-gate result.

- **Static controls.** Animated frames are byte-identical to independently authored
  static-frame control scenes at every compared moment: 11/11 pixel-perfect and 4/4
  orthographic 2.5×. Even the PNG files are identical. Each control replaces every
  flipbook with a plain `SpriteRenderer` showing the contract's frame for that tick. It
  keeps all entity IDs and places moving entities at their exact f64 positions.
- **Fixed-position entities.** 850/850 entity crops at the other ticks equal a control
  that shows the same frame.
- **Playback state.** The sidecar state matched an independent closed-form model of the
  brief's rules in 2,640 of 2,640 states, with 0 mismatches. 2,064 uncontrolled states
  also match exact `Fraction` arithmetic. Binding and topology generations stayed 0 in
  every capture, so there was no per-tick rebinding, including on control ticks.
- **Independent pixel model.** A CPU model built from the written contract and the source
  PNGs (not the cooked atlas or engine code) predicts 5,741,176 of 5,741,176 compared
  pixels across 28 ticks. Linear, nine-slice, rotated, sheared and tinted entities are
  masked out of this comparison.
- **Nothing outside the moving content changes.** Across 28 captures, 33,696 pixels change.
  All of them lie inside the union of animated and moving entities' quads (2 px dilation).
  The other 167,350 pixels never change. There are no trails, stale geometry, crop shifts
  or order changes.
- **Frame identity from pixels.** Each frame's unique interior colour identifies it. The
  identified frame equals the model and the report in 1,478/1,478 visible cases. Another
  frame's interior colour never appears inside an entity.
- **No bleeding.** 0 of 63 captures contain magenta (hidden zero-alpha RGB or the sheet's
  opaque guard). Pixel-region frames on an ordinary sheet render identically to the same
  atlas frames: 28/28 ticks nearest, 28/28 linear.
- **Repeatability and cooked loading.** I captured again with fewer synchronisations
  (8 ticks): identical to the main run, PNGs and report entries. I then removed the
  project, journal, PNG and atlas sources and captured again from the cooked scene and
  caches: identical again (8/8). This proves cooked loading only.
- **Cooked atlas page.** The decoded level 0 matches the sources exactly (0 content,
  extrusion, overlap or empty-texel mismatches). All 8 packed origins are nonzero.

## Packet, director feedback and bounds

- **Fresh packet.** The packet held only Astra's `brief.md` and `capture.py`. There were no
  earlier attempt files and no earlier artifacts.
- **No new director feedback.** The current brief has no priority-revision or
  director-feedback section, so I applied the brief and CLAUDE.md as written.
- **Other handoffs.** I read handoffs 0039 and 0032 for patterns only. My helpers import
  nothing from them, and no other handoff's file was modified.
- **Bounds kept:**
  - only the supplied `incant_headless`, `cook_scene` and `native_frames`;
  - every logged call hash-checked before and after;
  - no Cargo, Rust, shader, engine, schema, SDK or editor edits;
  - no headless screenshots, native application launch or system-wide input;
  - GPU work only through `capture.py`, never `native_frames` directly;
  - no budget-file or wrapper edits, accounts, credentials, publishing or merging.
- **Gate.** No capture ran before the gate existed. I read `gpu-review-ready` at 01:19:48
  −04:00. It said: granted_by `/root`, verified 2026-10-11T05:14:16.497505Z, handoff
  `0042-native-flipbook`, source `95df19d…`, packet `262de2f…`, wrapper sha256
  `7ddc1a4e…eacc`, the three tool hashes below, and "All GPU invocations through bounded
  capture.py; remove gate and reap children before release". This is transcribed; the file
  has since been withdrawn. The wrapper hash matched before my first capture.
- **Unlogged calls.** Before my driver existed, I ran five CPU-only help calls by hand:
  `incant_headless --help`, and `--help` for `init`, `import`, `rpc` and `validate`. They
  are not in `engine-calls.jsonl`. The executable hash was identical before (01:00) and
  after.

## Model, effort and provenance

- **Model:** Claude Opus 5.5, model ID `claude-opus-5-5`, as the session reports it.
- **Effort:** max, this session's reasoning-effort setting.
- **Transport:** The prompt arrived in the exact template of `tools/handoff/main.py`
  (lines 235–242). That runner sends a prompt only after `configure_model_and_effort`
  confirms "Opus 5.5" and effort `max` over ACP (`tools/acp/config.json`: `claude-opus-5-5`,
  `max`). I cannot inspect that handshake from inside the session. No other model or
  effort was used.

| Item | Value |
| --- | --- |
| Worktree HEAD | `262de2f9de3487a47c7a79e2c0a76246a2a5077c`. It differs from the binaries' source `95df19d549062538c4cd59d353f26f4e4a8015c1` only in STATUS.md, docs/spikes/native-flipbooks.md, its evidence JSON, and this packet's brief.md and capture.py (no source code) |
| `incant_headless` | 49,019,808 bytes, sha256 `34c0173d5f5d75734c5b94eaa9800752bf81ebac8b4668c48b133bf0f40b32ce` |
| `cook_scene` | 2,765,656 bytes, sha256 `256010202e5b272007bd6a655638985801f128d9188f0f4643723370743aa563` |
| `native_frames` | 18,995,728 bytes, sha256 `02acba93a337220e0f76d069fe0d245129fcdebf7fef552e02438cf2fecbbe12` |
| `binary.json` | all three equal; profile "ci; functional review utility, not shipping/performance" |
| `capture.py` | sha256 `7ddc1a4e18d32e9c91b65ed2e007c6d5a453754ec7481b27a0c7b86246b4eacc`, unchanged, equal to the gate's wrapper hash |
| Hash checks | all 127 logged calls kept every hash before and after (`evidence/engine-calls.jsonl`) |
| Adapter | "Apple M5 Pro", reported by every `native_frames` report |
| Backend | Metal, inferred: `Renderer::headless()` uses `wgpu::Instance::default()` on this macOS host, and the report names only the adapter |
| Host | macOS 26.6 (25G72), arm64; Python 3.14.8, standard library only |
| Tick rate | 60 Hz (project default, echoed by every report) |

## What I built

All authoring went through `incant_headless` `init`, `import`, `rpc` (`project.read`,
`command.execute`, `project.save`) and `validate`. Scenes were cooked with `cook_scene`.
My tools never write a project or journal file. Source PNG/atlas files and request JSON
are the only authored inputs.

### Original art (`helpers/fb_art.py`, committed in `art/`)

Drawn from code for this packet; it regenerates byte for byte.

- **Five animation frames, all different sizes:** f0 12×12, f1 14×11, f2 10×13, f3 15×10,
  f4 11×14.
- **What every frame has:**
  - a fully transparent one-texel outer edge, whose hidden RGB is magenta;
  - an asymmetric one-texel ring: red top, blue bottom, amber left, green right, and
    light-grey, cyan, pink and near-black corners;
  - a frame-unique interior colour;
  - its digit;
  - white 2×2 pips that step clockwise: f0 top-left, f1 top-right, f2 bottom-right,
    f3 bottom-left, f4 all four.
- **Tiles:** t0–t2, 8×8, packed in the same atlas.
- **`sheet.png`:** the same five frames at irregular nonzero origins inside an opaque
  magenta guard, for pixel-region flipbooks.
- **`badge.png`, `plates.png`:** ordinary static textures.
- **Colours** stay within sRGB 0–234.
- **Atlas:** `flip.atlas.json`, padding 1. It cooked to a 55×26 page with 6 mip levels.
  - Fingerprint `a3c678f6f2d60510dc7063fd558ea605dc304cd57045f521bc7893dd46529f0e`, KTX2
    `747d340d…43ae`.
  - Packed origins: f4 (1,1), f2 (14,1), f0 (26,1), f1 (40,1), f3 (26,15), t0 (43,14),
    t1 (14,16), t2 (1,17). All are nonzero.

| Source | sha256 |
| --- | --- |
| f0–f4.png | `b6571e88…9ebf`, `45d0f885…e901`, `2ce55acb…b911`, `d69167e0…1559`, `b4d1ea2b…9212` |
| t0–t2.png | `48022034…c67b`, `7694846…1863`, `a8bcbe23…fda1` |
| sheet.png, badge.png, plates.png | `03ab87e1…89f4`, `9bec75cf…ed85`, `659a0a6e…36c2` |
| flip.atlas.json | `860f3e4b4907c5d7c516907cd7c701ad43f2a07bca60381fc59df6d3f50eba10` |
| art/manifest.json | `212eaedc3954c7705c53fcf89717d05aa1a84e7b6be76bfec2d4fad04cedbfbe`; full hashes in it |

### Scene (`helpers/fb_scene.py`)

There is one scene of 76 entities, 55 of them flipbooks. Two cameras view it:

- `camera-pp`: pixel-perfect, 16 px/m, reference and output 640×360 (1×);
- `camera-ortho`: orthographic, vertical size 9 m (2.5×), offset +4.15/−1.96 px.

The board-A rate is 50/7 fps: 8.4 ticks per frame, with boundaries at ticks 9, 17, 26, 34,
42 and 51. Tick 42 is an exact boundary.

Every animated renderer's otherwise-unused static region deliberately names a different
frame: the t2 stone tile, or the sheet's magenta guard for sheet animations. Showing it
would be visible.

| Board | Content |
| --- | --- |
| A, sampling | nearest; linear; flip x, y and xy; a pixel-region sheet; a reversed frame list; a tint; 1:1, 2× and 3× sizes, nearest and linear; a stretched 3×1.5 m; nine-slice with asymmetric border [3,2,2,3] (nearest, linear, mirrored, ppu 8, and horizontally compressed to 4 px) |
| B, transforms | parent rotated 30°; parent reflected (−1,1) with child flip_y; nonuniform parent (1.5,0.75) with a child rotated 20° (shear, linear); rotated nine-slice; 45° linear; a moving parent (0.75 m/s) with an animated child and a static sibling; a moving sprite (−0.5 m/s); a two-level chain, scale 2 with −15° |
| C, controls | eleven entities, one control script each (table below) |
| D, painter order | a 16×7 two-layer tilemap (ground (0,0) with holes; stones (0,2)); an animated sprite on layer −1 seen through holes; a static, animated, static triple at equal (0,1), ordered by entity ID; an animated sprite on layer 1 under a 50% translucent static badge on layer 2; an animated (0,0) tie with the map (higher ID); a moving (3 m/s) animated (0,3) sprite paused at 25 and resumed at 50 that passes under a layer-2 badge; a static atlas-frame sprite beside a linear animation; 4× nearest, 4× linear and a large linear nine-slice |
| E, clock | rates 150/1, 13/3, 1/7, 1000/1, 60/1 and 7/11; a once sequence at 150/1; single-frame loop; single-frame once |

| Entity | Controls (tick: action), applied after that tick's step |
| --- | --- |
| C1 | 12 pause, 20 play (phase preserved) |
| C2 | 10 pause, 16 seek 3 while paused, 25 play, 41 restart |
| C3 | once; 60 play after completion (restart) |
| C4 | authored `playing: false`; 33 play |
| C5 | 17 seek 3 while playing |
| C6 | 33 restart while playing |
| C7 | 16 pause then restart (same tick) |
| C8 | 16 restart then pause (same tick); 50 play |
| C9 | once; 50 seek 1 after completion; 59 play |
| C10 | reversed list; 17 seek 1 |
| C11 | sheet frames; 9 pause |
| D3 | moving; 25 pause, 50 play |

### Projects, cooking and captures (`helpers/fb_review.py`)

- **Projects.** 12 projects: `anim`, plus one static control per moment t ∈ {0, 9, 17, 25,
  34, 42, 50, 60, 69, 420, 2100}.
- **Per project:** `init`, four imports (atlas, sheet, badge, plates; usage color), one
  `command.execute` of 76 `create_entity` commands, `project.save`, `validate`, then
  `cook_scene`.
- **Outcome.** All 12 validate with 0 diagnostics. Imports report no diagnostics.
- **Cooked scenes.** `anim.scene` is version 8: 24,815 bytes, sha256
  `742d068507f40ee7c703f0b08f23521625dc4386b6ba68c96114b61615c60aaa`, transaction
  `01M4MPDR118897A6SYKNFVRDMD`. Each control is version 6, 18,601 bytes. Hashes and
  transactions are in `evidence/run/build.json`.
- **Entity lists.** All 12 regenerate byte for byte from `fb_scene`
  (`evidence/run/entity-lists.json`).
- **Requests**, 640×360:

  | Request | Ticks | Controls | sha256 |
  | --- | --- | --- | --- |
  | `main` | 28: 0, 1, 2, 3, 8, 9, 10, 12, 16, 17, 20, 24, 25, 26, 33, 34, 41, 42, 50, 51, 59, 60, 68, 69, 419, 420, 2099, 2100 | 21 | `4b741005…0dd9` |
  | `ortho` | 0, 9, 17, 42 | 16 | `d519a785…43ea` |
  | `subset` | 0, 9, 25, 42, 60, 69, 420, 2100 | 21 | `3f7ada5f…e319` |
  | `control` | 0 | none | `d425a7bd…a32c` |

## Results

All values come from `evidence/analysis.json`. Two analysis runs, and a third after a
code-only cleanup, produced byte-identical output.

### Playback state (sidecar reports against the contract model)

`fb_scene.segments_model` is closed form. A segment's cursor is
base + numerator × (ticks since it began), counted in units of 1/(denominator × 60) frame;
controls start new segments. It does not step the runtime's remainder clock. A self-test
also checks it against a separately written stepped reference: 16 cases over 134 ticks.

| Capture | States | Exact-Fraction checks | Mismatches | Generations |
| --- | --- | --- | --- | --- |
| main | 1,540 | 1,204 | 0 | (0, 0) throughout |
| ortho | 220 | 172 | 0 | (0, 0) |
| subset / source-free | 440 / 440 | 344 / 344 | 0 / 0 | (0, 0) |

The evidence covers each control rule
(`figures/controls-timeline-zoom2.png`; P playing, - paused, C completed):

- **Pause keeps the subframe phase (C1).** Paused at 12 in frame 1 with phase 180/420.
  After play at 20, frame 2 appears at tick 25. A phase reset would delay it to 29. Tick
  24 still shows frame 1.
- **Seek while paused (C2).** Frame 3 at tick 16, paused through 24. Play at 25 starts at
  phase 0, so frame 4 appears at 34 (25 + 9 ticks). Restart at 41 gives frame 0 playing,
  then frame 1 at 50.
- **Once hold and completion (C3).** Frame 4 from tick 34, still playing at 41. At tick 42,
  exactly 5 × 8.4, it holds frame 4 and reports completed with playing false. Play at 60
  restarts frame 0, with frame 1 at 69.
- **Seek on a completed sequence (C9).** Seek 1 at tick 50 clears completion but keeps the
  sequence paused (frame 1). Play at 59 resumes from frame 1, without restarting.
- **Same-tick order (C7, C8).** C7, pause then restart, plays: frame 1 at 25. C8, restart
  then pause, stays paused at frame 0 until play at 50.
- **Seek on a reversed list (C10).** Seek 1 shows f3, index 1 of the reversed list.
- **Initially paused (C4).** Holds frame 0 until play at 33, then shows frame 1 at 42.
- **Moving and paused (D3).** The sprite keeps moving while paused. On resume it advances
  one tick later, because its phase was 410/420.
- **Board E clock arithmetic.**
  - 150 fps gives frames 2, 0, 2 at ticks 1–3.
  - 1000 fps gives frames 1, 3, 0.
  - 1/7 fps reaches frame 1 at tick 420 and wraps to frame 0 at 2100 (frame 4 at 2099).
  - The 150 fps once sequence completes at tick 2; the 2 fps single-frame once completes
    by tick 33.
  - At tick 2100, 50 loops later, board A is exactly at frame 0 again, frame 4 at 2099: no
    drift.

### Pixels

| Check | Result |
| --- | --- |
| Moments against static controls, pixel-perfect (t 0, 9, 17, 25, 34, 42, 50, 60, 69, 420, 2100) | 11 / 11 identical, 0 px, PNG files identical |
| Moments against static controls, orthographic 2.5× (t 0, 9, 17, 42) | 4 / 4 identical, 0 px |
| Fixed-position entity crops at other ticks against a same-frame control | 850 / 850 identical (52 entities). For 34 pairs, no control shows the same frame or a moving sprite touches the crop; those pairs are covered by state and identity checks only |
| Independent CPU model (contract + source PNGs) | 5,741,176 / 5,741,176 px over 28 ticks. Masked (2 px): 18 linear, nine-slice, rotated, sheared, tinted or translucent entities |
| Steadiness over 28 captures | 167,350 px never change; 33,696 changed px, 0 outside dynamic quads |
| Frame identity from unique interior colour | 1,478 / 1,478 agree with model and report; minimum 18 px; 0 foreign-frame px |
| No visible interior | 34 cases, as expected: the 4 px compressed nine-slice (28), and moving B4 (2) and D3 (4) after leaving the view. Their report states equal the model |
| Magenta (R ≥ 200, G ≤ 40, B ≥ 200) | 0 of 63 captures |
| Atlas frames against sheet pixel regions (A1/A6 nearest, A2/A16 linear) | 28 / 28 and 28 / 28 identical |
| Subset (8 syncs) against main; source-free against subset | 8 / 8 PNG + report entries; 8 / 8 PNG + report, report file hash `ce955d00…fa1a` both |
| Cooked atlas page level 0 | 0 content / 0 extrusion / 0 overlap / 0 empty mismatches |

- **First model run.** The model's first run reported 440,512 mismatches. Every one was a
  pixel outside every board plate: 15,744 per tick, fewer at 2099/2100, where the moving B5
  crosses the gap. My model's background was black; the renderer's clear colour is
  (20, 21, 25).
- **Final model run.** It reads the clear colour once from a corner and checks that it is
  uniform over all 15,744 gap pixels. Then 0 mismatches.

### Public workflow and source removal

- **Sources removed.** From `anim`: the project, the journal and 12 art files (14 files),
  after private copies in `r1/private-fixture/anim/`.
- **Kept, unchanged.** The cooked scene and 8 texture-cache files.
- **Source-free rerun.** It equals the earlier subset run byte for byte (PNGs and
  `report.json`).
- **Scope.** This proves cooked loading only. It is not a phone, performance or shipping
  gate.

## Appearance review

I viewed the engine frames and the derived figures, not only the statistics.

- **Sequence** (`figures/boundaries-row1-zoom3.png`, `board-a-frames-zoom2.png`)
  - On both sides of every boundary, the frame changes exactly at ticks 9, 17, 26, 34, 42
    and 51. The pips step clockwise, the digits read 0–4 and the reversed list counts down.
  - No in-between, doubled or stale frame appears.
  - The frames' different source sizes are stretched across the fixed destination. A
    15×10 frame and a 10×13 frame in the same 24 px square therefore show different texel
    widths (1.6 and 2.4 px), with nearest's uneven 1–3 px steps. This is the documented
    destination-size contract, not a crop shift. The transparent outer texel always leaves
    the same one-texel plate margin inside the rectangle.
- **Flips** (`details-zoom6.png`)
  - flip x puts green on the left, mirrors the digit and moves the pip;
  - flip y puts blue on top;
  - both flips give a half turn;
  - the parent reflection plus flip y in B2 also gives a half turn.
- **Nine-slice**
  - Borders keep their density: 1 px per texel at ppu 16, 2 px at ppu 8.
  - The asymmetric border mirrors under flip x (A13).
  - Centres stretch the glyph.
  - The 4 px compressed case shows only compressed borders.
  - There are no seams between slices in nearest, linear or rotated cases.
- **Linear sampling** (A2, A15, D8, D9 and the 2.5× view)
  - Soft as expected for upscaled pixel art.
  - Edges fade cleanly into the plate, with no dark rim and no magenta. The sheet-based
    linear animation equals its atlas twin.
- **Transforms and motion** (`board-b-motion-zoom2.png`)
  - Rotated and sheared sprites step cleanly.
  - The moving child and its static sibling stay 2 m apart.
  - The moving sprites change frames in place: no trail at earlier positions, no frame
    drawn at an old position.
- **Painter order** (`board-d-order-zoom2.png`, plus a pixel readout)
  - The layer −1 animation shows only through map holes.
  - The equal-(0,1) static, animated, static triple keeps ID order: the higher static
    badge covers the animation's corner.
  - The translucent layer-2 badge tints the layer-1 animation.
  - At the D5 tie, D5's ring corners draw over the ground tiles (higher entity ID). The
    order-2 stone covers D5's bottom ring between x 568 and 583, as the documented
    (layer, order, entity ID, layer ID, y, x) rule requires.
  - The moving sprite passes under the layer-2 badge.
  - The static atlas-frame sprite and the tilemap stay steady.
- **2.5× orthographic view** (`ortho-t0-t17-t42.png`)
  - Shows the usual uneven 2–3 px nearest texel widths at a non-integer scale.
  - Ticks 0 and 42 are identical, the exact wrap.

## Observations for Astra (no defect)

1. **Unequal frame sizes change texel density per frame** (appearance, informational).
   - **Contract:** each frame fills the fixed destination size, so mixing frame
     dimensions rescales texels from frame to frame, and nearest stretching makes the
     texel steps uneven. There is no per-frame trim offset or native-size mode.
   - **Effect:** an animator using a trimmed sheet would see the character's body
     rescale and jitter between frames.
   - **Option:** an authoring hint when one flipbook mixes frame sizes. No engine change
     is proposed.
2. **The static region of an animated renderer is never shown, but must still be valid.**
   - `validate` accepted the t2 stone and the sheet's magenta guard as the unused static
     regions, with no note.
   - The brief calls the ordinary headless screenshot a static first-frame preview. I did
     not run headless screenshots (outside this packet's bounds), so I cannot confirm that
     the authoring preview ignores the static region too.
   - **Option:** an Inspector note that the static region is unused while a flipbook is
     present.
3. **Report scope.** The sidecar gives frame index, playing and completed, but not the
   subframe phase. The phase was inferred from boundary timing (C1, C2, D3), not read
   directly. Recording the remainder would make phase evidence direct.

## Capture budget and timing

| Item | Value |
| --- | --- |
| Invocations | 19 of 40, all through `capture.py`, all exit 0 |
| Captures | 63 of 96: main 28, ortho 4, controls 11 pixel-perfect + 4 orthographic, subset 8, source-free 8 |
| Decoded bytes | 58,060,800 (55.37 MiB) of 256 MiB. My ledger equals `artifacts/flipbook-capture-budget.json` |
| Capture window | first started 01:20:09.454 −04:00; last ended 01:20:51.630 −04:00 |
| Failed or overwritten captures | none. Every output directory was new; `report.json` and PNG hashes are in `evidence/run/records/` |

## Changed paths

All changes are inside `handoffs/0042-native-flipbook/`. Astra's `brief.md` and
`capture.py` are unchanged.

| Path | Purpose |
| --- | --- |
| `result.md` | this report |
| `helpers/fb_png.py` | PNG I/O and the 3×5 font |
| `helpers/fb_art.py` | original art, atlas descriptor, manifest |
| `helpers/fb_scene.py` | scene, closed-form contract model, f64 motion replica, animated and control expansion, requests |
| `helpers/fb_model.py` | screen quads, masks, CPU nearest model and preview |
| `helpers/fb_review.py` | public-path driver: build and cook, gated `capture.py` calls, source-free step, hash checks, call log and ledger |
| `helpers/fb_analyse.py` | every check in this report |
| `helpers/fb_figures.py` | 9 derived figures |
| `helpers/fb_evidence.py` | evidence assembly; never replaces a file with different content |
| `helpers/test_fb_tools.py` | 7 self-tests |
| `art/` | 12 source files plus manifest; regenerate byte for byte |
| `evidence/` | analysis, run records, sidecar reports, call log, ledger, 10 unedited engine frames, 9 figures, `images.sha256`, `artifact-manifest.json` |

## Commands and results

Run from the repository root, with `H=handoffs/0042-native-flipbook/helpers` and
`R=artifacts/0042-native-flipbook/r1`. Every command exited 0.

```sh
python3 -I -B $H/fb_review.py build $R                     # 12 projects, 12 cooks, 4 requests
python3 -I -B $H/fb_review.py capture $R ctl-t00000-pp     # then: ortho, main
for t in 00009 00017 00025 00034 00042 00050 00060 00069 00420 02100; do
  python3 -I -B $H/fb_review.py capture $R ctl-t$t-pp; done
for t in 00000 00009 00017 00042; do python3 -I -B $H/fb_review.py capture $R ctl-t$t-ortho; done
python3 -I -B $H/fb_review.py capture $R subset
python3 -I -B $H/fb_review.py source-free $R               # 14 sources removed, cache kept unchanged
python3 -I -B $H/fb_review.py capture $R subset-source-free
python3 -I -B $H/fb_analyse.py $R artifacts/0042-native-flipbook/analysis/r1-final-1.json   # -2, -3 identical
python3 -I -B $H/fb_figures.py $R artifacts/0042-native-flipbook/analysis/r1-final-1.json artifacts/0042-native-flipbook/figures-final-1
python3 -I -B $H/fb_evidence.py $R artifacts/0042-native-flipbook/analysis/r1-final-1.json artifacts/0042-native-flipbook/figures-final-1
python3 -I -B $H/fb_art.py handoffs/0042-native-flipbook/art   # equals r1/art (diff -r)
python3 -I -B $H/test_fb_tools.py                               # 7 tests OK
cd handoffs/0042-native-flipbook/evidence && shasum -a 256 -c images.sha256   # 19 OK
```

Each `capture` call runs the protocol command, for example:

```sh
python3 handoffs/0042-native-flipbook/capture.py $R/cooked/anim.scene $R/empty-models \
  $R/anim/.incant/cache/textures 0192882754000000000000000000004b $R/requests/main.json $R/captures/main
```

- **Camera IDs.** `0192882754000000000000000000004b` is `camera-pp`; `…4c` is
  `camera-ortho`.
- **Model cache.** It is an empty directory, because no model assets exist.
- **Figures** rebuild byte for byte (`figures-final-1` equals `-3`).
- **Previews.** Before any capture, CPU previews checked the layout; they are kept as
  ignored `preview-*` and `art-check-*` files. They, an ad hoc detail sheet and the
  pre-final analyses (`r1-a1` to `r1-a3`) are not evidence.

**Not run:**
- Cargo, Clippy and the Rust or GPU test suites;
- `tools/tests` and the UI suite, since no file outside the packet changed;
- headless screenshots, native-play, the native window or player;
- hosted CI, other adapters or backends, phones and performance measurement.

## Artifact manifest

- **Committed:** `art/`, `helpers/` and `evidence/`, listed with sizes and SHA-256 in
  `evidence/artifact-manifest.json`. It records 92 committed files (`result.md` is written
  after it) and all 468 ignored files under `artifacts/0042-native-flipbook/`.
- **Ignored artifacts:** the projects, journals and caches of 12 projects, cooked scenes,
  63 captures with reports, private source copies, analyses, figures and previews.
- **Not committed:** no executable, cooked scene, cache or project.
- **Engine frames:** the 10 committed frames are unedited `native_frames` PNGs:
  - main ticks 0, 9, 17, 25, 26, 34, 42, 69 and 2100;
  - orthographic tick 17.
- **Figures** are derived: crops and integer nearest upscales with labels.

## Limitations

- **Hardware and path.**
  - One adapter (Apple M5 Pro, Metal by inference), on the bounded `native_frames` helper
    with a `ci`-profile binary.
  - It has no scripts, QuickJS/Wasm animation SDK, input, audio, saves, presentation or
    window.
  - No phone, performance, shipping, provider, interactive-player or phase-gate claim.
- **Cameras.** Pixel-perfect at 1× and orthographic 2.5× only.
  - No perspective, camera motion or following.
  - No magnification above 4× per texel.
- **Tie limitations.** I designed axis-aligned content away from exact ties.
  - Rotated, sheared and 2.5× content inherits the shared renderer's tie behaviour.
    Animated output equals static output there, so flipbooks neither fix nor add tie
    behaviour.
  - I did not re-measure the known shared limitations: ties above 128×, two-axis
    frame-edge and non-dyadic ties, and camera-following near ties.
- **Not exercised:**
  - runtime configuration changes (reinstall or changed animation), because the helper
    issues only `ControlFlipbook`;
  - invalid controls;
  - the 256-frame and scene limits;
  - RigidBody physics;
  - Astra's numerical GPU oracle and fresh shipping proof, which remain separate
    evidence.
- **Model scope.** The CPU model masks linear, nine-slice, rotated, sheared, tinted and
  translucent entities. Those are covered by the static controls and visual review, not
  by an independent prediction.

## Open questions

1. Should authoring warn when one flipbook mixes frame dimensions, since each frame is
   stretched to the destination size?
2. Should the Inspector mark an animated renderer's static region as unused? Should the
   static authoring preview be confirmed to show frame 0 even when the static region names
   another frame? I could not test that here.
3. Should `native_frames` reports include the subframe phase, so phase preservation is
   evidenced directly rather than through boundary timing?

No phase gate is approved or claimed.
