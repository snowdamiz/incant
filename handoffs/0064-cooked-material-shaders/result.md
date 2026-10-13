# 0064 result — fixed material backend acceptance (Metal)

## Status

Complete within the packet scope, on one Mac Metal adapter.

- **Numerical GPU tests.** Every GPU-ignored test of `incant_render` (32 library, 50
  `model_gpu`) and `incant_render_gpu_host` (5 game, 2 UI) ran once per variant, one
  test thread per process: **89 of 89 passed on the baseline and 89 of 89 on the
  candidate.** Listings and per-test result lines are identical across variants. As an
  extra control, the repository-default build (every shader family on WGSL) also passed
  89 of 89.
- **Rendered comparisons.** The packet review helper rendered 20 distinct frame states
  per variant through the retained native path from cooked scenes. **All 20 are
  whole-frame identical across variants** (RGBA SHA-256 and every row hash), and all 10
  PNG pairs are byte-identical. A PNG-free dry run of the same plans repeated every frame
  hash in a second process per variant.
- **Controls.** In each variant, 17 frames were re-rendered three ways in-process
  (retained resubmission, non-retained clustered, non-retained all-lights reference):
  **51 of 51 controls exact (0 differing pixels) in both variants.**
- **Coverage.** Material maps, opaque/mask/blend, single/double-sided and mirrored
  instances, minification with and without mips, cascaded directional shadows and their
  distance fade, an equirectangular environment, 120 moving local lights, light counts
  120 → 60 → 32 → 0 (mask words 4 → 2 → 1 → unclustered), three cluster-grid sizes, a
  no-light minimal scene, an empty scene, reimport retention and source-deleted cooked
  execution.
- **Visual check.** I inspected all 10 distinct captures and the zoomed crops
  personally. The appearance is correct for every listed property.

**Numerical verdict:** pass. `incant_render/cooked-materials` matches the WGSL material
path exactly on this device: identical test outcomes and bit-identical frames, including
every storage-length configuration exercised through the explicit group 3 / binding 7
lengths uniform.

**Appearance verdict:** accepted for appearance parity of `incant_render/cooked-materials`
against the baseline material WGSL, on an Apple M5 Pro with macOS 27.0.1 and Metal.

I found no production defect, so none is reported for Astra to fix. This is not
Vulkan, DX12, phone, performance, pipeline-cache, default-enablement, full offline
renderer or phase-gate evidence. Cluster assignment compute remains runtime WGSL in both
variants.

## Brief, grant and director feedback

- **Brief.** SHA-256 `7f3ca4ec33cf577c20a23b258620184a52769e195239d9e2fce492cdc0fe2b46`,
  committed by Astra in `e0fbd056`. Re-checked before committing; unchanged, with no
  uncommitted root revision. It has no priority-revision or director-feedback section.
- **Grant.** `/Users/andreyyurlov/.codex/incant-evidence/0064/review-ready.json` (SHA-256
  `7a067eeb…72b42bf`, granted 01:01:42.909Z) names `0064-cooked-material-shaders` and the
  target. Its prior-release hashes match 0063's receipts exactly: `a327d470…` is 0063's
  `gpu-released.json` and `8269967a…` is 0063's `target-lease.json`. A byte copy is in
  `evidence/gpu/grant-copy.json`. I did not modify or delete the grant.

## Model and thinking effort

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** Max, the session's configured reasoning effort.
- **ACP verification.** Line 3 of root's
  `/Users/andreyyurlov/.codex/incant-evidence/claude-0064-acp.log` reads
  `Claude session verified: Opus 5.5; thinking: max`. The file is still being appended
  in this session, so its hash will change; I read only the verification line.
- No subagent or auxiliary reviewer was used. No personal memory was edited.

## Host, adapter and toolchain

| Item | Value |
|---|---|
| OS | macOS 27.0.1 (26A434), Darwin 27.0.0, arm64 |
| GPU / adapter | Apple M5 Pro; wgpu reports `IntegratedGpu`, backend `Metal` (every review report) |
| Rust | rustc 1.99.0 (b940084d7 2026-09-28), LLVM 23.1.1; cargo 1.99.0 (5f94df478 2026-08-27) |
| Python | 3.14.8, standard library only, run with `-I -B` |
| Cargo target | `/Users/andreyyurlov/.codex/incant-core-presentation-target`, exclusive. Leased 01:08:59.458Z, released 01:33:31.500Z (`evidence/lease/target-lease.json`). Left in place as warm cache; nothing deleted |

At lease time one unrelated `cargo rustc` (another agent's iOS shipping build in its
own private target under `authored-ios-wasm-shipping-001`) was running. It was CPU-only
and did not use this target; I left it alone.

## Source and frozen binaries

- **Source.** HEAD `e0fbd056`. Its only difference from source commit `6bcb839a` is the
  brief: `git diff 6bcb839a HEAD -- crates Cargo.lock Cargo.toml tools/cargo
  rust-toolchain.toml third_party` is empty.
  - All 33 inputs in `docs/spikes/evidence/cooked-material-shaders.json` match both the
    source commit and the worktree.
  - `model_material.wgsl`, `lighting/shade.wgsl` and `shadows/shade.wgsl` are unchanged
    from the parent of the source commit.
  - Record: `evidence/build/source-freeze.json`.
- **Builds.** Every build used `./tools/cargo … --locked --offline --profile ci -j2
  --message-format=json-render-diagnostics`. Feature identity was checked from Cargo's
  own artifact messages for every non-test `incant_render` unit (`evidence/build/*.json`).

| Variant | `incant_render` features | lib tests | `model_gpu` | `gpu_host` game / ui | `material_review` |
|---|---|---|---|---|---|
| baseline | cooked-output, cooked-geometry, cooked-sprites, cooked-ui, cooked-text | `73b7ab5e…` | `2108d52e…` | `976e5e2d…` / `ea6c969f…` | `c6501e54…` |
| candidate | the same plus cooked-materials | `b14c4934…` | `a3558280…` | `09e1c860…` / `2a170986…` | `25346f7d…` |
| default (extra control) | repository defaults (no cooked family) | `b1b6f333…` | `47fe4b55…` | `c42b1112…` / `cfda78a5…` | not built |

- **Authoring tools** (default features): `incant_headless` `ce0d27d2…`, `cook_scene`
  `940529cf…`.
- **Which material shader each binary carries** (byte search, `evidence/build/embed-*.json`):
  - Baseline and default binaries contain each of the three material WGSL files once and
    none of Astra's retained cooked material artifacts.
  - Candidate binaries contain no material WGSL. They contain each of the six retained
    artifacts (MSL, SPIR-V and HLSL, vertex and fragment) byte-for-byte, once each. The
    candidate library-test binary holds two copies: `ARTIFACTS` and `BUNDLE` are `const`
    items used both by the production path and by its unit test.
- **Pipelines.** The warmup test reports 31 prepared render pipelines in both variants.
- **Review helper.** `tools/review` is a packet-local crate in its own workspace, so the
  repository manifests are untouched. Its `Cargo.lock` was seeded from the repository
  lock and pruned offline by Cargo. Apart from the helper's own entry, all 332 entries
  match the repository lock's versions, sources and checksums, with no added dependency
  edges (`evidence/build/helper-lock-*.json`).

## GPU interval, ledger and release

All GPU work went through `tools/gpu.py`. Under an exclusive `flock`, before each child,
it checks the grant (packet and target), the absence of this packet's release latch, and
the frozen binary SHA-256, which it checks again after the run. Each child ran alone in
its own process group with a clean environment (no `INCANT_`, `RUST_`, `WGPU_`,
`CARGO_`, `MTL_` or `METAL_` variables). Limits: 300 s per test suite, 120 s per review
capture and 60 s per listing. Survivors were killed and every child was reaped.

- **Ledger.** `evidence/gpu/ledger.jsonl` (`a89dab77…`) records 34 children: 12
  listings, 13 test processes and 9 review captures. All exited 0, none timed out, and
  no binary changed.
- **Interval.** First child started 01:24:33.654Z; last child ended 01:31:51.848Z.
- **Frames.** 21 PNGs were written (budget 24), all ≤960×540: 9 + 9 main, 1 source-free
  and 1 + 1 shadow. All other frames were read back and hashed in memory only.
- **Unintended extra child.** My first release attempt failed, because its process check
  matched my own shell's command line, which mentioned the binary path. The refusal check
  I had chained after it then ran for real: `gpu.py test candidate lib
  after-release-check`. The label consumed the only argument, so the candidate library
  suite ran in full (PID 87356, 01:31:50.377–01:31:51.848Z, 32 passed, no frames).
  - It ran inside the grant, before any latch existed, and is ledgered.
  - I fixed the check to match only executables whose argv[0] lies under `bin/`.
- **Release.** `evidence/gpu/released.json` (`116562e1…`), written 01:32:19.131Z:
  - no owned process;
  - no ledger PID alive;
  - grant hash unchanged.
- **After release.** `test`, `list`, `review` (with a plan never run for that variant)
  and a second `release` all refuse before creating any child or log. The ledger hash and
  log count were unchanged, and no renderer was started afterwards. Everything later was
  CPU-only.

## Numerical GPU tests (exact runs)

Command form: `<frozen binary> --ignored --test-threads=1 --nocapture`.

| Suite | Baseline | Candidate | Default control |
|---|---|---|---|
| `incant_render` library | 32 passed, 1.80 s (PID 83379) | 32 passed, 2.02 s (PID 84785) | 32 passed, 1.61 s |
| `model_gpu` | 50 passed, 46.67 s (PID 83450) | 50 passed, 46.44 s (PID 84866) | 50 passed, 45.93 s |
| `gpu_host` game | 5 passed, 0.72 s | 5 passed, 0.60 s | 5 passed, 0.63 s |
| `gpu_host` UI | 2 passed, 0.63 s | 2 passed, 0.54 s | 2 passed, 0.55 s |
| **Total** | **89 / 89** | **89 / 89** (plus the repeat library run: 32 / 32) | **89 / 89** |

- **What `model_gpu` covers here:**
  - material maps, color roles and sRGB;
  - alpha modes, back faces and culling;
  - emissive samplers;
  - lights: clustered masks across mask words, all 4096 local bits, range edges, depth
    boundaries and the all-light oracle;
  - orthographic and inherited cameras with custom clusters;
  - environments (intensity, rotation, roughness, malformed replacement, source-free);
  - highlight energy and HDR;
  - minification;
  - shadow cascades, normal offset, masked casters and queued shadow maps;
  - tangents;
  - reimport and retained versions;
  - pipeline warmup.
- **What the library suite covers here:** HDR output, the three lighting GPU tests
  (viewport offsets, depth slices, word-count changes) and the retained native tests.
- **Other suites.** The two `incant_headless` GPU tests (legacy document Play recorder)
  were not run (see Limitations).
- **Logs.**
  - Library and UI stderr are byte-identical across variants.
  - `model_gpu` stderr differs only in the printed preparation time (6.24 ms vs 2.71 ms;
    31 pipelines each).
  - Game stderr differs only in the thread ID of the deliberate unwind test's panic.
  - The candidate lists 74 filtered (non-GPU) library tests against 72 for the baseline
    (the two cooked-materials unit tests) and 63 for the default (Astra's 63 default
    tests).

## Fixtures (public command-bus authoring)

- **Art.** `tools/art.py` writes deterministic core glTF 2.0 assets (no extensions; the
  importer rejects them). `TANGENT` is omitted, so the importer's MikkTSpace path supplies
  the normal-map basis.
  - Textures: base color checker, metallic-roughness gradient, normal domes, occlusion
    grooves, emissive rings, alpha mask with holes and a ramp, an orientation arrow, a
    1-texel minification checker, two panel versions, an equirectangular sky with
    azimuth markers and a sun, and a dim sky.
  - Meshes: sphere, quad, floor and cube.
  - Manifest SHA-256 `db87e661…`.
- **Projects.** `tools/author.py` uses only frozen `incant_headless` commands: `init`,
  `import` (`--texture-usage color` for environments), `rpc` (`project.read`, one
  `command.execute` of `create_entity` ops, `project.save`) and `validate`. It then cooks
  with `cook_scene`. The script never writes project or journal JSON.
  - Run r1: 51 engine calls; run r2: 12. All exited 0 with tool hashes checked before and
    after each call. Every project validated with no diagnostics.

| Board | Content | Entities | Cooked scene (bytes / SHA-256) |
|---|---|---|---|
| M | Checker, metallic-roughness map, chrome, red dielectric, blend glass over a cube, mirrored sphere, cube; normal, occlusion, emissive and mask (double-sided) quads; single-sided front, single-sided back (culled), double-sided back and mirrored arrow quads; mipmapped and unmipmapped floor halves; shadowed sun, warm point, cool spot; sky environment rotated 30° | 25 | 4,439 / `401b77e7…` |
| L | Floor, 15 spheres, 3 pillars, dim key, dim environment; 96 point and 24 spot lights, each with a Velocity | 143 | 26,097 / `55ea8a0a…` |
| Z1 | One checker sphere; no authored light or environment (preview key) | 2 | 421 / `c8a0fb83…` |
| Z0 | Camera only | 1 | 234 / `867a4c58…` |
| R | Textured panel and an unchanged sphere; panel image reimported | 4 | v1 762 / `4aa40b77…`; v2 762 / `cd4fbf06…` |
| S (run r2) | Low shadowed sun over a plain floor; 8 pillars from 3 m to 49 m away, two spheres, a masked caster, a mirrored cube; dim sky fill | 16 | 2,686 / `1bfb3207…` |

- **Reimport.** After cooking R-v1, I replaced `art/panel.png` with version 2 on disk and
  re-ran `incant_headless import` on `quad-panel.gltf`.
  - The bus kept asset `01M4SDNGG87E4E3RZ4KMR4NYDZ` and reported `changed: true`.
  - The model cache retained the old fingerprint `7e8a1f90…` and added `05fcab1f…`. The
    project re-validated, and R-v2 was cooked.
- **Source-free.** For boards M, L, Z1 and R:
  - The project file, journal and all art were copied privately, verified and deleted
    (`fixtures/r1/source-free-*.json`).
  - The cooked scenes and caches were unchanged. The only other file left is the bus's
    empty `.incant-source-transactions/lock`.
- **Board S.** Board S was added during the GPU interval: shadows on M were too faint to
  review on its busy floor. It was authored, cooked and planned before its own GPU runs.

## Rendered comparisons

The helper (`tools/review/src/main.rs`) loads each cooked scene into `NativeWorld` and
`NativeRenderInput`, as a shipped world does. It submits frames through
`RetainedScene::submit` into an offscreen `Rgba8UnormSrgb` target, reads them back and
hashes every whole frame and every row. Velocity moves lights each tick. Despawns force
topology rebuilds, which produce replacement retained scenes with fewer lights. PNGs are
written only for named frames.

| Frame | Lights (dir + local), mask words, grid | Derived lengths: lights / counts / masks (bytes) | Baseline = candidate | PNG SHA-256 |
|---|---|---|---|---|
| M-wide 960×540 | 1 shadowed + 2, 1 word, 15×9×24 | 192 / 12,960 / 12,960 | identical | `601c5c62…` |
| M-close 960×540 | same | same | identical | `ca20486d…` |
| L-persp t0 | 1 + 120, 4 words, 15×9×24 | 7,744 / 12,960 / 51,840 | identical | `234f4fd8…` |
| L-persp t45 | same, lights moved | same | identical | hash only |
| L-persp t90 | same, lights moved | same | identical | `5a646960…` |
| L-persp t90 640×360 | 4 words, 10×6×24 | 7,744 / 5,760 / 23,040 | identical | hash only |
| L-persp t90 320×180 | 4 words, 5×3×24 | 7,744 / 1,440 / 5,760 | identical | hash only |
| L-persp t91 (60 despawned; rebuild) | 1 + 60, 2 words | 3,904 / 12,960 / 25,920 | identical | hash only |
| L-persp t92 (28 more; rebuild) | 1 + 32, 1 word | 2,112 / 12,960 / 12,960 | identical | `e162922d…` |
| L-persp t93 (last 32; rebuild) | 1 + 0, unclustered 1×1 | 64 / 96 / 96 | identical | hash only |
| L-persp t94 | same | same | identical | hash only |
| L-ortho t0, t90 | 1 + 120, 4 words, orthographic clusters | 7,744 / 12,960 / 51,840 | identical | t90 `fc937fb7…` |
| Z1 480×270 | preview key only | 64 / 96 / 96 | identical | `4af3023b…` |
| Z0 320×180 | none; no material draw | n/a | identical | hash only |
| M-wide again (after L/Z churn) | as M-wide | as M-wide | identical | hash only |
| R-v1, R-v2, R-v1 again 480×270 | 1 directional | 64 / 96 / 96 | identical | `54421f72…`, `2c3bb4d4…` |
| S-shadows 960×540 | 1 shadowed | 64 / 96 / 96 | identical | `1ed6f1e5…` |

- **Lengths.** The "derived lengths" are what the allocation rules imply for the light
  buffer and the cached cluster buffers. The helper computes them; they are not read back
  from the GPU. The candidate binds these allocations through the explicit lengths
  uniform. Bit-identity with the WGSL path at every row, including 4-word masks at three
  grid sizes and the 96-byte unclustered case, shows the bounds-protected indexing never
  clamped a valid index differently.
- **Cross-variant.** 20 of 20 frame states are identical (19 main, 1 shadow). 10 of 10
  PNG pairs are byte-identical, carrying only IHDR, IDAT and IEND. Every decoded PNG
  equals the helper's readback hash.
- **Cross-process.** The dry runs (same plans, no PNGs) matched the PNG runs frame for
  frame in each variant: 19 of 19 and 1 of 1.
- **In-process controls** (bounded only by exactness): retained resubmission,
  non-retained clustered and non-retained all-lights reference were exact on 17 frames ×
  3 controls in each variant. The all-light reference skips the cluster buffers entirely,
  so its exact match is direct evidence that the cluster counts and masks read through the
  candidate's lengths were complete.
- **Light motion.** Consecutive L frames differ (419 rows for t0 → t45 → t90; 412 and
  371 rows across the despawn rebuilds). t93 → t94 is unchanged, because no light
  remains to move. Retained scenes prepared: 1, then 2, 3 and 4 after each rebuild.
- **Resource churn.** M-wide rendered again after the L, Z and R steps is identical to
  the first M-wide.
- **Reimport retention.** In each variant, R-v1 rendered after R-v2 had been prepared is
  identical to the first R-v1, and the old cooked version still loads after the reimport.
  R-v1 and R-v2 differ in 37,390 pixels, all inside the panel's box (104,37)–(299,232);
  the unchanged sphere outside it is identical. In-process retention of an
  already-prepared scene across reimport is covered by `model_gpu`'s
  `color_maps_use_srgb_and_texture_reimport_keeps_retained_materials`, which passed in
  both variants.
- **Source-free.** After source deletion, the candidate rendered M-wide, M-close,
  L-persp (t0 and t90), Z1, R-v1 and R-v2: 7 of 7 identical to the pre-deletion frames.
  The source-free M-wide PNG is byte-identical to `M-wide.png`.
- **Self-checks** (`analysis/selfcheck.json`): a single 1-LSB change is reported as one
  pixel with delta 1, and all 21 captures round-trip exactly through the stdlib decoder
  and encoder.

## Appearance verdicts (personal pixel inspection)

I viewed all 10 distinct captures at native size and 11 nearest-zoom crops
(`tools/figures.py`; 3 are committed). Baseline and candidate frames are byte-identical,
so each verdict applies to both.

- **Material maps:** pass.
  - The checker base color reads in sRGB with correct orientation.
  - The metallic-roughness sphere splits cleanly between a reflective metal top and a
    dielectric bottom, rougher to one side.
  - Normal-mapped domes shade consistently with the lights.
  - Occlusion grooves darken only the ambient contribution.
  - Emissive rings glow on a dark base.
- **Environment:** pass. The chrome sphere reflects the 30°-rotated sky: blue zenith,
  warm horizon, the colored azimuth markers and the sun highlight. Grazing-angle Fresnel
  slivers of the green marker appear on dielectric silhouettes, as expected.
- **HDR and tone mapping:** pass. Sun and spot highlights compress smoothly to white
  without hue shifts. Saturated emissive colors stay saturated.
- **Alpha:** pass.
  - Masked holes cut cleanly, revealing the red sphere and floor behind.
  - The blend sphere tints the cube behind it.
  - The masked caster's shadow keeps the hole pattern.
- **Culling and reflection:** pass.
  - The single-sided back face is absent.
  - The double-sided back face and the mirrored instance both show the arrow reversed,
    with the red marker top-right, and are lit like front faces.
  - The mirrored sphere and cube render outward, not inside-out.
- **Minification:** pass for both sampler modes as authored. The mipmapped floor half
  converges to a smooth average with distance. The half authored with `minFilter` LINEAR
  (no mips) shows strong moiré, which is that sampler's expected behavior, not a defect.
- **Many lights and clusters:** pass. The 120 colored pools are smooth, with no
  64-pixel cluster-tile seams, no abrupt range edges and no missing pools at any depth,
  in both perspective and orthographic views. Motion and the despawn steps change exactly
  the expected pools; the 12 warm spots remain after t92.
- **Shadows and cascades:** pass.
  - The near casters' shadows have soft 3×3 PCF edges, contact the floor and show no
    acne or peter-panning.
  - Far shadows get thinner and softer.
  - Casters beyond the 40 m shadow distance cast no visible shadow.
  - No cascade seams are visible.
- **Minimal and empty scenes:** pass. Z1 is lit by the preview key and studio
  environment. Z0 shows only the background color.
- **Reimport and source-free:** pass. The panel shows version 1 (blue stripes, one bar)
  or version 2 (orange squares, two bars) according to the cooked scene, and source-free
  frames are identical.

## Defects and observations

- **Production code.** No defect found, so nothing is reported for Astra to fix.
- **Observations for Astra** (not defects here):
  - As with sprites in 0062, pinned `wgpu-hal` 29.0.4 compiles passthrough MSL with
    default `MTLCompileOptions`: no `preserveInvariance` and the system-default language
    version. WGSL modules get the adapter's version and `preserveInvariance`. Neither path
    sets fast-math options. Every material frame here is bit-identical on this M5 Pro with
    macOS 27.0.1; other Apple GPUs and OS versions were not tested.
  - The lengths uniform is a new 16-byte `create_buffer_init` on every `prepare`. That
    matches the existing per-frame grid uniform and is not a correctness issue.
- **Packet helper (mine).**
  - `cargo clippy -D warnings` reports exactly three `clippy::chunks_exact_to_as_chunks`
    style lints (`tools/review/src/main.rs` lines 423 and 440, two at 440;
    `evidence/build/helper-clippy.txt`), and nothing else.
  - I did not change helper logic after it executed, so the committed helper stays the
    one that ran. Its source was rustfmt-formatted after execution, whitespace only:
    rustfmt of the as-built copy (`evidence/build/review-main.rs.as-built`, SHA-256
    `08b4aebe…`, matching both build records) reproduces the committed file
    (`ad6095e1…`) exactly.

## Screenshot paths

Committed captures are unedited candidate PNGs, each byte-identical to its baseline
file:

- `evidence/captures/M-wide.png`
- `evidence/captures/M-close.png`
- `evidence/captures/L-persp-t000.png`
- `evidence/captures/L-persp-t090.png`
- `evidence/captures/L-persp-t092.png`
- `evidence/captures/L-ortho-t090.png`
- `evidence/captures/Z1.png`
- `evidence/captures/R-v1.png`
- `evidence/captures/R-v2.png`
- `evidence/captures/S-shadows.png`

The source-free M-wide PNG is the same bytes as `M-wide.png` and is recorded by hash only.

Derived figures are CPU crops of those captures, not renders:
`evidence/derived/{M-wide-culling-row,S-shadows-near,S-shadows-far}.png`, plus
`figures.json`. All 21 captures, every report and all 11 crops remain under
`/Users/andreyyurlov/.codex/incant-evidence/0064/{captures,analysis/figures}`.

## Commands run (from the worktree root; all exited 0 unless noted)

```
T=handoffs/0064-cooked-material-shaders/tools; E=/Users/andreyyurlov/.codex/incant-evidence/0064
python3 -I -B $T/lease.py acquire
python3 -I -B $T/build.py freeze
python3 -I -B $T/build.py tests-baseline | tests-candidate | tools      # first attempt: shell redirect into a missing logs/ dir, nothing ran
python3 -I -B $T/build.py helper-lock
python3 -I -B $T/build.py review-baseline | review-candidate
python3 -I -B $T/author.py build $E/fixtures/r1
python3 -I -B $T/plan.py $E/fixtures/r1
python3 -I -B $T/build.py embed
# GPU (serial, guarded)
python3 -I -B $T/gpu.py list <baseline|candidate|default> <lib|model_gpu|gpu_host-game|gpu_host-ui>
python3 -I -B $T/gpu.py test <baseline|candidate|default> <suite> all
python3 -I -B $T/gpu.py review <baseline|candidate> main-dry, then main
python3 -I -B $T/author.py source-free $E/fixtures/r1 <M|L|Z1|R>
python3 -I -B $T/gpu.py review candidate source-free
python3 -I -B $T/build.py tests-default                                   # extra control, built mid-interval
python3 -I -B $T/author.py build $E/fixtures/r2 shadow; python3 -I -B $T/plan.py $E/fixtures/r2 shadow
python3 -I -B $T/gpu.py review <baseline|candidate> shadow-dry, then shadow
python3 -I -B $T/gpu.py release            # exit 1 (self-matched shell); see "Unintended extra child"
python3 -I -B $T/gpu.py test candidate lib after-release-check   # ran 32 tests before the latch existed
python3 -I -B $T/gpu.py release            # exit 0 after the fix
python3 -I -B $T/gpu.py test|list|review|release …   # refusal checks: exit 1, no child/log
# CPU after release
./tools/cargo fmt --manifest-path $T/review/Cargo.toml [--check]     # check exit 1 before, 0 after formatting
./tools/cargo clippy --manifest-path $T/review/Cargo.toml --bin material_review --features <candidate> --locked --offline --profile ci -j2 -- -D warnings   # exit 101: 3 style lints
python3 -I -B $T/lease.py release
python3 -I -B $T/analyze.py $E $E/analysis/analysis.json
python3 -I -B $T/selfcheck.py $E $E/analysis/selfcheck.json
python3 -I -B $T/figures.py $E $E/analysis/figures
python3 -I -B $T/package.py && python3 -I -B $T/package.py --check
git diff --cached --check
```

`build.py`, `boards.py`, `author.py`, `plan.py`, `gpu.py`, `lease.py`, `analyze.py` and
`figures.py` were extended during the session:

- the default control (01:29Z);
- board S and its plans (01:30–01:31Z);
- the release process-match fix (01:32Z);
- shadow-plan analysis and crops (after release).

The compared baseline and candidate binaries, the review helper binary and the r1
fixtures and plans were all frozen before their first GPU use. The ledger records exact
argv, PIDs, times and hashes.

## Changed paths

Packet only; nothing outside `handoffs/0064-cooked-material-shaders/` changed in the
worktree.

- `handoffs/0064-cooked-material-shaders/result.md`
- `handoffs/0064-cooked-material-shaders/tools/`:
  - Python: `analyze.py`, `art.py`, `author.py`, `boards.py`, `build.py`, `common.py`,
    `figures.py`, `gpu.py`, `lease.py`, `package.py`, `plan.py`, `pngio.py`,
    `selfcheck.py`.
  - Helper crate: `review/{Cargo.toml,Cargo.lock,src/main.rs}`.
  - Adapted read-only from 0062's committed `pngio.py`, `gpu.py`, `author.py` and
    `package.py`. 0063's uncommitted helper lock approach was consulted but no 0063 file
    was copied.
- `handoffs/0064-cooked-material-shaders/evidence/`: 150 files plus `manifest.json`
  (3.4 MB).
  - The manifest lists each file's source path, raw hash and committed hash. 14 text
    logs are normalised for `git diff --check`: the 13 libtest stdout logs drop their
    final blank line, and the pre-format rustfmt check log drops one line-end space.
    Captures, JSON and the as-built helper source are byte-exact.
  - Binaries, Cargo JSON logs, cooked scenes, caches, art, private source copies,
    dry-run reports, the duplicate source-free PNG and uncommitted crops are recorded by
    hash only.

The brief is Astra's, already committed, and unchanged. Root's ACP log is outside the
worktree and was not modified. No production code, other handoff, CI, account or memory
was touched.

## Limitations and unavailable checks

- **Single backend.** One adapter (Apple M5 Pro), one OS (macOS 27.0.1), Metal only. The
  SPIR-V and HLSL material stages were not executed. No Vulkan, DX12, lavapipe, WARP,
  phone or Device Farm run; no performance, pipeline-cache, hitch or reference-phone
  claim; no phase gate.
- **Offscreen only.** All frames come from the packet helper in the `ci` profile. No
  window, presentation path, editor or shipping build was used.
- **Tests are Astra's oracles.** I executed the existing suites. Visual acceptance rests
  on my inspection, the cross-variant identity and the exact in-process controls.
- **Lengths are not read back.** The lengths-uniform values are derived from the
  allocation rules, not read from the GPU. The evidence for them is behavioral:
  bit-identical frames and exact all-light controls across every configuration listed.
- **Not run: the two `incant_headless` GPU tests.** They exercise the legacy document
  Play recorder rather than the native material renderer, spawn CLI subprocesses and
  write temporary PNG frames outside the 24-frame budget.
- **Shadow casting.** Only directional lights cast shadows in this engine, so point and
  spot shadows were not reviewed.
- **Shared traits, not regressions.** No MSAA (stair-stepped silhouettes on far pillars).
  The environment lights the scene but is not drawn as a background.
- **Retention scope.** The retained-scene reimport retention in my fixtures is at the
  cooked level: the old cooked version still renders after a newer version was prepared
  in the same renderer, and after source deletion. Retention of an already-prepared GPU
  scene is covered by the passing `model_gpu` test named above.
- **Scratch.** Scratch files (a trial art generation, a texture contact sheet, one crop
  and a rustfmt proof copy) stay in the ignored evidence directory's `scratch/`.

## Open questions

1. Should Astra pin the passthrough MSL `MTLCompileOptions` (language version and
   `preserveInvariance`) to match the WGSL path before cooked materials reach other Apple
   GPUs or OS versions? Parity held on this device.
2. Should a later packet execute the SPIR-V and HLSL material stages (lavapipe and WARP
   in CI, or hosted devices) before any default enablement?
