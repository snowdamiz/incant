# 0073 StageB — output-upscaling appearance review preparation

**Status: PREPARED, not run.** Nothing in this stage was built, compiled,
rendered or captured, and no GPU child ran.
- The future evidence directory `~/.codex/incant-evidence/0073-upscaling-visual`
  does not exist, and there is no grant.
- StageA's numeric pass is not used as appearance evidence. Appearance will be
  judged only by inspecting the PNGs after a root grant.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's ACP log header reads `Claude session verified: Opus 5.5; thinking: max`.

## Frozen identity for root's grant

| Item | Value |
|---|---|
| Commit | `290c1e3556d8cdf5f089744bb681c8632de2b4e5` |
| Visual tree (`git rev-parse 290c1e35:handoffs/0073-upscaling-review/visual`) | `3e328c4754a91d13a036811bc25f8efedcab512f` |
| Fixture `visual/review/upscaling_review.rs` | 1,972 lines, `f8ec6311…a7ee`; rustfmt 1.10.0 clean |
| Plan `visual/plan.json` | `3b6ee771…511b` |
| Tools | `common` `9256090f…`, `lease` `e1fe8738…`, `build` `004f6349…`, `gpu` `b581c19c…`, `analyze` `6afa2325…`, `selfcheck` `5aabaa7a…`, `test_lifecycle` `5005ac36…`, `test_analyze` `cdcbd3e9…` |
| Reviewed producer | `0452c5f5`; manifest `2d2f4ffa…`; 39-file freeze `b7ee5ca2…` |

### What `review-ready.json` must contain

- **Path:** `~/.codex/incant-evidence/0073-upscaling-visual/review-ready.json`.
- **`packet`:** `0073-upscaling-review`. **`stage`:** `upscaling-visual`.
- **`source_commit`:** any commit whose build inputs equal `0452c5f5`.
  - Build inputs are `crates`, `Cargo.*`, `third_party`, `tools/cargo` and
    `rust-toolchain.toml`.
  - Both `3caadfee` and `290c1e35` qualify; `build.py checkout` refuses any other.
- **`visual_tree`:** the tree hash above. The tools refuse a dirty or different
  tree.
- **`target`:** a dedicated `~/.codex/<name>-target` that root chooses. `/tmp`, the
  worktree and the evidence directory are refused.
- **At least one `prior_<name>_release_path` / `_sha256` pin:** for example,
  StageA's `0073-upscaling-numeric/gpu-released.json` (`fe268a84…6678`). Every
  pinned release must list no live process.

## Fixture design

**One physical output for every frame.** All frames are 960×540 Rgba8UnormSrgb
through `RetainedScene::submit`, the native host path, and each is read back in
full and hashed. Three variants share one renderer:
- **FULL** is the ordinary output-size control, unjittered.
- **DRS** is the same scene at a fixed 8/16 scale through the existing
  nearest-texel expansion, unjittered.
- **UP** is 8/16 plus output-resolution upscaling with its 8-phase jitter.

All crops use identical output-pixel rectangles in every variant.

**Policy `REVIEW`:** history 0.9, relative depth 0.05, absolute depth 0.02,
reactive threshold 4.0, strength 0.5. These are 0072's values, so the
fixed-resolution and output-resolution reviews share one policy. There is no
preset.

**Stage.** Camera: perspective, 50° FOV, at (0, 2.4, 9.5), pitched −0.16 rad.
- **Ground:** runs to the horizon (row ≈188), with the display backdrop above it.
- **Thin detail against the backdrop:** an antenna with horizontal wires about
  1.6, 0.8 and 0.4 px thick, plus a 0.56 px diagonal.
- **Thin detail against geometry:** 16 posts receding from 6.2 px to about
  1.3 px wide.
- **Texture detail:** a 2.2 m chart mapped at about 1.05 px per texel at full
  size, sampled trilinearly from its full mip chain. Its quadrants hold 1-texel
  lines, a 2-texel checker, rings that fall below a texel, and a 4-texel checker
  crossed by diagonals.
- **Shapes and light:** sphere and cube silhouettes, sun shadows and a dim
  environment fill.
- **Current-only overlays:** an orange 45% BLEND card over the cube's edge,
  three nearest-sampled 16×16 badge sprites, and world text: `ffi سلام 中文 ש`
  at 20 px and `Ag` at 48 px.
- **Physical UI:** an opaque button and a label drawn straight over the scene.

| Case | Frames | PNGs | Coverage |
|---|---|---|---|
| `probe` | 0 | — | Adapter and backend. The owner refuses before the startup opt-in; the opt-in succeeds and is idempotent. The policy validates; five invalid policies are refused, both by validation and by the owner. The logical owner footprint is recorded, but not as a budget |
| `stationary` | 36: FULL+UI 2, DRS+UI 2, 24 UP + 8 UI | full, drs50, up50, phases, detail, activity | Reduced versus full control; static thin detail; the BLEND card, sprites, text and UI; three jitter cycles, with the last full 8-phase cycle observed |
| `motion` | 24: 20 UP, FULL and DRS at frames 14 and 20 | full, up50, detail | 8 static frames. Then the cube moves 0.12 m (about 8 px) per frame out from behind the sphere and across the chart, covering occlusion and disocclusion. From frame 15 the camera also pans 0.05 m per frame |
| `transitions` | 29: FULL, DRS, 27 UP | full, up50 (frame 10), strip, cut (frame 23) | Fractional offset rounded viewport `[37.5, 21.25, 885, 497.5]`. 10 frames at 8/16, then internal-only changes to 12, 4, 16 and 8 (3 frames each), an explicit cut, and 5 frames at 8/16 |

**Totals:** **89 of 96 frames** (reserve 7) and **13 of 16 PNGs** (reserve 3),
all 960×540.

**Counting rule.**
- Every `RetainedScene::submit` attempt counts, including refusals, and so does
  every `UiLayer::submit`.
- The helper charges each frame before attempting it and panics past its case
  limit.
- `gpu.py` debits each case's full reserve before launch.

**Figures.** Each is a 4×4 grid of 240×135 nearest-zoom tiles with a 1 px
separator, using 80×45 px crops at 3× (or 120×67 at 2× for motion):
- `stationary-phases`: the wires and the world text in all 8 phases of the last
  cycle.
- `stationary-detail`: wires, card edge, chart and sprite, each as FULL, DRS, UP
  frame 24 and activity ×4.
- `stationary-activity`: a full-frame map of the per-pixel range over the last
  cycle, ×4.
- `motion-detail`: the mover crop at UP frames 8, 9, 11 and 14, then FULL, DRS
  and the |UP−FULL| and |DRS−FULL| ×4 maps; the same for the pan crop at frames
  14–20.
- `transitions-strip`: the chart corner across every boundary and the cut, then
  FULL, DRS and |UP22−FULL| ×4.

## What decides pass or fail

**Contract checks only.** These are deterministic facts the source and StageA
already establish:
- one owner, with consecutive sequences;
- jitter phases `k % 8`;
- history only after the first frame, kept through object motion, camera pans
  and internal scale changes, and reset by the cut (phase 0, epoch +1);
- internal sizes `ceil(960·n/16) × ceil(540·n/16)` at a 960×540 output;
- plain controls: FULL has no owners, and DRS runs at 480×270 without upscaling;
- the opaque button interior is byte-identical across all ten UI frames;
- the chrome at least 1 px outside the viewport is byte-identical in all 29
  transition frames;
- no blank frames.

**Observations only.** These never decide a case:
- the FULL-versus-variant mean, p99 and changed fraction, split into control edge
  and interior;
- last-cycle activity;
- consecutive-frame change;
- per-crop errors;
- wire presence per phase;
- the boundary jump versus the ordinary frame-to-frame change;
- the viewport border band.

`analyze.py` re-checks these independently:
- it re-decodes every PNG;
- it re-cuts 19 figure tiles from the retained raw frames;
- it re-checks UI and chrome identity on the raw frames;
- it recomputes the DRS and UP-frame-24 statistics.

## Expectations from source, to verify visually

These are not results; each one needs inspection of the PNGs.

1. **Edges stay current-only.** `resolve.wgsl` refuses history whenever any
   positive tap is uncovered or not depth-compatible. So every silhouette and
   depth edge is a bilinear sample of one jittered 480×270 frame. Expect
   softer edges than FULL that change with phase (shimmer), where DRS is blocky
   but static.
2. **Sub-texel wires flicker.** The 0.8 and 0.4 px wires (0.4 and 0.2 internal
   px) and the diagonal should appear and vanish by phase without accumulating.
   FULL and DRS show each wire either solid or missing, consistently.
3. **Interiors accumulate.** Shading and shadow edges should approach FULL.
4. **Texture detail stays low-pass.** I found no LOD bias in the source, so mip
   selection uses internal derivatives. The 1-texel chart lines should average
   to grey in both DRS and UP, with UP smoother than DRS but not recovering
   FULL's detail.
5. **Overlays shimmer.** BLEND, sprites and world text are drawn into an
   internal-resolution overlay and de-jittered bilinearly. Expect softened
   glyphs and sprite texels whose edges change by phase. The opaque content
   behind the card should still accumulate.
6. **Scale changes and cuts.** Interiors should hold across 8→12→4→16→8 while
   edges switch character immediately; 16/16 still jitters its edges. After the
   cut, frame 23 drops to current-only and has visibly reconverged by frame 27.
7. **Motion leaves trails.** Pixels the cube uncovers should be current-only
   until they reconverge. Bilinear reprojection during the pan may soften
   static content.
8. **Viewport border band.** A thin current-only band is possible along the
   viewport edge, where history taps fall outside the output rectangle.

## Helpers and the brief's essentials

The tools adapt 0068's root-reviewed `common`/`lease`/`build`/`gpu`/`analyze`/
`selfcheck`/`test_lifecycle` helpers. 0072's unreviewed tools were not copied.
The changes:

- **Freeze.** The grant pins the source commit and visual tree.
  - The checkout is a `git archive` export with every blob and mode verified,
    re-verified before every child.
  - Build inputs must equal `0452c5f5`, and the 39-file freeze must match.
  - The helper copy must equal the committed blob.
  - The binary is the one `upscaling_review` test executable named in Cargo's
    JSON. Exactly one `incant_render` library is allowed, with no `cooked-*`
    feature, which preserves the native-cooked refusal.
  - The binary is frozen at mode `0555`, and its hash and execute bit are
    checked before and after each child.
- **Predebit under one lock acquisition.** For each child the runner:
  - takes the packet flock (blocking), then the shared `native-gpu.lock`
    (non-blocking; busy refuses with no charge);
  - re-checks the gate and budgets;
  - fsyncs the full reserve to `debits.jsonl`;
  - launches and records while still holding both locks.

  Debits are never refunded.
- **Unconditional kill, wait and release.**
  - Each child runs in its own session with an empty signal mask.
  - The runner keeps INT/TERM/HUP blocked from the debit until the ledger row is
    fsynced, and polls the child and `sigpending()` every 50 ms.
  - Timeouts and interrupts both end in SIGKILL of the whole group and a reap.
    The interrupt surfaces only after recording.
  - Release: `gpu.py release` checks for owned processes and live ledger PIDs,
    then `lease.py release` runs.
- **Explicit environment.**
  - `HOME` is passed through without being recorded.
  - Otherwise the allowlist is `PATH`, `TMPDIR`, `RUST_BACKTRACE`,
    `INCANT_EXPECT_GPU_BACKEND=Metal` and the output directory, plus the Cargo
    variables for builds.
- **No reuse by filename.**
  - Every output, log, capture and copy is created exclusively, and label reuse
    is refused.
  - Publication verifies each copy's hash and copies at most 16 PNGs.
- **Stop on failure.**
  - Any failed build or child latches `stop.json`, and every later GPU step
    refuses.
  - Cases run once each, in order. This stage has no reruns; a retry needs a
    fresh grant.

## Commands run and results (CPU only)

| Command | Result |
|---|---|
| `python3 -I -B visual/tools/selfcheck.py --write visual/preparation/selfcheck.json` | `all_pass: true` (details below) |
| `python3 -I -B visual/tools/test_lifecycle.py --receipt visual/preparation/lifecycle.json` | **17/17** in 2.9 s, using real children (details below) |
| `python3 -I -B visual/tools/test_analyze.py --receipt visual/preparation/analyze-selftest.json` | Pass, 6/6 (details below) |
| `rustfmt --edition 2024` (1.10.0, stdin, minimal env, `RUSTUP_AUTO_INSTALL=0`) | Parses; formatting applied; `--check` clean. No Cargo target or compilation |

**Selfcheck** (`all_pass: true`):
- 15 plan checks and 11 fixture-versus-plan checks pass. These cover the tests,
  `Report::open` limits, per-case PNG names, 960×540, the output variable, the
  policy, and the constants mirrored in `analyze.py`.
- All 12 guarded commands refuse with the grant absent.
- The evidence directory was not created, `git status` was unchanged, and the
  shared lock was untouched.

**Lifecycle tests.** These used real `/usr/bin/true`, `/usr/bin/false`,
`/bin/sleep`, `/bin/sh` and Python children. Each of the following passed:
- the debit is visible to the child;
- a failure latches stop;
- a timeout kills the group;
- SIGINT, SIGTERM and SIGHUP each record the attempt and then raise;
- the child's signal mask is empty;
- an exec failure is recorded and charged;
- a busy shared lock, both budgets, a stop latch, a release latch and a
  non-executable binary each refuse with no charge;
- the release latch.

**Analyzer self-test:**
- hashes match;
- 14 tiles re-cut correctly;
- UI identity holds;
- the statistics reproduce;
- a one-pixel change in a tile is detected.

**Uncompiled-source review.** Compiling was not permitted, so I checked every
call against the `0452c5f5` build inputs. These include `prepare_retained`,
`RetainedTarget`/`submit`, `enable_temporal_upscaling` (renderer and owner),
`set_dynamic_resolution`, `upscale_frame`/`MotionFrame`,
`resolution_frame`, `invalidate_motion_history`, `ui_layer`/`stage`/`submit`,
`UiWidget`/`UiInput`, `TextNode`/`TextInput`, the `SpriteRenderer` field types,
`project_input` node identifiers and the glTF and texture cooking.
- Sprites and world text draw through the jittered camera matrix, so the
  perspective stage is valid.
- No workspace lint denies warnings.
- Readback, receipts, mesh builders, the badge and text shaping reuse the 0068
  helper's compiled code.

## Order for the future grant

1. `lease.py acquire`
2. `build.py checkout`, then `toolchain`, then `review`
3. `gpu.py list`
4. `gpu.py review` for `probe`, `stationary`, `motion`, then `transitions`
5. `analyze.py check`, and my inspection of every PNG
6. `gpu.py release`
7. Optionally `build.py lint`
8. `lease.py release`
9. `analyze.py publish`

## Changed paths

- `handoffs/0073-upscaling-review/visual/plan.json`
- `handoffs/0073-upscaling-review/visual/review/upscaling_review.rs`
- `handoffs/0073-upscaling-review/visual/tools/{common,lease,build,gpu,analyze,selfcheck,test_lifecycle,test_analyze}.py`
- `handoffs/0073-upscaling-review/visual/preparation/{selfcheck,lifecycle,analyze-selftest}.json`
- `handoffs/0073-upscaling-review/preparation-result.md` (this file)

## Limitations and open items

- **Uncompiled fixture.**
  - A compile error would surface in `build.py review`, before any GPU child, and
    latch stop.
  - The fix would need a new visual tree and a new grant.
  - Root may choose to run only the CPU build steps first.
- **No visual evidence yet.** There are no screenshots, pixels or appearance
  judgments.
- **Crop placement is projected.** Crops come from world anchors through the
  renderer's camera conventions. I checked them against an independent
  projection model, but they are unconfirmed until the first frames exist.
- **Limited scope.** One local Metal host only. There is no phone, performance,
  MetalFX, cooked-native or other-backend evidence. The 640 MiB + 16 KiB logical
  ceiling is not a budget.
- **Conservative counting.** UI submissions count as frames.
- **No regression suites.** The brief does not ask for them; root may add them
  outside this budget.
- **Ownership.** 0072 still owns the fixed-resolution temporal review; this packet
  covers only the output-resolution extension.
