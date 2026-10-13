# 0066 result — fixed cluster compute backend acceptance (Metal)

## Status

Complete within the packet scope on one Mac Metal adapter, with one item routed to
Astra: a direct raw count/mask comparison at the extended configurations needs a
cfg(test) observer that does not exist yet. I specified it below and did not
implement it.

- **Native GPU suites.** All 89 current GPU cases ran in each variant, one test thread
  per process, with every ignored GPU test enabled: 32 `incant_render` library, 50
  `model_gpu`, 5 `gpu_host` game and 2 `gpu_host` UI.
  - **Baseline: 89 of 89 completed. Candidate: 89 of 89 completed.** None failed,
    skipped or refused.
  - Listings and per-test result lines are identical across the variants.
- **Cluster readbacks.** The three crate-internal lighting GPU tests also ran in
  isolation in each variant. All passed.
  - Two of them read back the real GPU counts and masks. They cover local-light counts
    1, 31, 32, 33 and 4096, plus 63, 64, 65, 127, 128, 129, 4095, a downward 65 and
    a downward 1.
  - They also cover a 24-light sparse depth-slice case behind a directional light, and
    a stale-membership clear.
  - The tests compare each readback to closed-form oracles. Both variants matched the
    same exact arrays.
- **Rendered comparisons.** The packet helper rendered 20 frame states per variant
  through the retained native path from cooked scenes.
  - **All 20 are whole-frame identical across variants**, and all 6 PNG pairs are
    byte-identical.
  - The states cover 4096 → 33 → 32 → 31 → 1 → 0 local lights, retained motion at a
    fixed grid, five grid sizes, an offset viewport, perspective and orthographic
    cameras, and a directional-only control.
  - Separate dry runs repeated every frame hash in a second process per variant.
- **Controls.** In each variant, 17 frames were re-rendered three ways: retained
  resubmission, non-retained clustered, and the non-retained all-lights reference.
  **51 of 51 controls were exact (0 differing pixels) in both variants.**
- **Source-deleted fixture.** After deleting both boards' project, journal and art, the
  candidate rendered 5 of 5 frames identical to the pre-deletion frames.

**Numerical verdict: pass for everything exercised.**

- The test readbacks are identical between variants at the configurations those tests
  encode.
- Every rendered state is bit-identical between variants.
- The exact all-lights control shows the candidate's masks were complete wherever a
  light reached a visible pixel.
- **Not established:** a direct byte comparison of raw counts/masks for moving, resized,
  offset-viewport, orthographic and spatially varied layouts. That needs the observer
  requested under "Request to Astra".

**Appearance verdict:** I accept `incant_render/cooked-clusters` for appearance parity
with the baseline WGSL cluster compute on an Apple M5 Pro, macOS 27.0.1, Metal. I
inspected all six capture pairs and six tile-aligned zoom crops personally.

I found no production defect, so none is reported to Astra. This is not Vulkan, DX12,
phone, performance, pipeline-cache, prewarming, default-enablement or phase-gate
evidence.

## Brief, grant and director feedback

- **Brief.** SHA-256 `9700510e…5814b21b3`, committed by Astra in `96bb23df`.
  - It has no priority-revision or director-feedback section, so I had nothing to
    acknowledge or apply first.
  - I re-checked it before acquiring the lease, before writing this file and before
    committing. It was unchanged each time, with no uncommitted root revision.
- **Grant.** `/Users/andreyyurlov/.codex/incant-evidence/0066/review-ready.json`, SHA-256
  `43eaf4ea…3a94d0e`, granted 02:23:00.62Z.
  - It names this packet, source commit `96bb23df8eab86e00bef5b22e85525d36dd84972`
    (= HEAD) and the target.
  - Its prior-release pins verify:
    - `0b88d47a…` is 0065's `release.json`;
    - `eb2d547f…` is 0065's `lease.json`;
    - `5ae8d09f…` is the first 99 lines (85,525 bytes) of 0065's `ledger.jsonl`, which
      is followed only by 0065's release event.
  - A byte copy is in `evidence/gpu/grant-copy.json`. I did not modify or delete the
    grant.

## Model and thinking effort

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** Max, the session's configured reasoning effort.
- **ACP verification.** Line 3 of root's
  `/Users/andreyyurlov/.codex/incant-evidence/claude-0066-acp.log` reads
  `Claude session verified: Opus 5.5; thinking: max`. That file is this session's live
  transport log, so its hash keeps changing; I read only that line.
- No subagent or auxiliary reviewer was used. No personal memory was edited.

## Host, adapter and toolchain

| Item | Value |
|---|---|
| OS | macOS 27.0.1 (26A434), Darwin 27.0.0, arm64 |
| GPU / adapter | Apple M5 Pro; wgpu reports `IntegratedGpu`, backend `Metal` (every review report) |
| Rust | rustc 1.99.0 (b940084d7 2026-09-28), LLVM 23.1.1; cargo 1.99.0 (5f94df478 2026-08-27) |
| Python | 3.14.8, standard library only, run with `-I -B` |
| Cargo target | `/Users/andreyyurlov/.codex/incant-core-presentation-target`, exclusive. Leased 02:42:56.87Z, released 02:53:20.68Z (`evidence/lease/target-lease.json`). Left in place as warm cache; nothing deleted |

At lease time no cargo/rustc process used the target. During the interval, the only
other related processes were Astra's CI checkpoint watcher (Python, no GPU) and this
ACP session.

## Source and frozen binaries

- **Source.** HEAD is the granted commit `96bb23df`. Every child rechecked:
  - the grant;
  - HEAD;
  - an empty `git status` over `crates`, `Cargo.lock`, `Cargo.toml`,
    `rust-toolchain.toml`, `third_party` and `tools/cargo`;
  - the hashes of all 19 inputs in `docs/spikes/evidence/cooked-cluster-shaders.json`.
- **Grid WGSL.** `grid.wgsl` (`461606ed…`) is byte-identical to the parent of the
  source commit.
- **Review receipt.** The independent review receipt matches its manifest hash and
  says `accepted: true`.
- **Freeze record:** `evidence/build/source-freeze.json`.
- **Builds.** Every build used `./tools/cargo … --locked --offline --profile ci -j2
  --message-format=json-render-diagnostics`.
  - Feature identity comes from Cargo's own artifact messages for every non-test
    `incant_render` unit.
  - Each build used one `incant_render` build-script output set, which was hashed
    (`evidence/build/{tests,review}-*.json`).

| Variant | `incant_render` features (Cargo artifact) | lib tests | `model_gpu` | `gpu_host` game / ui | `cluster_review` |
|---|---|---|---|---|---|
| baseline | cooked-output, -geometry, -sprites, -ui, -text, -materials, physics-deterministic | `c2b1adff…` | `24578d77…` | `e217a4c3…` / `e8a87ae0…` | `dedd2296…` |
| candidate | the same plus cooked-clusters | `a8d7d2fb…` | `c1c8dc84…` | `2abb9bc1…` / `9dc6bb6a…` | `ebe78919…` |

- **Authoring tools** (default features): `incant_headless` `d3a2e53e…`, and
  `cook_scene` `940529cf…`, which is byte-identical to 0064's.
- **Actual ci artifacts.** The candidate's ci-profile build script wrote three cluster
  artifacts, each matching the manifest's separately recorded ci hash:
  - Metal `1d501c9d…` (7,848 B);
  - Vulkan `b5b5e4eb…` (9,760 B). This is the ci SPIR-V, not the dev-profile
    `9cd2cf87…`;
  - DX12 `e7e4401e…` (6,998 B).
- **Other cooked families.** All 49 non-cluster build-script outputs are byte-identical
  between the variants: diagnostic, material, output, shadow, sprite, text and UI
  artifacts plus manifests. The baseline has no cluster output.
  - The review-helper builds produced exactly the same outputs as the test builds.
  - Record: `evidence/build/embed-*.json`.
- **Embedded shaders** (byte search of every frozen binary):
  - Baseline binaries contain `grid.wgsl` once and no cooked cluster artifact.
  - Candidate binaries contain each of the three ci artifacts once and no `grid.wgsl`.
    The one exception is the candidate library-test binary: its cfg(test) hash check
    in `lighting/cooked.rs` `include_str!`s the WGSL.
- **Review helper.** `tools/review` is a packet-local crate in its own workspace, so the
  repository manifests are untouched.
  - Its `Cargo.lock` is a pruned copy of the repository lock: 333 entries, with no
    foreign entry, checksum mismatch or added dependency edge
    (`evidence/build/helper-lock-*.json`).
  - Both variants were built from the same helper sources, and the committed sources
    equal them (`package.py --check` verifies this).
  - The helper was rustfmt-clean as built and was not edited after it ran.

## GPU interval, ledger and release

All GPU work went through `tools/gpu.py`.

- **Checks before each child.** Under an exclusive `flock`, it checked the grant
  (packet, source commit, target), the frozen source as above, the absence of this
  packet's release latch, and the frozen binary SHA-256. It checked the binary hash
  again after the run.
- **Isolation.** Each child ran alone in its own process group with a clean
  environment: no `INCANT_`, `RUST_`, `WGPU_`, `CARGO_`, `MTL_` or `METAL_` variables,
  so no test wrote optional evidence PNGs.
- **Limits.** 300 s per suite, 120 s per capture and 60 s per listing. Survivors were
  killed and every child was reaped.
- **Ledger.** `evidence/gpu/ledger.jsonl` (`5946821d…`) records 23 children: 8
  listings, 10 test processes and 5 review captures. All exited 0, none timed out and
  no binary changed.
- **Interval.** The first child started at 02:47:03.16Z and the last ended at
  02:52:36.28Z.
- **Frames.** 13 PNGs were written, all 960×540, against a budget of 24: 6 baseline, 6
  candidate and 1 source-free.
  - Every other frame was read back and hashed in memory only. That is 289 helper
    renders in total, including controls and dry runs.
  - The test suites render their own internal frames.
  - I counted the cap as retained captures, the reading root accepted for 0064 and
    0065. Under a stricter reading that counts every render, this packet would exceed it.
- **Release.** `evidence/gpu/released.json` (`b5d56b5a…`), written 02:52:50.41Z: no
  owned process, no ledger PID alive and the grant hash unchanged.
- **After release.** `test`, `list`, `review` (with a plan never run for that variant)
  and a second `release` all refused before creating any child or log.
  - The ledger hash and log count were unchanged.
  - The three refusals are in `evidence/gpu/refusals.jsonl`.
  - Everything after that was CPU-only.
- **Lease.** Released after a CPU-only Clippy pass on the helper. No 0066 cargo/rustc
  process remained and no process referenced the target or frozen binaries.

## Numerical GPU tests (exact runs)

Command form: `<frozen binary> --ignored --test-threads=1 --nocapture [filter]`.

| Suite | Baseline | Candidate |
|---|---|---|
| `incant_render` library | 32 completed, 1.67 s (PID 37516) | 32 completed, 1.53 s (PID 38795) |
| `model_gpu` | 50 completed, 45.53 s (PID 37521) | 50 completed, 45.59 s (PID 38800) |
| `gpu_host` game | 5 completed, 0.58 s | 5 completed, 0.57 s |
| `gpu_host` UI | 2 completed, 0.54 s | 2 completed, 0.54 s |
| **Total** | **89 completed; 0 failed, 0 skipped, 0 refused** | **89 completed; 0 failed, 0 skipped, 0 refused** |
| Isolated `lighting::gpu_tests::` (extra) | 3 completed, 0.41 s | 3 completed, 0.41 s |

- **Case lists.** The exact cases were listed and retained before running
  (`evidence/logs/*-list.stdout.txt`). They are identical across variants.
- **Source union.** The source union has 91 `#[ignore]` sites under `crates/`:
  - 32 compile into the `incant_render` library: `src/*_tests.rs` plus the `#[path]`
    modules `tests/{diagnostic_output,hdr_output,frame_graph,lighting_gpu}`;
  - 50 compile into `model_gpu`, 5 into the game suite and 2 into the UI suite;
  - the remaining 2 are `crates/incant_headless/tests/model_gpu.rs`, the legacy
    document Play recorder, which is outside the brief's four suites and was not run;
  - nothing is cfg-gated differently between the variants (`analysis.json`
    `source_union`).
- **Skipped means none.** No GPU test has an internal skip path. The only early returns
  gate optional evidence writers, and those are disabled by the clean environment.
- **Log differences.** Logs differ across variants only in these places:
  - the filtered-out count: 75 vs 74 for the candidate library, because of its extra
    `lighting/cooked.rs` descriptor unit test;
  - timings;
  - `model_gpu`'s printed preparation time: 2.28 ms vs 2.03 ms, with 31 render
    pipelines each;
  - the thread ID in the game suite's deliberate unwind test.
- **What the suites cover here:** clustered masks across mask words, all 4096 local
  bits against the unculled oracle, the last mask bit after a directional prefix,
  orthographic and inherited cameras with custom clusters, viewport offsets, depth
  slices and word-count changes. They also cover the full material, shadow,
  environment, sprite, text and UI suites.

## Cluster count/mask readbacks (numerical membership, independent of frames)

`crates/incant_render/tests/lighting_gpu/mod.rs` (`eb7fae99…`) compiles into the
library as the cfg(test) module `lighting::gpu_tests`.

**How the readback works.** Its `grid_readback` runs the production
`LightingSystem::prepare` and `ClusterPass::encode`. It then copies the cached
`GridBuffers` counts and masks (allocated `COPY_SRC`) into a `MAP_READ` buffer. The
viewport is 320×192, giving a 5×3×24 = 360-cluster grid, and the camera is
`CameraView::preview()` (perspective).

The tests ran in both variants, in the full suites and in isolation.
`analysis/analysis.json` → `readback_oracle` reconstructs the exact arrays they
assert:

| Test case | Lights | What the test asserts against the real readback | Coverage |
|---|---|---|---|
| `masks_represent_every_light…` | 31, 32, 33, 63, 64, 65, 127, 128, 129, 4095, 4096, 65, 1 point lights, range 10000 at the origin | every cluster count = N; every mask word = its full/tail bit pattern | complete arrays (counts 360, masks 360 × words) |
| `every_depth_slice…` (1) | 1 directional + 24 tiny local lights, one per depth slice | the 24 named clusters count 1 with mask `1 << z` (directional excluded from local bits); counts sum to 24 | counts complete; masks only at the 24 named clusters |
| `every_depth_slice…` (2) | 1 local light at x = 1e6, after (1) | every count and mask word zero (stale membership cleared) | complete |

- **Result.** Each case's assertions held for the real readback in both variants, so the
  baseline and candidate arrays are identical within each case's coverage. A pass
  cannot show differences the test does not assert, such as unasserted mask words.
- **Directional-only control.** `prepare` returns `compute: None` when there is no local
  light, so neither variant dispatches the cluster pass and nothing exists to read back.
  The frame review covers that path: frames D and C-t35/t36.
- **Gap.** These readbacks use one viewport, one camera and degenerate or axis-aligned
  layouts. They cannot cover moving, resized, viewport-offset, orthographic or spatially
  varied membership. See "Request to Astra".

## Fixtures (public command-bus authoring)

- **Art.** `tools/art.py` is a byte-identical copy of 0064's generator. It produces core
  glTF 2.0 assets with no extensions; the boards use `floor-gray`, `sphere-white`,
  `cube-white` and the uniform `sky-dim.png`.
- **Projects.** `tools/author.py` uses only frozen `incant_headless` commands:
  - `init`;
  - `import`;
  - `rpc`: `project.read`, then one `command.execute` of `create_entity` ops per board,
    then `project.save`;
  - `validate`.

  It then cooks with `cook_scene`. The script never writes project or journal JSON.
  It made 17 engine calls, all exited 0, with tool and source hashes checked before
  each call and tool hashes after. Both projects validated with no diagnostics.

| Board | Content | Entities | Cooked scene (bytes / SHA-256) |
|---|---|---|---|
| C | Floor, 5 spheres, 3 pillars, dim directional key (a directional prefix), dim uniform environment. 4096 local lights in a 64×64 carpet: 3072 point and 1024 down-facing spot. 4063 carry a Velocity; 33 are static | 4109 | 775,485 / `bb895850…` |
| D | Floor, sphere, cube, one directional light; no local light and no environment (minimal directional-only control) | 5 | 949 / `a2eb5805…` |

- **Boundary lights.** The 33 static point lights were selected by projecting with the
  renderer's conventions (`look_to_rh`, vertical-FOV `perspective_rh`, pixel Y down)
  at 960×540.
  - Each centre lies 0.1–3.7 px from a 64-px tile edge.
  - The corner light `light-17-30` sits at (446.9, 257.4), next to tile corner
    (448, 256).
  - Depths are 8.5–29.2 m. Four lights lie within 0.03–0.25 m of a logarithmic slice
    bound, where their 0.55–0.85 m ranges straddle two depth slices.
  - Records: `fixtures/r1/boards/C.json.gz` → `boundary_lights`.

## Rendered comparisons

The helper (`tools/review/src/main.rs`) is adapted from 0064's `material_review`.

- **Loading.** It loads each cooked scene into `NativeWorld` and `NativeRenderInput`,
  as a shipped world does.
- **Rendering.** It submits frames through `RetainedScene::submit` into an offscreen
  `Rgba8UnormSrgb` target, optionally inside an offset `Viewport` rectangle.
- **Hashing.** It reads each frame back and hashes the whole frame and every row.
- **Changes.** Velocity moves lights each tick. Runtime despawns force topology rebuilds
  (replacement retained scenes).
- **Controls.** These use the public non-retained `draw_scene` with the frame's own
  viewport.
- **Limits.** Derived lengths come from the allocation rules; they are not read back.

| Frame | Update | Dir + local | Grid, mask words | Derived lights / counts / masks bytes | Baseline = candidate | RGBA | PNG |
|---|---|---|---|---|---|---|---|
| C-persp t0 960×540 | prepared 1 | 1 + 4096 | 15×9×24, 128 | 262,208 / 12,960 / 1,658,880 | identical | `7e226543` | `C-persp-n4096-t000.png` |
| C-persp t15 | motion | 1 + 4096 | 15×9×24, 128 | same | identical | `18fffe4f` | |
| C-persp t30 | motion | 1 + 4096 | 15×9×24, 128 | same | identical | `9986f5b8` | `C-persp-n4096-t030.png` |
| C-persp t30 640×360 | resize | 1 + 4096 | 10×6×24, 128 | 262,208 / 5,760 / 737,280 | identical | `e159eb79` | |
| C-persp t30 333×197 | resize | 1 + 4096 | 6×4×24, 128 | 262,208 / 2,304 / 294,912 | identical | `4e98c3da` | |
| C-persp t30 960×540, viewport (37, 41, 640×360) | offset viewport | 1 + 4096 | 10×6×24, 128 | 262,208 / 5,760 / 737,280 | identical | `d81ec3ab` | `C-persp-n4096-t030-viewport.png` |
| C-persp t30 960×540 again | back to 15×9 | 1 + 4096 | 15×9×24, 128 | as t30 | identical; = first t30 | `9986f5b8` | |
| C-persp t31 (4063 despawned) | rebuilt, 2 | 1 + 33 | 15×9×24, 2 | 2,176 / 12,960 / 25,920 | identical | `5171e30d` | `C-persp-n33.png` |
| C-persp t31 512×288 | resize | 1 + 33 | 8×5×24, 2 | 2,176 / 3,840 / 7,680 | identical | `e1fb3409` | |
| C-persp t32 | rebuilt, 3 | 1 + 32 | 15×9×24, 1 | 2,112 / 12,960 / 12,960 | identical | `36ab3f14` | |
| C-persp t33 | rebuilt, 4 | 1 + 31 | 15×9×24, 1 | 2,048 / 12,960 / 12,960 | identical | `35ef134c` | |
| C-persp t34 | rebuilt, 5 | 1 + 1 | 15×9×24, 1 | 128 / 12,960 / 12,960 | identical | `4123f3fb` | |
| C-persp t35 | rebuilt, 6 | 1 + 0 | unclustered 1×1×24 | 64 / 96 / 96 | identical | `91ceeba9` | |
| C-persp t36 | motion | 1 + 0 | unclustered | same | identical; = t35 | `91ceeba9` | |
| C-ortho t0, t30 960×540 | motion | 1 + 4096 | 15×9×24, 128 (orthographic) | 262,208 / 12,960 / 1,658,880 | identical | `9121b34b`, `bdfdec64` | t30: `C-ortho-n4096-t030.png` |
| C-ortho t30 480×270 | resize | 1 + 4096 | 8×5×24, 128 | 262,208 / 3,840 / 491,520 | identical | `c06d9467` | |
| D t0 960×540, 480×270 | — | 1 + 0 | unclustered | 64 / 96 / 96 | identical | `5946d501`, `35037293` | `D-directional-only.png` |
| C-persp t0 again (after ortho and D) | — | 1 + 4096 | 15×9×24, 128 | as t0 | identical; = first t0 | `7e226543` | |

- **Cross-variant.** 20 of 20 frame states are identical in both the main and the dry
  runs. All 6 PNG pairs are byte-identical, carrying only IHDR, IDAT and IEND, and
  every decoded PNG equals the helper's readback hash.
- **Cross-process.** Each variant's dry run matched its main run, 20 of 20.
- **Controls** (exactness only): retained repeat, non-retained clustered and
  non-retained all-lights were exact on 17 frames × 3 controls per variant, including
  the offset viewport, every resize, 4096/33/32/31/1/0 lights and the orthographic
  camera.
  - The all-lights reference does not read the cluster buffers. Its exact match means
    no cluster omitted a light that reaches a pixel.
  - It cannot detect conservative extra members, by design.
- **Motion and despawns.** At a fixed 15×9 grid:
  - consecutive motion frames differ in 347 and 353 rows;
  - each despawn step changes the frame: 353, 2, 5, 233 and 14 rows. The 33rd and
    32nd lights are distant grazing pools, hence the 2 and 5 rows;
  - with no light left, t35 → t36 is unchanged.
- **Resize/viewport churn.** The t30 frame rendered again after 640×360, 333×197 and the
  offset viewport is identical to the first. C-persp t0 after the orthographic and D
  steps is identical to the first t0.
- **Viewport framing.**
  - Every pixel outside the rectangle is the canvas colour (20, 21, 25).
  - 303 of the 360 interior rows are byte-identical to the standalone 640×360 frame.
    That frame was hashed only, so the size of the other 57 rows' differences is not
    measured here. They are identical in both variants, and the retained and
    non-retained viewport frames match exactly.
  - The engine's own `clustered_viewport_offsets…` test bounds this offset-versus-plain
    difference at ≤ 1 code, and it passed in both variants.
- **Source-free.** After deletion, the candidate rendered C-persp t0, t30 and t31 (33
  lights), C-ortho t30 and D t0, all 5 identical to the main frames.
  `C-persp-n33-source-free.png` is byte-identical to `C-persp-n33.png`.
  - **Exact deletion boundary** (`fixtures/r1/source-free-{C,D}.json`): each project's
    `<key>.incant.json`, `<key>.incant.journal.jsonl` and its whole `art/` directory
    (the glTF, `.bin` and PNG sources) were privately copied, verified and deleted.
  - **What remains:** the cooked scenes (`fixtures/r1/cooked/{C,D}.scene`, outside the
    project directories), the `.incant/cache` model and texture caches (all unchanged)
    and the command bus's empty, 0-byte `.incant-source-transactions/lock`.
  - That lock is why the inherited `no_authoring_or_art_left` predicate (copied from
    0064, which checks for a `.incant/` prefix) reads `false`.
- **Self-checks** (`analysis/selfcheck.json`):
  - a 1-LSB change to one pixel is reported as exactly one pixel with delta 1;
  - all 13 captures round-trip exactly and equal the helper's readback hash;
  - the reconstructed oracle has the asserted tail-bit layout, and a one-bit change
    alters its hash.

## Appearance verdicts (personal pixel inspection)

I viewed all six captures at native size and six tile-aligned nearest-zoom crops
(`evidence/derived/`). Each crop has white ticks marking the 64-px tile edges in an
added margin; the crop pixels themselves are unaltered. Baseline and candidate frames
are byte-identical, so each verdict applies to both.

- **Populated cluster boundaries: pass.**
  - All 33 boundary pools are smooth, foreshortened ellipses. Those straddling tile
    edges continue across them without any step. Examples:
    - the corner light at x = 448, 4× crop `n33-corner-light`;
    - green pools at x = 448 and 512, `n33-near-row`;
    - pink and blue pools at x = 576 and 704, `n33-horizontal-edge`.
  - A contrast-stretched scratch view with the lattice overlaid showed no tile-aligned
    discontinuity anywhere.
  - It did show one large soft ellipse on the lower-right floor. Sampling shows it is a
    one-code contour (floor 27,29,34 vs 26,28,34): 8-bit quantization of smooth
    view-dependent shading, not a lighting gap. The scratch view stays in the ignored
    `scratch/` directory, hashed only.
- **4096-light density: pass.** The full carpet of overlapping coloured point pools and
  tight warm/cool spot pools has no rectangular cut-offs, missing pools or tile seams at
  any depth (crops `n4096-t000-centre`, `n4096-t030-centre`). Spheres and pillars pick
  up coloured light from below.
- **Retained motion: pass.** From t0 to t30 the regular lattice disperses as each light
  drifts 0.3–0.5 m in its own direction. Pools stay round and continuous across tile
  edges, and sphere lighting changes with them.
- **Projection: pass.** The orthographic top-down view places the four visible spheres
  and the nearest pillar top where the board geometry predicts. Pools are uniform in size, as
  parallel projection implies, and there are no seams (`ortho-t030-centre`).
- **Viewport: pass.** The scene is cleanly clipped to the offset rectangle on a uniform
  canvas, framing the same view as the full frame.
- **Minimal/directional-only control: pass.** The sphere and cube are lit from the upper
  right as authored, with a darker left cube face and a uniformly lit floor. There are
  no shadows, as authored.
- **Shared traits, not regressions:** no MSAA, and no shadows from local lights (only
  directional lights cast shadows in this engine).

## Request to Astra: cfg(test) cluster readback observer (not implemented)

**Why.** Direct, byte-exact baseline/candidate comparison of raw counts/masks beyond the
existing tests needs new raw-buffer access. The brief asked me to report that seam
first, so I report it here and changed no engine file.

**Where.** `crates/incant_render/tests/lighting_gpu/mod.rs`, which
`crates/incant_render/src/lighting/mod.rs:6-8` already compiles as
`#[cfg(test)] mod gpu_tests` inside `lighting`. No production or public change is
needed.

**What exists.** That module can already reach `Renderer.lighting`, the private
`LightingSystem::cache` (`Mutex<Option<Arc<GridBuffers>>>`, `mod.rs:105`), the private
`buffers()` and `GridBuffers::{counts, masks}`, which are created
`STORAGE | COPY_SRC` at `mod.rs:255-268`.

**Proposed shape:**

1. **Generalize the helper.** Turn `grid_readback` into
   `grid_readback_at(r, &GpuLights, viewport: [f32; 4], camera: CameraView)`. The body
   stays the same, but viewport and camera become parameters instead of the constants
   `[0, 0, 320, 192]` and `CameraView::preview()`.
2. **Add an observer of the last prepared grid.** It should clone
   `r.lighting.cache.lock()?.as_ref().map(Arc::clone)` and copy that grid's counts and
   masks after the caller's own submission, so `RetainedScene::submit` can be observed.
   It must not call `buffers()`: a mismatched key replaces the cache with fresh
   zero-filled buffers and would read back false zeros.
3. **Add one `#[ignore]` GPU test.** It should run:
   - local counts {1, 31, 32, 33, 4096}, with a directional prefix and seeded,
     spatially varied positions and ranges (for example `tests/lights/masks.rs`'s 4096
     layout);
   - viewports 320×192, 640×360, 960×540, 333×197 and offset `[37, 41, 640, 360]`;
   - the preview, an authored perspective and an authored orthographic `CameraView`;
   - a retained sequence: `prepare_retained`, then `submit` with moved light transforms
     at an unchanged size, asserting `Arc::ptr_eq` on the cached grid across moves,
     followed by a resize;
   - a directional-only control asserting `compute.is_none()`.
4. **Assert invariants per configuration.** For every cluster:
   - `counts[c]` equals the total `count_ones()` of that cluster's mask words;
   - no bit at or beyond the local count is set.

   Then print one line, for example:
   `cluster-readback <config> grid=WxHx24 words=N counts_sha256=… masks_sha256=…`.

Running that test in the baseline and candidate lib-test binaries compares
masks/counts exactly, independent of frames. The frozen 0066 binaries and fixtures
remain under `/Users/andreyyurlov/.codex/incant-evidence/0066` for a follow-up.

## Defects and observations

- **Production code.** No defect found, so nothing is reported for Astra to fix.
  Nothing showed a violation of the brief's contracts:
  - Metal compute bindings 0–3 plus the binding-4 lengths uniform are exercised by every
    clustered frame and readback above.
  - Fragment binding 4 remained the shadow texture: the candidate's shadow-cascade
    tests in `model_gpu` passed.
  - The 516-byte static workgroup storage and the 64×1×1 dispatch are consistent with
    the pinned wgpu-hal passthrough path noted below, and their results are exact.

  I did not inspect the Metal pipeline state directly.
- **Observation for Astra (not a defect here).**
  - Pinned wgpu-hal 29.0.4 compiles passthrough MSL with a default `MTLCompileOptions`:
    no language version and no `preserveInvariance` (`metal/device.rs:1175-1179`).
  - WGSL modules get the adapter's `msl_version` and `preserveInvariance`
    (`metal/device.rs:227-233`).
  - The passthrough compute pipeline is created with `wg_memory_sizes: []`,
    `sized_bindings: []` and the cooked `num_workgroups` as its threadgroup size
    (`metal/device.rs:1686-1700`). That matches the packet's static-workgroup and
    explicit-lengths design.
  - Every cluster result here is bit-identical on this M5 Pro with macOS 27.0.1; other
    Apple GPUs and OS versions were not tested. This is the same compile-options
    question 0064 raised for materials.
- **Packet helper (mine).** `cargo clippy -D warnings` reports exactly three
  `clippy::chunks_exact_to_as_chunks` style lints, inherited unchanged from 0064's
  helper code (`tools/review/src/main.rs` lines 485 and 502, two at 502;
  `evidence/build/helper-clippy.txt`), and nothing else. I left the executed helper
  unchanged.

## Screenshot paths

The committed captures are unedited candidate PNGs. Each is byte-identical to its
baseline file:

- `evidence/captures/C-persp-n4096-t000.png` (`cdfab98b…`)
- `evidence/captures/C-persp-n4096-t030.png` (`1b1e2a16…`)
- `evidence/captures/C-persp-n4096-t030-viewport.png` (`a9281535…`)
- `evidence/captures/C-persp-n33.png` (`088a796a…`)
- `evidence/captures/C-ortho-n4096-t030.png` (`b1455487…`)
- `evidence/captures/D-directional-only.png` (`b227eae4…`)

The source-free PNG has the same bytes as `C-persp-n33.png` and is recorded by hash
only.

Derived figures are CPU crops of those captures, not renders: the six files in
`evidence/derived/*.png`, described in `evidence/derived/figures.json`. All 13
captures, every report and the crops remain under
`/Users/andreyyurlov/.codex/incant-evidence/0066/{captures,analysis/figures}`.

## Commands run (from the worktree root; all exited 0 unless noted)

```
T=handoffs/0066-cooked-cluster-shaders/tools; E=/Users/andreyyurlov/.codex/incant-evidence/0066
python3 -I -B $T/lease.py acquire
python3 -I -B $T/build.py freeze
python3 -I -B $T/build.py tests-baseline | tests-candidate | tools | helper-lock
python3 -I -B $T/build.py review-baseline | review-candidate
python3 -I -B $T/build.py embed
python3 -I -B $T/author.py build $E/fixtures/r1
python3 -I -B $T/plan.py $E/fixtures/r1
# GPU (serial, guarded)
python3 -I -B $T/gpu.py list <baseline|candidate> <lib|model_gpu|gpu_host-game|gpu_host-ui>
python3 -I -B $T/gpu.py test <baseline|candidate> <suite> all
python3 -I -B $T/gpu.py test <baseline|candidate> lib cluster-readback lighting::gpu_tests::
python3 -I -B $T/gpu.py review <baseline|candidate> main-dry, then main
python3 -I -B $T/figures.py $E $E/analysis/figures                       # CPU crops, during the interval
python3 -I -B $T/author.py source-free $E/fixtures/r1 <C|D>
python3 -I -B $T/gpu.py review candidate source-free
./tools/cargo fmt --manifest-path $T/review/Cargo.toml -- --check          # helper as built: clean
python3 -I -B $T/gpu.py release
python3 -I -B $T/gpu.py test|list|review|release …                          # refusal checks: exit 1, no child/log
# CPU after release
CARGO_TARGET_DIR=<target> ./tools/cargo clippy --manifest-path $T/review/Cargo.toml --bin cluster_review --features <candidate> --locked --offline --profile ci -j2 -- -D warnings   # exit 101: 3 style lints
python3 -I -B $T/lease.py release
python3 -I -B $T/analyze.py $E $E/analysis/analysis.json
python3 -I -B $T/selfcheck.py $E $E/analysis/selfcheck.json
python3 -I -B $T/package.py && python3 -I -B $T/package.py --check
git diff --check (on the staged packet)
```

- **Scratch inspection.** I also ran a one-off, contrast-stretched scratch rendering of
  `C-persp-n33.png`, using inline Python and `pngio` (CPU only). Its output is under
  the ignored `scratch/` directory, hashed only.
- **Frozen before first GPU use:** the compared binaries, the review helper binary, the
  fixtures and the plans.
- **Written after the GPU runs:** `figures.py`, `selfcheck.py` and `package.py`. These
  are CPU-only tools.
- **Not edited after use:** every other tool.
- **Ledger contents:** exact argv, PIDs, times and hashes.

## Changed paths

Packet only. Nothing outside `handoffs/0066-cooked-cluster-shaders/` changed in the
worktree.

- `handoffs/0066-cooked-cluster-shaders/result.md`
- `handoffs/0066-cooked-cluster-shaders/tools/`:
  - Python: `analyze.py`, `art.py`, `author.py`, `boards.py`, `build.py`, `common.py`,
    `figures.py`, `gpu.py`, `lease.py`, `package.py`, `plan.py`, `pngio.py`,
    `selfcheck.py`.
  - Helper crate: `review/{Cargo.toml,Cargo.lock,src/main.rs}`.
  - Copied or adapted read-only from 0064's committed tools: `art.py` and `pngio.py`
    are byte-identical bodies; the others are adapted. The boards, plans, viewport
    controls and the readback-oracle analysis are new.
- `handoffs/0066-cooked-cluster-shaders/evidence/`: 106 files plus `manifest.json`
  (4.1 MB).
  - Text: 10 logs are normalized for `git diff --check`; they drop only line-end
    whitespace and a final blank line.
  - Large JSON: 5 files over 200 KB are committed as deterministic gzip that decompresses
    to the raw bytes. These are board C's description and entities, the authoring
    record, and the two main reports.
  - Everything else is byte-exact.
  - Hash-only: 92 files, including binaries, Cargo JSON logs, cooked scenes, caches,
    art, private source copies, dry-run reports, the baseline and source-free PNGs and
    scratch.

The brief is Astra's, already committed, and unchanged. Root's ACP log and grant are
outside the worktree and were not modified. No production code, other handoff, CI,
account or memory was touched.

## Limitations and unavailable checks

- **Single backend.** One adapter (Apple M5 Pro), one OS (macOS 27.0.1), Metal only.
  The SPIR-V and HLSL cluster kernels were not executed.
  - No Vulkan, DX12, lavapipe, WARP, phone or Device Farm run.
  - No frame-time, performance, pipeline-cache, prewarming, hitch or reference-phone
    claim.
  - No phase gate.
- **Raw membership scope.** Raw counts/masks were compared only through the existing
  readback tests' exact oracles: one 320×192 viewport, the preview perspective camera,
  global-sphere and depth-slice layouts. The extended raw comparison waits on the
  requested observer.
- **Rendered checks.** Frame identity and the exact all-lights control prove complete,
  identical shading. By design they cannot reveal conservative extra cluster members.
- **Lengths are not read back.** The binding-4 lengths values are derived from the
  allocation rules, not read from the GPU. The evidence for them is behavioral:
  bit-identical frames and exact controls at every listed allocation, from 96 B to
  1,658,880 B of masks.
- **Offscreen only.** All frames come from the packet helper in the `ci` profile. No
  window, presentation path, editor or shipping build was used.
- **Not run:** the two `incant_headless` GPU tests, which exercise the legacy document
  Play recorder outside the four suites the brief names.
- **Tests are Astra's oracles.** I executed the existing suites. Visual acceptance rests
  on my inspection, the cross-variant identity and the exact in-process controls.

## Open questions

1. Should Astra add the cfg(test) readback observer above, so a follow-up can compare raw
   counts/masks byte-for-byte at moving, resized, offset, orthographic and spatially
   varied configurations?
2. Should the passthrough MSL `MTLCompileOptions` (language version) be pinned before
   cooked clusters reach other Apple GPUs or OS versions? Parity held on this device.
3. Should a later packet execute the SPIR-V and HLSL cluster kernels (lavapipe and WARP
   in CI, or hosted devices) before any default enablement?
