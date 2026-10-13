# 0063 result — cooked UI and world-text backend appearance acceptance (Metal)

## Status

Complete within the packet scope, on one Mac Metal adapter.

- **Numerical GPU tests.** All 89 GPU-ignored tests in the relevant suites ran once per
  variant, one test thread per process, and passed on both the baseline and the candidate:
  - 11 UI/text tests of the `incant_render` library (UI draw, text draw, retained text and
    source-free native text);
  - the 21 remaining library tests;
  - `model_gpu`: pipeline warmup (1) and the 49 others;
  - `incant_render_gpu_host`: `ui` (2) and `game` (5).
- **Rendered comparisons.** All 8 baseline/candidate frame pairs are byte-identical PNG files
  (and therefore identical RGBA, 0 differing pixels in the full frame). They cover Arabic and
  CJK shaping, ligatures, bidi, reflected (negative scale, flipped parent, back-facing),
  rotated, yawed, magnified and minified world text, coverage over dark and bright
  backdrops, overlapping coverage, depth occlusion, motion, and the game UI at 1×, 1.5× and
  2× into sRGB and UNORM targets with rounded panels, packed colours, clipping, focus,
  selection and preedit.
- **Source-free.** After each board's project, journal, fonts and meshes were copied aside
  and deleted, the candidate re-rendered W-persp (ticks 1 and 61) and the 1.5× preedit frame
  from the cooked scenes and caches alone: 3 of 3 frames are byte-identical to the
  pre-deletion frames.
- **Bounded in-scene controls.** 4 of 4 geometric controls (duplicate, mirror, back-face,
  vertical flip) are exact; 53 of 53 visible packed-colour probes equal the theme values
  exactly in both target formats.
- **Visual check.** I inspected all 8 distinct frames at native size and 19 zoomed crops. The
  shared appearance is correct for every listed property.

**Verdict:** accepted for appearance parity of `incant_render/cooked-ui` and
`incant_render/cooked-text` against the baseline UI and text WGSL, on an Apple M5 Pro with
macOS 27.0.1 and Metal. Numerical verdict: pass (89/89 per variant, identical diagnostics).
Visual verdict: pass (identical frames, correct appearance). No cooked-path defect was
found, so none is reported for Astra to fix. Diagnosis with one added feature at a time
was not needed, because no comparison failed. This is not Vulkan, DX12, phone,
performance, default-enablement, fully cooked renderer or phase-gate evidence.

## Priority revisions and director feedback

- **Brief.** SHA-256 `7994fcf8adc1f88d6cb2025b0746f9a3d954b4cd189071d5b5fa9c7f9b37cb1e`,
  committed by Astra in `91b70493`. It has no priority-revision or director-feedback
  section, and it was unchanged at the start, before the first GPU child and before
  committing (`git diff HEAD -- brief.md` empty each time).
- **Grant.** Astra's `review-ready.json` (SHA-256 `a7477ce2…7c7e8e`, granted
  00:32:40.9Z) names `0063-cooked-ui-text-shaders`, the exclusive target and the
  restrictions "serial offscreen only; 24 frames max 960x540; bounded processes; no native
  windows/desktop capture; no further use by 0062". Its prior-release hashes match 0062's
  `gpu-released.json` (`b035a689…`) and `target-lease.json` (`385ba125…`) exactly. A byte
  copy is in `evidence/gpu/grant-copy.json`; I did not modify or delete the grant.
- **No earlier attempt.** The packet directory held only the brief; no interrupted files
  existed.

## Model and thinking effort

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** Max, the session's configured reasoning effort.
- **ACP verification.** Root's log `claude-0063-acp.log` contains
  `Claude session verified: Opus 5.5; thinking: max`. I read only that line. The log is
  still being appended, so its hash changes (it was `286cf779…` when read).
- No subagent or auxiliary reviewer was used. Personal memories were not edited.

## Host, adapter and toolchain

| Item | Value |
|---|---|
| OS | macOS 27.0.1 (26A434), Darwin 27.0.0, arm64 |
| GPU / adapter | Apple M5 Pro, `IntegratedGpu`, backend `Metal` (every helper report and the UI test adapter lines) |
| Rust | rustc 1.99.0 (b940084d7 2026-09-28), LLVM 23.1.1; cargo 1.99.0 (5f94df478 2026-08-27) |
| Python | 3.14.8, standard library only, always `-I -B` |
| Cargo target | `/Users/andreyyurlov/.codex/incant-core-presentation-target`, exclusive. Leased 00:40:55.3Z, released 01:00:41.2Z (`evidence/lease/target-lease.json`). Left in place as warm cache; nothing deleted |

## Source and frozen binaries

- **Source** (`evidence/build/source-freeze.json`).
  - Worktree HEAD `91b70493` differs from source commit `bacdd6f2` only by Astra's
    retained text artifacts and this brief. `git diff bacdd6f2 HEAD -- crates Cargo.lock
    Cargo.toml tools/cargo rust-toolchain.toml third_party` is empty.
  - All 22 sources in `cooked-text-shaders.json` match `bacdd6f2` and the worktree. All 15
    sources in `cooked-ui-shaders.json` match `f8b95de5`; 9 of them were later changed by
    the text commit, as expected. Both source-review files match their manifest hashes.
  - `ui.wgsl` (`f67e4982…`), `text.wgsl` (`e6925b05…`), `ui/geometry.rs`, `ui/theme.rs` and
    `text/geometry.rs` are unchanged from `f8b95de5~1` to HEAD. The pipeline files changed
    only by moving layouts into shared functions; blend, depth and formats are the same.
- **Builds.** Every build used `./tools/cargo … --locked --offline --profile ci -j2
  --message-format=json-render-diagnostics`. Exact argv, exits and artifacts are in
  `evidence/build/{baseline,candidate,tools}.json`. Every non-test `incant_render` unit in
  each build had exactly the variant's features (checked from Cargo's artifact messages).

| Variant | Features | lib tests | `model_gpu` | `ui` | `game` | `ui_text_review` |
|---|---|---|---|---|---|---|
| baseline | `incant_render/cooked-output,incant_render/cooked-geometry,incant_render/cooked-sprites` | `7db28f4a…a8b1` | `f88b756c…f48b` | `2ddea6d0…5ba7` | `9ad9f5af…9e94` | `bc380a6b…1721` |
| candidate | the same plus `incant_render/cooked-ui,incant_render/cooked-text` | `79c03167…937c` | `9c24b655…73cb` | `7a29c04d…098a` | `62d432b3…58fe8` | `be25c276…3780` |

Full hashes are in the build records. CPU authoring tools (default features):
`incant_headless` `ba50e683e0df74b35c8ea8bdaf44fbe64fb263bd0218cddd3ea6553cc80d40ae`,
`cook_scene` `940529cf0de9af90c2fc71253b02a738c7ebe21056c64957df396210b58f0a53`.

- **Review helper.** No repository binary renders cooked UI plus world text into a frame of
  at most 960×540: `native_frames` loads no fonts, `ui_review` also writes 1920×1080 and
  720×1280 frames, and the player's `--gpu-offscreen` mode is 160×96, hash-only and refuses
  UI. I wrote `tools/review/`, a packet-local Cargo package with its own `[workspace]`
  (no repository manifest touched).
  - It adapts `ui_review`: one cooked scene with its model, texture and font caches runs
    through `HostedOfflineGameSession`, `NativeRenderHost` and `GpuRenderConsumer`, with a
    JSON step plan and at most 960×540.
  - Its `Cargo.lock` is the repository lock pruned by `cargo metadata --offline`. All 375
    non-helper entries have identical version, source and checksum; no dependency edge was
    added; 33 packages drop feature-unused edges; the unused `tao` patch is recorded
    (`evidence/build/helper-lock-*.json`).
  - Clippy with `-D warnings` passes for both feature sets. rustfmt changed one line break
    after the build; the frozen binaries were built from the as-built source
    (`evidence/build/review-main.rs.as-built`, `9546373e…`), which differs from the
    committed `main.rs` (`fb6b1f11…`) only in whitespace.
- **Which shader bytes each binary embeds** (`evidence/analysis/embed.json`, exact byte
  search):
  - Every baseline binary embeds `ui.wgsl` and `text.wgsl` once and none of the 12 cooked
    UI/text stage artifacts.
  - The candidate `model_gpu`, `ui`, `game` and `ui_text_review` binaries embed no UI/text
    WGSL and each of the 12 stages (MSL, SPIR-V and HLSL, vertex and fragment, for UI and
    text) exactly once, byte-equal to Astra's retained `cooked-ui-shaders/metal/artifacts`
    and `cooked-text-metal-001/artifacts`.
  - The candidate library-test binary also contains the WGSL (its source-hash tests
    `include_bytes!` it) and two copies of each stage (const data duplicated across
    codegen units).
- **Pipelines.** Warmup reports 31 prepared render pipelines in both variants.

## GPU interval, ledger and release

All GPU work went through `tools/gpu.py`. Before each child, under an exclusive `flock`,
it checks that the grant exists and names this packet, that my release latch is absent,
the frozen binary's SHA-256 before and after, and the frame budget. Each child ran alone
in its own process group with a clean environment (no `INCANT_*`, `RUST_*`, `WGPU_*`,
`CARGO_*` or Metal variables): 300 s limit for tests, 120 s for captures. Survivors were
killed and every child reaped.

- **Ledger.** `evidence/gpu/ledger.jsonl` (`cd326607…`) has 38 children:
  - 8 test listings;
  - 12 numerical test groups;
  - 18 helper captures.

  None timed out, and no binary hash changed. Two children exited 1: U-15x in both
  variants (my fixture error, below), writing no frame.
- **Interval.** First child 00:54:09.0Z; last child ended 00:58:29.7Z.
- **Frames.** 21 frames written, all 960×540: 2 superseded r2 baseline frames, 16 r3
  baseline/candidate frames and 3 source-free frames. My runner counted conservatively
  (failed U-15x reserved 2), so its budget stood at 23 of 24. No test wrote frames: the
  clean environment drops `INCANT_UI_EVIDENCE`.
- **Release.** `evidence/gpu/released.json` (`gpu-released.json`, 00:59:12.8Z): no process
  under the evidence `bin/` remained and no ledger PID was alive. Afterwards `gpu.py test`,
  `list` and `capture` all refuse ("GPU slot already released by this packet") before
  creating any log or child. No renderer ran after the release, and no cargo after the
  lease release. `evidence/gpu/release-receipt.json` records both releases, the unmodified
  grant hash and zero processes using the frozen binaries or target.
- **Ledger of every command.** `evidence/ledger/commands.json` lists all 8 cargo builds
  (0 failures), all 78 engine calls (0 failures), all 38 GPU children and every other
  command in session order with its exit, including the failed and refused ones.

## Numerical GPU tests (exact runs)

Command form: `<frozen binary> --ignored --test-threads=1 --nocapture <filters>`.

| Group | Filters | Baseline | Candidate |
|---|---|---|---|
| library, UI/text | `ui::draw_tests:: text::draw_tests:: models::retained::text_native_tests:: models::retained::text_tests::` | 11 passed, 0.86 s (PID 56681) | 11 passed, 0.93 s (PID 56684) |
| library, rest | `--skip` the four filters | 21 passed, 1.17 s (PID 56705) | 21 passed, 1.14 s (PID 56726) |
| `model_gpu`, warmup | `pipeline_warmup::` | 1 passed, 0.49 s (PID 56757) | 1 passed, 0.49 s (PID 56759) |
| `model_gpu`, rest | `--skip pipeline_warmup::` | 49 passed, 43.05 s (PID 56778) | 49 passed, 45.22 s (PID 57007) |
| `incant_render_gpu_host` `ui` | all | 2 passed, 0.55 s (PID 56761) | 2 passed, 0.55 s (PID 56765) |
| `incant_render_gpu_host` `game` | all | 5 passed, 0.61 s (PID 56763) | 5 passed, 0.60 s (PID 56771) |

- **Totals.** 89 of 89 per variant, matching the listings (32 + 50 + 2 + 5), which are
  identical across variants.
- **Coverage named by the brief.**
  - UI coverage, clipping and scales: integer and fractional scales, nested clips including
    negative-x RTL overflow, compositing over the finished world, editor viewport regions,
    retained atlas/quads across refused stages.
  - Text: baselines, bearings, bidi order and missing glyphs; rotated, scaled, minified and
    magnified text against an affine reference; depth/occlusion against opaque geometry;
    retained motion, rebuild and changed text with exact identity reuse; source-free native
    text through the retained consumer.
  - Hosted UI over the moving world with resource reuse, focus, IME commit/preedit and
    relayout; hosted text with models, script and audio and no motion uploads.
- **Diagnostics are identical across variants** (the UI/text library stderr is
  byte-identical, 2,523 bytes):
  - text against the affine CPU reference, max |alpha error|: 0.00135 at 1:1, 0.00201 at
    scale 0.42 and 0.4 rad, 0.00324 at scale 1.7 and −0.15 rad, 0.00227 at scale 0.24 — all
    under 1/255;
  - UI against the independent CPU canvas: `outside 0` everywhere, worst 1 code at 1×, 2×
    and 1.5× sRGB, nested clips and over-world; worst 0 for the 1× UNORM target;
  - the deliberately wrong-order negative control (text under selection) is detected in
    both variants (`outside 279 by 47`; the test requires > 20 and > 8).
- **Other log differences** are only libtest's filtered-out counts (the candidate compiles
  two extra non-ignored cooked-stage CPU tests), durations, preparation time
  (39.43 ms vs 38.25 ms) and the thread ID of the `game` suite's deliberate
  "target presentation panic" (its fault-injection test). Logs: `evidence/logs/`.

## Fixtures (public command-bus authoring)

`tools/author.py` builds two projects using only frozen `incant_headless` commands:
`init`, `import` (the three packaged test fonts and four generated cube glTFs), `rpc`, then
`validate`, then `cook_scene`. The `rpc` step runs `project.read`, one atomic
`command.execute` of `create_entity` ops, `history.undo`, `history.redo` (project equal),
`project.save`, and a journal reopen (project equal). The script never writes project JSON.
Both projects validate with `valid: true` and no diagnostics. Strings use only glyphs in
the packaged fonts (Latin, Arabic, CJK subset 中文日本語一).

| Board | Content | Entities | Cooked scene (bytes / SHA-256) |
|---|---|---|---|
| W | 22 world-text nodes, two cameras (orthographic at 100 px/m, i.e. one atlas texel per pixel at scale 1, and perspective framing z = 0 the same), a dark wall, a bright panel and a crate. Nodes: Latin with ffi/fl ligatures, Arabic, CJK, an integer-pixel duplicate, 30° rotation, bidi over the bright panel, X-mirror (scale −1) with an unmirrored control, the same string seen from behind, 2.5× magnification, a child of a Y-flipped parent with an unflipped control, minification 0.5/0.25/0.12 (2, 4 and 8.3 texels per pixel), −60° yaw, text behind and in front of the crate, word wrap, two overlapping copies, and a moving node (0.6 m/s) | 29 | 6,130 / `31fc2ac382852b9731ed71c1a6729c9d1604e0e69eb71d90a3b98d8f269d55e2` |
| U | The 0056 game-UI layout: title, Latin/Arabic/CJK body, four buttons (one disabled, one Arabic), a fixed 250 px clip strip whose third button overflows, and caption/field groups whose long LTR and RTL values overflow. Behind it a moving crate, a pillar and a world-text sign | 31 (25 UI nodes, 17 drawn widgets) | 7,726 / `9514ddcd5f64e2a2a43da353732373937513a8f80ea5983569483a026c49729d` |

- **Runs.** r1 and r2 were superseded; their records are in `evidence/fixtures/`.
  - r1: board W had no exact in-scene controls; it was never rendered.
  - r2: the first two baseline frames showed two fixture weaknesses of mine. The +60° yaw
    text ran back into the wall and was depth-occluded after "Yaw sixty", and the Arabic
    field value did not overflow. r3 uses −60° and a longer value. The two r2 frames are kept
    and counted (`evidence/gpu/r2-superseded-moved.json`).
- **U-15x failure (mine).** The first 1.5× request focused the CJK field, which at a
  640×360 logical viewport lies below the root's padded content box. Its hit box is empty,
  and both variants failed identically with "invalid UI declaration: widget cannot receive
  focus", writing no frame. That is the documented refusal of a focus receipt for a
  widget without a hit box, not a rendering result. A supplementary request (`U-15x-name`,
  `evidence/fixtures/r3/cases-extra/`) focuses the visible name field instead.
- **Cases** (one tick per step; `evidence/fixtures/r3/requests/`):
  - W-ortho and W-persp: Rgba8UnormSrgb 960×540, no UI; W-persp captures ticks 1 and 61;
  - U-1x: viewport 960×540, idle, then focus the name field and select "Lovelace";
  - U-15x-name: viewport 640×360 at 1.5×, focus the name field, preedit "語";
  - U-2x: viewport 480×270 at 2×, Tab focuses Resume;
  - U-1x-unorm: the 1× idle frame into an Rgba8Unorm target.

## Rendered comparisons (full frame)

| Case | Frame | Tick | PNG SHA-256 (baseline = candidate) | RGBA SHA-256 | Source-free |
|---|---|---|---|---|---|
| W-ortho | t1 | 1 | `a446d2d0db44fa13a6b623aaa5feca3d4e573f396d53833ff62c457f5abb4a28` | `219b6903…` | — |
| W-persp | t1 | 1 | `3d8c373c09f6c97823d05300528672a1eee059a58668f38f9f42e22bf736a95a` | `cb78a174…` | identical |
| W-persp | t61 | 61 | `b2cc9cabd58f00acb87c22ee601552799d6903491ed9b6af08e8c40f0e0454e1` | `07e7a633…` | identical |
| U-1x | idle | 1 | `c0fd239d40e6e52fa68048127039aa215731dc0850e2b1fadd1af6cb36fbf467` | `e8de33a0…` | — |
| U-1x | select | 3 | `00e01c7414e68d3249f2d48c513efdc9710735c3280cfd1b86b78b8b6be538a4` | `d2993315…` | — |
| U-15x-name | preedit | 2 | `cd9e0281be552e0c83c767bd857a68e2f59e8068da9511d3f1d8749753ccd88d` | `188f5d35…` | identical |
| U-2x | focus | 2 | `f19d0a7a0d04b825ab044e912b43daa233dd4bbea5886adadab023af8bece788` | `a0eb44a8…` | — |
| U-1x-unorm | idle | 1 | `d526b55a17643b8f8364799097feb05dcd42f79b6f76ef8a83c4a7a4baeaf595` | `3be58198…` | — |

- **Pairs.** 8 of 8 PNG files and 8 of 8 decoded images are identical over the whole frame;
  no region is excluded, and there is no difference to explain. PNGs carry only IHDR, IDAT
  and IEND.
- **Source-free.** Each board's project, journal, three fonts and four meshes (9 files) were
  copied to a private fixture directory, verified and deleted
  (`evidence/fixtures/r3/source-free-{W,U}.json`). The cooked scene and the font and model
  caches were unchanged. The only other file left is the bus's empty
  `.incant-source-transactions/lock`. 3 of 3 re-renders are byte-identical; these are also
  separate-process repeat runs of the same binary.
- **Retained updates in the reports.**
  - Over 60 motion ticks W-persp makes no text atlas or glyph-buffer upload (one each at
    startup).
  - Focus and selection add UI quad buffers (+1 at 2×, +2 at 1×) and no UI atlas.
  - The preedit "語" adds exactly one UI atlas and two quad buffers.
  - 0 missing glyphs everywhere (W: 22 nodes, 327 glyph quads; U: 17 widgets, 250–251 UI
    glyph quads, 1 world-text node).
- **Analysis.** `evidence/analysis/analysis.json`. Self-checks (`selfcheck.json`): a single
  1-LSB change is reported as 1 pixel with delta 1; all 19 captures round-trip exactly
  through the stdlib codec; mirroring a control crop changes 3,514 pixels, so the mirror
  controls are not vacuous.

## Bounded in-scene controls (separate from the full-frame result)

These compare regions within one frame or check fixed values. They are appearance controls,
not parity evidence, and are reported with their exact boxes in `analysis.json`.

- **Geometric controls on W-ortho** (uniform wall background, one colour over the 4,400-pixel
  sample box): 4 of 4 exact, 0 differing pixels each.
  - `w_latin_dup` equals `w_latin` 85 px higher (2,016 ink pixels).
  - `w_mirror` (scale −1, 1, 1), mirrored back, equals its unmirrored control (1,937 ink
    pixels).
  - `w_back` (rotated 180° about Y, seen from behind) equals `w_mirror`.
  - `w_child` (parent scale 1, −1, 1), flipped back, equals its unflipped control (2,558 ink
    pixels).

  Exact equality was not assumed: the subjects' origins were placed on pixel boundaries so
  that the controls sample the same atlas positions. No tolerance was needed.
- **Packed UI colour.** 53 of 53 probes inside visible widget boxes equal the theme values
  exactly, in U-1x idle (sRGB), U-1x-unorm idle (UNORM), U-1x select, U-2x focus and
  U-15x-name preedit.
  - Surfaces: button 46,50,58; disabled button 35,38,44; field 21,23,28; field border
    100,106,119.
  - Accent rings 76,146,255: 2, 4 and 3 physical pixels wide at 1×, 2× and 1.5×.
  - Full-coverage title glyphs: 236,238,242.
  - Ten further probes fell on widgets clipped to zero height (fields below the viewport or
    the root's padded content box at 2× and 1.5×). They are recorded as clipped, not as
    results.
- **State scope.**
  - W-persp tick 1 → 61 changes 3,127 pixels, all inside [658,446,905,478], the moving text.
  - U-1x idle → select changes 2,963 pixels inside the name field [32,264,392,300]. The
    other 465 lie inside [630,270,764,387], the crate moving by Velocity for two ticks.
- **sRGB vs UNORM target** (one UI and world, U-1x vs U-1x-unorm; identical in both
  variants, and a target-format property, not a parity result): 23,385 pixels differ.
  - 13,793 lie on drawn widgets (glyph and corner edges, max 63). `ui.wgsl` blends encoded
    values for UNORM and linear values for sRGB, so light-on-dark glyphs look thinner on
    UNORM.
  - 9,592 lie in the world (max 1 code): the output pass encodes in the shader for UNORM
    and on store for sRGB.
  - Opaque surfaces agree exactly.

## Appearance verdicts (personal pixel inspection)

I viewed all eight distinct frames at native size and the 19 derived crops. Baseline and
candidate are byte-identical, so each verdict applies to both.

- **Shaping and scripts:** pass. "ffi" and "fl" ligatures in Noto Sans. Arabic joined and
  right-to-left, right-aligned in its box. CJK from the fixture font. Mixed bidi lines and
  word wrap in the correct visual order.
- **Coverage edges:** pass. White text has smooth antialiased edges over the dark wall and
  the bright amber panel, with no dark fringe or halo. The two overlapping copies composite
  without seams.
- **Reflection:** pass. The X-mirrored, back-facing and Y-flipped samples have the correct
  handedness and equal their controls exactly.
- **Rotation and yaw:** pass. 30° text is smooth. The −60° yawed text is compressed
  horizontally in orthographic view and grows toward the camera in perspective, without
  breakup.
- **Magnification:** pass. 2.5× text is a soft bilinear enlargement of its 16 px raster, as
  the text contract specifies (no mipmaps or distance fields).
- **Minification:** pass. 0.5× and 0.25× are clean. 0.12× (8.3 texels per pixel, beyond the
  4×4 tap cap) is legible but aliased, as documented. There are no neighbour-glyph marks at
  any level, so atlas isolation holds.
- **Depth and occlusion:** pass. Text behind the crate is cut at its silhouette; text in
  front draws over it.
- **Motion:** pass. The moving node shifts about 60 px between ticks 1 and 61 with an
  unchanged shape.
- **UI surfaces and packed colour:** pass. Rounded corners are antialiased over about one
  physical pixel at 1× and stay smooth at 2×. Surface colours are exact. Disabled text is
  dimmed. Label shadows show over the bright pillar at 2× and vanish over dark areas.
- **Clipping:** pass.
  - The 250 px strip cuts the third button's surface and its CJK glyph at a hard edge.
  - The long LTR value clips at the field's right content edge, and the long Arabic value at
    its left content edge (negative-x overflow).
  - At 1.5× and 2×, the root's padded content box clips lower content.
- **Focus, selection and preedit:** pass. Focus rings are 2, 4 and 3 px. The translucent
  selection tint keeps the selected text legible. The "語" preedit is underlined at the
  caret at 1.5×.
- **UI over world text:** pass. The UI draws over the moving world and over the world-text
  sign.
- **Source-free:** pass. Frames rendered from cooked inputs alone are identical to the
  pre-deletion frames.

## Defects and observations

- **Cooked UI/text paths.** No defect found, so nothing is reported for Astra to fix.
- **Observation for Astra** (compiler/runtime owner; same as 0062, independently
  re-checked; not shown to matter here). In pinned `wgpu-hal` 29.0.4,
  `src/metal/device.rs`:
  - Naga-generated WGSL modules get `setLanguageVersion(msl_version)` and
    `setPreserveInvariance(true)` (lines 227–233).
  - Passthrough MSL (`ShaderInput::Msl`, lines 1171–1178) uses default `MTLCompileOptions`.

  On this M5 Pro with macOS 27.0.1 every compared UI/text frame is bit-identical, including
  the snapped UI edges and the text tap loops. Other Apple GPUs and OS versions were not
  tested.
- **Fixture limitations (mine).**
  - r2's yaw and Arabic weaknesses and the U-15x focus error, all explained above.
  - U-2x and U-15x-name show no field groups below the root's padded content box; the
    corresponding probes are recorded as clipped.
  - Both boards use the packaged test fonts only. The CJK subset limits CJK strings to
    中文日本語一.
- **Shared rendering traits, not regressions.**
  - No MSAA, so mesh edges are aliased.
  - Bilinear magnification is soft and minification beyond 8 texels per pixel aliases,
    both as specified.
  - Light-on-dark UI text looks lighter on UNORM targets because of encoded-space blending.

## Screenshot paths

Committed, all relative to this packet directory:

- **Captures** — unedited candidate PNGs, each byte-identical to its baseline and, where
  run, to its source-free re-render:
  - `evidence/captures/W-ortho-t1.png`
  - `evidence/captures/W-persp-t1.png`
  - `evidence/captures/W-persp-t61.png`
  - `evidence/captures/U-1x-idle.png`
  - `evidence/captures/U-1x-select.png`
  - `evidence/captures/U-15x-name-preedit.png`
  - `evidence/captures/U-2x-focus.png`
  - `evidence/captures/U-1x-unorm-idle.png`
- **Derived figures** — CPU crops and nearest zooms of those captures, not renders, each
  within 960×540, with `figures.json`. `evidence/derived/` holds:
  - world text: `w-mirror-controls`, `w-flip-parent-control`, `w-scripts`,
    `w-coverage-over-bright`, `w-magnified-rotated`, `w-minification-ladder`, `w-overlap`,
    `w-persp-yaw-depth`, `w-persp-motion`;
  - UI: `u-rounded-corners-1x`, `u-disabled-and-arabic-buttons`, `u-clip-strip`,
    `u-overflow-clips`, `u-selection-focus`, `u-focus-2x`, `u-preedit-15x`,
    `u-label-over-world-2x`, `u-srgb-vs-unorm`, `u-srgb-vs-unorm-text`.

All 21 frames, every `report.json` and the figures stay in
`/Users/andreyyurlov/.codex/incant-evidence/0063/{captures,captures-r2-superseded,analysis/figures}`.

## Commands run (from the worktree root; all exited 0 unless noted)

```
T=handoffs/0063-cooked-ui-text-shaders/tools; E=/Users/andreyyurlov/.codex/incant-evidence/0063
python3 -I -B $T/lease.py acquire
python3 -I -B $T/build.py freeze
python3 -I -B $T/build.py helper-lock      # exit 1: my verifier mis-parsed [[patch.unused]] and elided versions
python3 -I -B $T/build.py verify-lock      # exit 1 (same defect), then exit 0 after the fix
python3 -I -B $T/build.py tools            # first background attempt: shell redirect failed, no cargo ran
python3 -I -B $T/build.py {tools,baseline,candidate}
python3 -I -B $T/author.py build $E/fixtures/r1             # superseded
python3 -I -B $T/embed.py $E/analysis/embed.json
python3 -I -B $T/author.py build $E/fixtures/r2             # superseded after 2 frames
python3 -I -B $T/gpu.py list {baseline,candidate} {lib,model_gpu,ui,game}
python3 -I -B $T/gpu.py test <variant> lib ui-text ui::draw_tests:: text::draw_tests:: models::retained::text_native_tests:: models::retained::text_tests::
python3 -I -B $T/gpu.py test <variant> lib rest --skip <the four filters>
python3 -I -B $T/gpu.py test <variant> model_gpu warmup pipeline_warmup::
python3 -I -B $T/gpu.py test <variant> model_gpu rest --skip pipeline_warmup::
python3 -I -B $T/gpu.py test <variant> {ui,game} all
python3 -I -B $T/gpu.py capture baseline {W-ortho,U-1x-unorm}  # r2, superseded
python3 -I -B $T/author.py build $E/fixtures/r3
python3 -I -B $T/gpu.py capture <variant> {W-ortho,W-persp,U-1x,U-15x,U-2x,U-1x-unorm}  # U-15x children exit 1
python3 -I -B $T/author.py add-case $E/fixtures/r3 U-15x-name
python3 -I -B $T/gpu.py capture <variant> U-15x-name
python3 -I -B $T/author.py source-free $E/fixtures/r3 {W,U}
python3 -I -B $T/gpu.py capture candidate {W-persp,U-15x-name} --source-free
python3 -I -B $T/gpu.py release
python3 -I -B $T/gpu.py {test,list,capture ... --source-free}  # exit 1: refused after release, no child or log
rustfmt --edition 2024 --check $T/review/src/main.rs         # exit 1 (one line break), then formatted
./tools/cargo clippy --manifest-path $T/review/Cargo.toml --locked --offline --profile ci -j2 --features <variant> -- -D warnings
python3 -I -B $T/lease.py release          # exit 1: scan matched its own shell line; rerun exit 0
python3 -I -B $T/analyze.py $E $E/analysis/analysis.json
python3 -I -B $T/selfcheck.py $E $E/analysis/selfcheck.json
python3 -I -B $T/figures.py $E $E/analysis/figures
python3 -I -B $T/ledger.py $E/commands.json $E/release-receipt.json
python3 -I -B $T/package.py && python3 -I -B $T/package.py --check
git diff --check
```

The exact argv, timestamps, PIDs and hashes are in `evidence/ledger/commands.json`,
`evidence/gpu/ledger.jsonl` and `evidence/build/*.json`.

## Changed paths

Packet only; nothing outside `handoffs/0063-cooked-ui-text-shaders/` changed in the
worktree.

- `handoffs/0063-cooked-ui-text-shaders/result.md`
- `handoffs/0063-cooked-ui-text-shaders/tools/`: `common.py`, `lease.py`, `build.py`,
  `boards.py`, `author.py`, `gpu.py`, `embed.py`, `analyze.py`, `selfcheck.py`,
  `figures.py`, `pngio.py`, `package.py`, `ledger.py`, and `review/{Cargo.toml,Cargo.lock,src/main.rs}`
- `handoffs/0063-cooked-ui-text-shaders/evidence/`: 165 files plus `manifest.json` (about
  1.5 MB). The manifest lists each file's source path, raw hash and committed hash.
  - Twelve text logs drop only trailing whitespace or a final blank line so
    `git diff --check` passes.
  - Cargo JSON logs, binaries, analysis drafts, baseline and source-free frame copies,
    superseded r2 frames, cooked scenes and the private fixture copies stay in the
    ignored evidence directory, recorded by hash (55 files).

The brief is Astra's, already committed, and unchanged. The grant, root's ACP logs and
other packets' files are outside the worktree; I left them unchanged and did not commit
them. No production code, other handoff, CI, account or memory was touched.

## Limitations and unavailable checks

- **Single backend.** One adapter (Apple M5 Pro), one OS (macOS 27.0.1), one backend
  (Metal). The SPIR-V and HLSL UI/text stages were not executed on any driver. No phone,
  Device Farm, Linux lavapipe or Windows WARP run; no performance, pipeline-cache or hitch
  measurement; no phase gate.
- **Offscreen only.** All frames come from the packet's `ui_text_review` helper in the `ci`
  profile through the composed game host. No window, presentation path, editor, exported
  player or shipping build was used. The helper is review tooling, not a repository
  example.
- **Tests are Astra's oracles.** The numerical suites are Astra's; I only executed them.
  Visual acceptance rests on my inspection and the bounded controls.
- **Fonts and text color.** World text has one fixed unlit color (`TEXT_COLOR`), so
  colored world text was not exercised. Fonts are the packaged test fonts.
- **Scratch.** One transient file (`/tmp/claude-0063-diffcheck.txt`, the empty
  `git diff --cached --check` output) was written and deleted. All other work stayed in the
  worktree, the granted evidence directory and the leased target.

## Open questions

1. Should Astra pin passthrough MSL `MTLCompileOptions` (language version and
   `preserveInvariance`) to match the WGSL path before the cooked families reach other
   Apple GPUs? Parity held on this device for sprites (0062) and now UI and text.
2. Should a later packet repeat this comparison on Vulkan and DX12 (lavapipe and WARP in CI,
   or hosted devices) before default enablement? The SPIR-V and HLSL UI/text stages have not
   yet run on any driver.
