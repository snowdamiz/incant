# 0062 result — cooked sprite backend appearance acceptance (Metal)

## Status

Complete within the packet scope, on one Mac Metal adapter.

- **Numerical GPU tests.** All 82 GPU-ignored tests ran once per variant, one test
  thread per process, in four groups. Every test passed on the baseline and on the
  candidate:
  - 6 sprite, tilemap and flipbook tests from the `incant_render` library;
  - 19 `model_gpu` tests for sprites, cameras and pipeline warmup;
  - the 26 remaining library tests;
  - the 31 remaining `model_gpu` tests.
- **Rendered comparisons.** All 9 baseline/candidate capture pairs are byte-identical
  PNG files, and therefore identical decoded RGBA, with 0 differing pixels. The fixtures
  cover nearest and linear sampling, ties, atlas and plain-sheet edges, premultiplied
  transparency, tint, depth and order, reflection, rotation, shear, nine-slice, wide
  strips, tilemaps, and flipbook first and final frames.
- **Source-free.** After deleting each board's private project, journal and art
  sources, the candidate re-rendered A-pp, W-pp and B-pp (ticks 0 and 48) from the
  cooked scenes and caches alone. All 4 frames are byte-identical to the pre-deletion
  frames.
- **In-scene controls.** All 33 controls are exact, after excluding one band where my
  own fixture overlapped two strips (explained below).
- **Visual check.** I inspected every capture and 20 zoomed crops personally. The shared
  appearance is correct for every listed property.

**Verdict:** accepted for appearance parity of `incant_render/cooked-sprites` against
the baseline sprite WGSL, on an Apple M5 Pro with macOS 27.0.1 and Metal. I found no
cooked-path defect, so none is reported for Astra to fix. This is not Vulkan, DX12,
phone, performance, default-enablement or phase-gate evidence.

## Priority revisions and director feedback

- **Brief.** The brief is unchanged across all three sessions: SHA-256
  `25a0219e72c7f5e9403bf1543b5fe126cf9460a66c8fc197890e205cfe679dcb`, re-checked
  before committing. It has no priority-revision or director-feedback section.
- **Director pause.** Root recorded `user-pause.json` at 23:41:10Z (SHA-256
  `b04c8121…`): "Director requested a good checkpoint to pause all work".
  `pause-worktree.json` (`a275180e…`) preserved my five tool files; all five hashes
  matched on resume, so I resumed from them.
  - At that point no GPU work had started: no marker existed and no renderer had run.
- **Resumes.**
  - The first resume was interrupted right after the last numerical test exited
    (00:19:22.8Z).
  - The second resume found no new pause record, and the brief and marker unchanged.
    It continued with the captures.
- **Grant.** Astra's `gpu-review-ready` (SHA-256 `9c61859d…`, granted
  00:13:59.399Z) names `0062-cooked-sprite-shaders`. Its scope: "Offscreen sprite GPU
  and appearance checks under existing packet constraints; no desktop/nativewindow
  capture". It also says to inspect and reuse the paused tools and builds before
  rebuilding; I did.
  - A byte copy is in `evidence/gpu/grant-marker-copy.json`. I did not modify or delete
    the marker.

## Model and thinking effort

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** Max, the session's configured reasoning effort.
- **ACP verification.** Root's ACP logs each contain
  `Claude session verified: Opus 5.5; thinking: max`:
  - first session: `claude-0062-acp.log` (`164df5cd…`);
  - first resume: `claude-0062-resume-acp.log` (`484861c5…`);
  - current session: `claude-0062-resume-002-acp.log`, which is still being appended,
    so its hash will change.

  I read only those verification lines.
- No subagent or auxiliary reviewer was used. I did not edit personal memories.

## Host, adapter and toolchain

| Item | Value |
|---|---|
| OS | macOS 27.0.1 (26A434), Darwin 27.0.0, arm64 |
| GPU / adapter | Apple M5 Pro. wgpu `AdapterInfo` reports `IntegratedGpu`, backend `Metal`, as printed by the sprite tie and wide tests. Every `native_frames` report names adapter "Apple M5 Pro" |
| Rust | rustc 1.99.0 (b940084d7 2026-09-28), LLVM 23.1.1; cargo 1.99.0 |
| Python | 3.14.8, standard library only, always run with `-I -B` |
| Cargo target | `/Users/andreyyurlov/.codex/incant-core-presentation-target`, used exclusively. Leased 23:31:19.7Z and released 00:28:56.3Z (`lease/target-lease.json`). Left in place as warm cache; nothing deleted |

## Source and frozen binaries

- **Source.**
  - Worktree HEAD is `ca7f4521`. The only difference from source commit `23f92d71` is
    retained evidence `.gz` files and this brief. `git diff 23f92d71 HEAD -- crates
    Cargo.toml Cargo.lock tools/cargo rust-toolchain.toml` is empty.
  - All 13 source hashes in `docs/spikes/evidence/cooked-sprite-shaders.json` match both
    the worktree and the source commit.
  - `sprite.wgsl` is `1b8b9125…774c`, which matches the manifest and is unchanged from
    the parent commit. Record: `build/source-freeze.json`.
- **Builds.** Every build used
  `./tools/cargo … --locked --profile ci -j2 --message-format=json-render-diagnostics`.
  The exact argv, exits and artifact lists are in `build/{baseline,candidate}.json`.

| Variant | Features | `incant_render` library tests | `model_gpu` | `native_frames` |
|---|---|---|---|---|
| baseline | `incant_render/cooked-output,incant_render/cooked-geometry` | `fcc8743eb77471bd922d2f76eca2bc6f3ea89eaed6cc9281e55175e3e9a84439` | `4128c1cf863318f6a55a5f31239984861464943396c7b98df4423c8ca4341cd5` | `f48a4e16b6a6362e481ed042537fb59d4385404bce2b3e25029d4d8bdf90d3b4` |
| candidate | the same plus `incant_render/cooked-sprites` | `0e74223a770bbb0e8b11ba326f69e2044addeb05801b480ade4b43552d024b1c` | `2954e09db2bcfab426e37cc63f358631948201cf15345cb63d5241e7a62f01bd` | `a8321a5075a66dfd45fed83ec844be7ea283797d70013900de6023b2054fe6cc` |

- **CPU-only authoring tools** (default features):
  - `cook_scene` example: `940529cf0de9af90c2fc71253b02a738c7ebe21056c64957df396210b58f0a53`
  - `incant_headless`: `e8260ae81a641e1362aed22a1a15e20b374b454757ff0814695ece807805ea50`
- **Which shader each binary embeds** (byte search):
  - The baseline `native_frames` and `model_gpu` embed the complete sprite WGSL and
    contain no cooked MSL.
  - The candidate `native_frames` and `model_gpu` embed no sprite WGSL.
  - The candidate embeds each of the six stage artifacts (MSL, SPIR-V and HLSL, vertex
    and fragment) exactly once, byte-equal to Astra's retained
    `cooked-sprites-checks-001/artifacts/*.shader.gz`. This includes the 12/52-byte
    `vb_15`/`vb_14` vertex-pulling structs.
- **Pipelines.** The warmup test reports 31 prepared render pipelines in both variants.

## GPU interval, ledger and release

All GPU work went through `tools/gpu.py`. Before each child, under an exclusive `flock`,
it checks:

- that the marker exists and names this packet;
- that my release latch is absent;
- the frozen binary's SHA-256, before and after the run.

Each child ran alone in its own process group with a clean environment (no `INCANT_*`,
`RUST_*` or `WGPU_*` variables): 300 s limit for tests and 120 s for captures. Survivors
were killed and every child was reaped.

- **Ledger.** `gpu/ledger.jsonl` (`e7219317…`) has 29 children:
  - 4 test listings (`--list --ignored`);
  - 8 numerical test processes;
  - 17 `native_frames` runs, producing 22 PNGs.

  All exited 0. None timed out. Binary hashes were unchanged after every run.
- **Interval.** First child started 00:17:19.06Z; last child ended 00:21:40.44Z.
- **Release.** `gpu/released.json` (`b035a689…`), written 00:22:30.81Z:
  - no process with a path under the evidence `bin/` remained;
  - no ledger PID was alive;
  - the marker hash at release was unchanged.
- **After release.**
  - `gpu.py test …` and `gpu.py list …` both refuse ("GPU slot already released by this
    packet") before creating any log or child.
  - No renderer process was started after the release.
  - Everything afterwards was CPU-only.
- **Captures:** 22 of the 32 allowed, all 960×540.

## Numerical GPU tests (exact runs)

Command form:

```
<frozen binary> --ignored --test-threads=1 --nocapture <filters>
```

| Group | Filters | Baseline | Candidate |
|---|---|---|---|
| library, sprite-relevant | `sprite_tests:: tilemap_tests:: flipbook_tests:: retained_sprite retained_tile retained_flipbook` | 6 passed, 0.54 s (PID 25777) | 6 passed, 0.67 s (PID 25858) |
| `model_gpu`, sprite-relevant | `sprites:: cameras:: pipeline_warmup::` | 19 passed, 18.29 s (PID 26160) | 19 passed, 18.28 s (PID 26635) |
| library, rest | `--skip` the six filters above | 26 passed, 1.41 s (PID 27083) | 26 passed, 1.38 s (PID 27358) |
| `model_gpu`, rest | `--skip sprites:: --skip cameras:: --skip pipeline_warmup::` | 31 passed, 27.47 s (PID 27129) | 31 passed, 27.40 s (PID 27371) |

- **Totals.** 82 of 82 per variant, matching the 32 + 50 test listing, which is
  identical across variants.
- **What the sprite-relevant groups include:**
  - the 70-case magnified tie sweep and the 8-orientation tie test;
  - the 5,768-case wide-region sweep;
  - atlas repacking and source removal;
  - nine-slice and the hidden-colour fringe check;
  - painter order and alpha, subregions and flips;
  - all ten camera tests;
  - pipeline warmup;
  - the retained native sprite, tilemap and flipbook source-free oracles.
- **Logs.** Library stderr (UI and text diagnostics) is byte-identical across variants.
  `model_gpu` stderr differs only in the printed preparation time (7.96 ms vs 6.93 ms).
  The logs are in `evidence/logs/`.

## Fixtures (public command-bus authoring)

- **Art.** `tools/art.py` draws deterministic art; its manifest is
  `fixtures/art-manifest.json` (`6e6355b9…`).
  - Every transparent texel hides magenta RGB under alpha 0, so missing premultiplication
    or region bleed would show.
  - Pieces: an extruded atlas (padding 2) of 14 frames; a plain 48×16 sheet with three
    unpadded adjacent regions; 4096-texel horizontal and vertical strips with a unique
    colour per texel; flipbook frames of five sizes with index pips; and an emissive
    opaque glTF quad.
- **Projects.** `tools/author.py` builds three projects, using only frozen
  `incant_headless` commands:
  - `init`;
  - `import` (`--texture-usage color`; the model with no usage flag);
  - `rpc` with `project.read`, one `command.execute` of `create_entity` ops, and
    `project.save`;
  - `validate`.

  It then cooks each scene with `cook_scene`. The script never writes project JSON.
- **Engine-call log.** `fixtures/engine-calls.jsonl` records 22 calls, all exiting 0,
  with tool hashes checked before and after each call:
  - 3 `init`, 7 `import`, 6 `rpc`, 3 `validate`;
  - 3 cooks.

  All three projects validated with `valid: true` and no diagnostics.
- **Failed first attempt.** The first run failed: my tilemap layer ULID prefix contained
  `L`, which is not Crockford base32. The bus rejected the transaction with "tilemap
  identities must be canonical uppercase ULIDs". I corrected the prefix and rebuilt
  everything as run r2; r1's call log is kept as `fixtures/r1-failed-engine-calls.jsonl`.

| Board | Content | Entities | Cooked scene (v / bytes / SHA-256) |
|---|---|---|---|
| A | Sampling: nearest 1×, 2×, 2.5×; linear 4× and 0.5×; half-pixel ties; rotation by 90° and 30°; yaw 60°. Atlas ring nearest and linear; plain-sheet middle region and edge region; alpha ramps over a backdrop; tint and half-alpha. Arrow glyph flips, mirrored parents, rotations by 90°, 180°, 30° and 45°, shear. Nine-slice nearest, linear and compressed. Painter order and ID ties. Depth against an opaque mesh, front, piercing and translucent sprites | 55 | 5 / 12,115 / `87a66d20…` |
| W | Horizontal and vertical strips 4096 texels long at 3 lp/texel, plain and flipped, nearest and linear, plus a half-pixel tie strip, each with a cropped tail control | 14 | 5 / 3,226 / `47ba75a7…` |
| B | Tilemap 8×4 with two layers and its raw-cell sprite control; the same map rotated 90° and mirrored at 1.5×; 8 flipbooks (loop, linear, once with flip, reversed, sheet regions, paused, rotated, once/linear/30°) with static first and final-frame controls | 73 | 8 / 18,966 / `ee6de868…` |

Cameras and cases:

- **Cameras** (cases are in `fixtures/build.json`):
  - pixel-perfect: 320×180 lp at 3×;
  - orthographic: A at 6.17 m, W at 7.77 m and B at 5.11 m vertical size, each with
    sub-pixel offsets;
  - perspective (A only): yaw 5° from z = 12.
- **Final tick.** Tick 48 is the final frame for every animated flipbook, as the
  B-pp report shows (`reports/*-B-pp.json`).

## Rendered comparisons

| Case | Tick | PNG SHA-256 (baseline = candidate) | Decoded RGBA SHA-256 | Source-free re-render |
|---|---|---|---|---|
| A-pp | 0 | `da6ef43edbb57e4e39ada3479aac2dfd43002782247d2171d49d05bbb9a2ce70` | `f4ca98b1…dd60d` | identical |
| A-ortho | 0 | `70f6876e6e7084442051e5fb27b0defb230df4c50acf033e82df9781b1b915d3` | `16b0aed2…2304` | — |
| A-persp | 0 | `12703d40a3b69a427746ff20812b182317933e5bff45d2963ecf178de1de1e7d` | `bc617e36…6336` | — |
| W-pp | 0 | `31ba871b89ee2a4ea3b9d803b81d207e3e75a9e7861c1cd4667ed2ab59ce8604` | `ce4cc8e5…3449` | identical |
| W-ortho | 0 | `f9627d71d5e1a1409a4640847619fb546614ff5d8ff8d84b677a10c3ceec34c1` | `3055ab8f…14bd6` | — |
| B-pp | 0 | `9afbfd6bec2519b513cda81d0129d778950d716676d325a470f81c6ef69e29c5` | `fe031c00…3e4d27` | identical |
| B-pp | 48 | `efb8d40b0eda1a9ff3cd3afbcb4936ddc52adf50ea55511357d9e4367b559bc3` | `8bd790d6…70f5` | identical |
| B-ortho | 0, 48 | `a31e024b2c272b6d0a5e829c24eb67ed4e2fb0fdbafa9aeff71440d1305aaa50`, the same file at both ticks | `73a25002…3d376e` | — |

- **Pairs.** 9 of 9 PNG files and 9 of 9 decoded images are identical, so there is no
  difference to explain. The PNGs carry only IHDR, IDAT and IEND.
- **Source-free.** Each board's project file, journal and 20 art files were copied to a
  private fixture directory, verified, then deleted (`fixtures/source-free-*.json`).
  - The cooked scene and texture and model caches were unchanged.
  - The only other file left is an empty `.incant-source-transactions/lock` file
    (0 bytes, no content) created by the bus.
  - The three candidate re-renders were byte-identical to the pre-deletion frames,
    4 of 4.
- **Animation scope.** From tick 0 to tick 48, B-pp changes 28,242 pixels, all inside
  the seven animated sprites' boxes: 0 outside them, and 0 in the paused book, the
  controls or the tilemaps. B-ortho frames only tilemaps, so its two ticks are
  identical.
- **Analysis.** `analysis/analysis.json` (`70bf8618…`) reproduces byte-for-byte on a
  rerun.
- **Self-checks** (`analysis/selfcheck.json`):
  - a single 1-LSB mutation is detected as 1 pixel with delta 1;
  - all 22 captures round-trip exactly through the stdlib decoder and writer.

### In-scene controls (crops that must match within one capture)

All exact (0 differing pixels):

- **Ties.** A6 (2× at a half-pixel offset) equals A2 (aligned 2×). A7 (the tie sprite
  flipped in x and y) equals A2 flipped.
- **Nearest replication.** A2 equals A1 with every texel replicated 2×2.
- **Mirrors and rotations.** C5 (mirrored parent) equals C2 (`flip_x`). C6 (mirrored
  parent plus `flip_x`) equals C1. C8 (rotated 180°) equals C4 (`flip_xy`). C2 and C3
  are exact mirrors of C1.
- **Wide strips.** W1, W2 and W3 trailing ends equal their tail controls. The W5 linear
  strip end equals its control after excluding the control's first texel, which clamps
  differently by design. The W6 half-pixel tie end equals its control.
- **Tilemap.** T1 equals its raw cell-sprite grid at ticks 0 and 48.
- **Flipbooks at tick 0.** F1–F5 and F7 equal their first-frame controls.
- **Flipbooks at tick 48.** F1–F5, F7 and F8 equal their final-frame controls.
- **Paused flipbook.** F6 equals its held frame at both ticks.

The one mismatch is a fixture overlap of my own:

- **What I saw.** The W4 versus W4c comparison first showed 324 differing pixels
  (worst delta 28) in rows 321–338.
- **Cause.** The W6 tie strip runs left from x = 810.5 to far off-screen at rows
  321.5–339.5. It crosses both W4 (x 291–309) and W4c (x 321–339), and it is drawn on top
  of them by entity-ID painter order. Each row of those 18 is therefore W6's horizontal
  gradient, at different x positions.
- **Result.** Outside the band, W4 equals W4c exactly (rows 294–320 and 339–437). Inside
  the band, every column is constant, as expected for W6 drawn on top.

This is a fixture layout limitation, identical in both variants. It is not a rendering
defect.

## Appearance verdicts (personal pixel inspection)

I viewed all eight distinct capture frames at native size, plus 20 nearest-zoom crops
(`tools/figures.py`; 8 are committed under `evidence/derived/`). Baseline and candidate
are byte-identical, so each verdict applies to both.

- **Nearest sampling:** pass.
  - Crisp texels at 1×, 2× and 3×.
  - At non-integer scales texel widths alternate as expected: 7.5 px per texel (A3,
    2.5× pixel-perfect), 4.5 px (the yawed sprites and the 1.5× tilemap) and 5.47 px
    (the orthographic A view).
  - No sub-texel shimmer or extra rows.
- **Ties:** pass. Half-pixel-offset sprites keep uniform 6-pixel texels, and their first
  and last columns equal the aligned control. The flipped tie sprite is the exact 180°
  image. The wide tie strip end matches its control.
- **Linear filtering:** pass.
  - Smooth bilinear magnification at 4×, 3× and 2×.
  - Deterministic minification at 0.5×.
  - Rotated (30°, 45°) and yawed linear sprites have soft edges.
  - Region clamping holds: the unpadded plain-sheet middle region at 2× has a cyan edge
    with no red or blue from its neighbours, and the red edge region has no cyan.
- **Atlas edges:** pass. Ring frames show their distinct side and corner colours. The
  transparent outer texels give background (nearest) or a smooth fade (linear), with no
  magenta and no neighbour-frame colour. Tiles are seamless.
- **Premultiplied alpha and transparency:** pass.
  - Alpha ramps composite over the checker backdrop smoothly from transparent to opaque.
  - The alternating alpha-128 and alpha-255 columns are correct.
  - There is no magenta anywhere, although every zero-alpha texel hides magenta.
  - Half-alpha tint shows the backdrop at about half intensity.
  - The translucent sprite blends over the mesh.
- **Tint:** pass. The colour tint (1, 0.6, 0.3) warms and darkens the checker uniformly.
- **Depth and order:** pass.
  - Painter order: red (layer 1) over green (layer 0, order 5) over blue (order 0). In
    the equal-key pair, the later ID draws on top.
  - Depth: the opaque emissive quad hides the part of the checker behind it, and the
    front arrow (z = 1) draws over the quad.
  - The yaw-70° ring is visible only on its front half. The perspective view keeps the
    same depth relationships.
- **Reflection, rotation and shear:** pass. All flips and mirrored parents give the
  correct handedness, by both the glyph's blue and green marker texels and the exact
  controls. Rotations by 90° and 180° are exact. At 30° nearest edges are stepped and at
  45° linear edges are smooth. The sheared arrow shows no gaps.
- **Nine-slice:** pass. Nearest borders stay pixel-exact while the centre stretches.
  Linear borders are kept with a smooth centre. The compressed 4×12 lp sprite scales its
  borders proportionally.
- **Wide sprites:** pass. Trailing texels 4080–4095 appear at the correct ends, plain and
  flipped, horizontal and vertical. The 9-pixel bands are uniform, with no lost or
  doubled texels at 768 m extents, including in the fractional orthographic view.
- **Tilemaps:** pass. T1 is pixel-identical to its raw cell sprites. The rotated and
  mirrored map at 1.5× is seamless, with expected 4/5-pixel nearest widths, and is crisp
  at the fractional orthographic scale.
- **Flipbooks:** pass. At tick 0 every animated sprite shows its first frame; at tick 48
  it shows its final frame. The once-books report `completed`. The reversed book shows
  f4 then f0, the sheet book red then blue, and the paused book holds f0. Rotated and
  linear books equal their static controls.
- **Source-free:** pass. The frames rendered from cooked data alone are identical to the
  pre-deletion frames.

## Defects and observations

- **Cooked path.** No defect found, so nothing is reported for Astra to fix.
- **Observation for Astra** (compiler and runtime owner; not shown to matter here). In
  pinned `wgpu-hal` 29.0.4, `metal/device.rs` compiles passthrough MSL
  (`ShaderInput::Msl`) with default `MTLCompileOptions`:
  - no `preserveInvariance`;
  - the system-default language version.

  WGSL modules are compiled with the adapter's `msl_version` and
  `setPreserveInvariance(true)`. On this M5 Pro with macOS 27.0.1 every compared frame,
  including the half-pixel edge ties, is bit-identical. Other Apple GPUs and OS
  versions were not tested.
- **Fixture limitations** (mine; identical in both variants):
  - the W6/W4 overlap described above;
  - in A-persp, the 60°- and 70°-yawed sprites land nearly edge-on, so perspective-correct
    texel interpolation is shown only weakly there (the numerical camera tests cover
    perspective);
  - B-ortho excludes the flipbooks;
  - C11 and F8 are clipped by a few pixels at the frame edge.
- **Shared rendering traits, not regressions.**
  - No MSAA, so polygon edges are aliased: stepped edges on the rotated and perspective
    quads.
  - Nearest widths are uneven at non-integer scales, as specified.

## Screenshot paths

Committed (unedited candidate PNGs; each is byte-identical to the baseline file):

- `evidence/captures/A-pp-tick-00000.png`
- `evidence/captures/A-ortho-tick-00000.png`
- `evidence/captures/A-persp-tick-00000.png`
- `evidence/captures/W-pp-tick-00000.png`
- `evidence/captures/W-ortho-tick-00000.png`
- `evidence/captures/B-pp-tick-00000.png`
- `evidence/captures/B-pp-tick-00048.png`
- `evidence/captures/B-ortho-tick-00000.png`

B-ortho at tick 48 is the same file as at tick 0. These paths are relative to the packet
directory.

Derived figures (CPU crops and zooms of those captures, not renders): `evidence/derived/`
holds `ties-A2-A6-A7`, `atlas-edges-ring`, `plain-sheet-neighbours`,
`premultiplied-alpha`, `order-ties-depth`, `tilemap-vs-cells`, `flipbooks-tick48` and
`wide-ends`, plus `figures.json`.

All 22 captures, their `report.json` files and all 20 figures stay in
`/Users/andreyyurlov/.codex/incant-evidence/0062/{captures,analysis/figures}`.

## Commands run (from the worktree root; all exited 0 unless noted)

```
# CPU builds: frozen copies and hashes go to /Users/andreyyurlov/.codex/incant-evidence/0062/{bin,build}
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/build.py freeze
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/build.py baseline     # cargo test --no-run + build --example native_frames
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/build.py candidate
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/build.py cook         # -p incant_cook --example cook_scene
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/build.py headless     # -p incant_headless --bin incant_headless
# CPU fixtures (r1 failed: invalid layer ULID prefix; r2 succeeded)
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/author.py build <evidence>/fixtures/r2
# GPU, serial, after the grant
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/gpu.py list {baseline,candidate} {lib,model_gpu}
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/gpu.py test <variant> lib sprite-relevant sprite_tests:: tilemap_tests:: flipbook_tests:: retained_sprite retained_tile retained_flipbook
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/gpu.py test <variant> model_gpu sprite-relevant sprites:: cameras:: pipeline_warmup::
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/gpu.py test <variant> lib rest --skip <the six library filters>
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/gpu.py test <variant> model_gpu rest --skip sprites:: --skip cameras:: --skip pipeline_warmup::
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/gpu.py capture <variant> <A-pp|A-ortho|A-persp|W-pp|W-ortho|B-pp|B-ortho>
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/author.py source-free <evidence>/fixtures/r2 <A|W|B>
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/gpu.py capture candidate <A-pp|W-pp|B-pp> --source-free
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/gpu.py release
# Refusal checks after release (expected exit 1, no child or log)
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/gpu.py test candidate lib after-release-check
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/gpu.py list candidate lib
# CPU analysis and packaging
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/analyze.py <evidence> <evidence>/analysis/analysis.json
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/figures.py <evidence> <evidence>/analysis/figures
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/selfcheck.py <evidence> <evidence>/analysis/selfcheck.json
python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/package.py && python3 -I -B handoffs/0062-cooked-sprite-shaders/tools/package.py --check
git diff --check   # clean
```

`<evidence>` is `/Users/andreyyurlov/.codex/incant-evidence/0062`. The exact argv,
timestamps, PIDs and hashes of every GPU child are in `evidence/gpu/ledger.jsonl`.

## Changed paths

Packet only; nothing outside `handoffs/0062-cooked-sprite-shaders/` changed in the
worktree.

- `handoffs/0062-cooked-sprite-shaders/result.md`
- `handoffs/0062-cooked-sprite-shaders/tools/`: `analyze.py`, `art.py`, `author.py`,
  `build.py`, `figures.py`, `gpu.py`, `package.py`, `pngio.py`, `pngtool.py`,
  `scene.py`, `selfcheck.py`
- `handoffs/0062-cooked-sprite-shaders/evidence/`: 132 files plus `manifest.json`
  (about 667 KB). It lists each file's source path, raw hash and committed hash.
  - The eight libtest stdout copies drop only the final blank line, so
    `git diff --check` passes; their raw hashes are recorded.
  - Cargo JSON logs (0.34–0.66 MB each), binaries and private fixtures stay in the
    ignored evidence directory and are recorded by hash only.

The brief is Astra's, already committed, and unchanged. Root's pause records and ACP
logs are outside the worktree; I left them unchanged and did not commit them. No
production code, other handoff, CI, account or memory was touched.

## Limitations and unavailable checks

- **Single backend.** One adapter (Apple M5 Pro), one OS (macOS 27.0.1), one backend
  (Metal). Vulkan SPIR-V and DX12 HLSL artifacts were not executed. No phone, Device
  Farm, Linux lavapipe or Windows WARP run; no performance, pipeline-cache or hitch
  measurement; no phase gate.
- **Offscreen only.** All frames come from the `native_frames` review helper in the
  `ci` profile. No window, presentation path, editor or shipping build was used.
- **Tests are Astra's oracles.** The numerical tests are Astra's existing suites; I only
  executed them. Visual acceptance rests on my inspection and the in-scene controls.
- **Scratch file.** One transient analysis copy was written to `/tmp` and deleted
  immediately to confirm the analysis reproduces. All other work stayed in the worktree
  and the granted evidence directory.

## Open questions

1. Should Astra pin passthrough MSL `MTLCompileOptions` (language version and
   `preserveInvariance`) to match the WGSL path before `cooked-sprites` reaches other
   Apple GPUs? The note above has the details. Parity held on this device.
2. Should a later packet run the same comparison on Vulkan and DX12 (lavapipe and WARP
   in CI, or hosted devices) before default enablement? The SPIR-V and HLSL sprite
   stages have not yet run on any driver.
