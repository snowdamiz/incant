# 0070 result — combined SSAO + bloom: bounded GPU execution

## Status

**The numeric analysis failed (15 of 27 gates), so device work ended there.**

- All 94 regression tests and all five numeric cases passed their own
  in-run checks.
- The appearance plan did not run, so no PNG exists and **no appearance
  judgment is possible**.
- The GPU lease is released through `runner.py release` (07:59:59Z). No owned
  or frozen-binary process survives.
- No retry was run, and nothing is latched.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort
**max**, as configured and verified by root before this prompt.

**My diagnosis, CPU-only, from retained frames:**

- **The SSAO and mixed-pane gates fail mainly because of my analyzer.** It
  never inverts the engine's tone curve, and the white-furnace fixture sits on
  that curve's shoulder.
- **The perspective furnace precondition is too strict.** The specular term
  depends on view direction.
- **A real open question remains.** Even with the tone curve inverted, the
  measured AO darkening is about 0.6× the reference blocked-ray count at every
  occluded pixel, so it cannot simply pass.

Root owns these nonvisual findings. I changed no source, helper or plan.

## Exact identities

| Item | Value |
|---|---|
| Source | HEAD `729e206af5f1f10c380794b5f66bf0beb33081ab`; root's `brief.md` uncommitted; generated helper `Cargo.lock` untracked |
| Grant | `~/.codex/incant-evidence/0070-effects-build-002/gpu-grant.json`, sha256 `b498dd0e…` |
| Build record | `build/build.json`, sha256 `96e878bf…` |
| Plan / helper digest | `27ea7b44…` / `66d18150…` |
| Toolchain | rustc 1.99.0; the renderer and host feature sets are identical in the regression and helper builds |
| Backend | **Metal** on an Apple M5 Pro, integrated GPU; adapter driver strings empty. No other backend ran |
| Frozen binaries (sha256 prefix) | `lib` `732ac9ed…`, `model_gpu` `64edd8ae…`, `gpu_host-game` `f64e5c95…`, `gpu_host-ui` `b6a33514…`, `effects_gpu` `eba0cd86…` |
| Runner invocation | `env -i PATH=/usr/bin:/bin LC_ALL=C /opt/homebrew/bin/python3 -I -B runner.py …`. Children get the runner's explicit environment: PATH, private TMPDIR, `RUST_BACKTRACE`, `LC_ALL` and `INCANT_EXPECT_GPU_BACKEND=Metal` |
| Release | `released.json` sha256 `aa8fd844…`; ledger sha256 `5bf553db…` |

## What ran

Thirteen serial foreground children, all with exit 0, their groups verified
absent, and `success: true`. The full record, with PIDs and kernel start
times, log hashes and argv, is in `evidence/execution-002.json`.

| Children | Result |
|---|---|
| `list` ×4 | lib 37, model_gpu 50, gpu_host-game 5, gpu_host-ui 2 = **94**; each exactly matches the source-derived plan |
| `suite lib` | `ok. 37 passed; 0 failed` in 3.06 s, including the four new SSAO and bloom device tests; emitter-position ran once |
| `suite model_gpu` | `ok. 50 passed; 0 failed` in 48.12 s, including `sprites::wide…` |
| `suite gpu_host-game` / `gpu_host-ui` | `ok. 5 passed` in 0.59 s / `ok. 2 passed` in 0.54 s |
| `case ssao_core` | 20 of 20 frames; 18 checks ok |
| `case ssao_materials` | 10 of 10 frames; 8 checks ok |
| `case ssao_geometry` | 17 of 17 frames; 13 checks ok |
| `case lifecycle_drs` | 18 of 18 frames; 17 checks ok |
| `case bloom_numeric` | 8 of 8 frames; 12 checks ok |

Every case also checked for wgpu validation errors (none) and for pipelines
created after setup (none).

**Budgets used:**

| Budget | Used | Limit |
|---|---|---|
| Children | 13 | 16 |
| Existing frames (static counts) | 8,092 | 8,192 |
| New frames (20 suite + 73 cases, measured) | 93 | 128 |
| PNGs | 0 | 32 |
| Retries | 0 | — |
| Wall clock, first child to release | about 3 minutes | 45 minutes |

**Not run:** `appearance`, which is gated on a passing analysis.

## Numeric analysis (`analysis.json` sha256 `3aacf18f…`, exit 1, report retained)

**Passing gates (12):**

- the four core identity gates (flat, reversed normal, far, radius 0.5);
- the three determinism gates;
- DRS SSAO nearest expansion at 12/16 and 8/16;
- bloom coverage, threshold and constant field (predicted 248 and 243 bytes).

**Failing gates (15), raw:**

- **All 13 SSAO reference gates.**
  - Every gate disagrees at exactly the pixels the reference marks occluded:
    perspective 8,558, orthographic 7,075, edge 11,025, MASK cutout 10,853,
    and 8,459 of 8,558 at strength 0.5. All unoccluded pixels agree, and no
    inference was rejected.
  - The perspective gates also fail the furnace precondition: the off frames
    take 3 distinct values (247–249) and the specular frames 56.
  - The orthographic bases are uniform, yet that gate still fails.
  - The two absence controls (`g_ao_skin_t0`, `g_ao_motion_far`) agree
    everywhere, failing only on the perspective precondition.
- **`mixed_blend_pane_scales_receiver_change`:** median ratio 2.92, expected
  0.4–0.6; 425 occluded pixels under the pane.
- **`drs_bloom_halo_centroid_on_physical_emitter`:** the expected centre is
  (186.55, 83.06).

  | Scale | Whole-viewport centroid | Distance | Tolerance |
  |---|---|---|---|
  | 16/16 | (186.16, 83.21) | 0.42 px | 1.5 px |
  | 12/16 | (190.54, 86.47) | 5.25 px | 2.0 px |
  | 8/16 | (181.23, 83.21) | 5.32 px | 3.0 px |

## Diagnosis (CPU-only, after release; changes no gate)

### 1. The analyzer omits the tone curve (my defect)

The engine's `tone_map.wgsl` is the identity below a peak of 0.8 and
`1 − 0.04/(v − 0.6)` above it. This matches the analyzer's `tone()`, which the
constant-field gate confirms at 1.0 and 1.25.

**The furnace sits on the shoulder.** The white off frame reads 247, which is
pre-tone 1.17 (diffuse about 1.13, specular 0.042). It is not the "below the
shoulder" level the fixture assumed, so AO darkening is compressed in the
bytes. `infer_k` and the mixed-pane ratio work on tone-mapped bytes, which
invalidates all 13 SSAO gates and the pane gate as written.

**Inverting the tone curve in a scratch computation** moves agreement to:

| Gate | Agreeing pixels |
|---|---|
| Orthographic | 40,993 of 43,489 |
| Perspective | 39,760 of 43,489 |
| Strength 0.5 | 42,050 of 43,489 |
| Edge | 37,969 of 43,489 |
| MASK cutout | 38,242 of 43,489 |

### 2. The furnace precondition is too strict in perspective (my defect)

The material uses `reflectance = f0·dfg.x + dfg.y`, a BRDF lookup on N·V, so
the specular term varies with each pixel's view ray. Per pixel it is still
identical for every +Z surface, so per-pixel bases remain valid; whole-frame
uniformity holds only in orthographic.

### 3. Open: measured AO is about 0.6× the reference

This is unresolved, and root's question.

**What was measured** (orthographic, tone inverted):

- The measured blocked-ray count is about 0.6× the reference count at every
  occluded pixel. Reference k 1→0, 2→1, 3→2, 4→2, 5→3, 6→3, 7→4 (median
  measured k).
- There is no false occlusion anywhere.
- No single radius (1.0–2.0) or slab-gap scale (1.0–1.5) in the reference
  reproduces it.

**What matches at code level:**

- `ao.wgsl`: 16×4 rays, the same bias and radius tests, and AO =
  `clamp(1 − strength·k/16)`;
- the direction kernel;
- raw prepass normals;
- forward use: AO multiplies only `(1−reflectance)(1−metallic)·base·irradiance`;
- the fixture camera and geometry.

**Settling it needs** direct observation of the R8 accessibility attachment,
for example a test-only readback. That attachment is not publicly
observable, as v1 noted. Until then the efficacy of real occlusion is **not
accepted**.

### 4. Open: mixed pane

The pre-tone ratio is a median of 0.313 (p10 0.279, p90 0.56) where 0.5 is
expected. This is unresolved and may be linked to item 3.

### 5. DRS bloom centroid: likely the metric

At every scale, the emitter footprint centre and the halo centroid within
24 px sit within 0.5 px of the physical emitter. The whole-viewport shift comes
from tails beyond 48 px; the centroid within 48 px is within 1.7 px. This looks
like the gate's whole-viewport metric picking up far-field tails at coarse
internal levels and viewport clipping, not a position bug. The engine's own
reduced-resolution emitter test passed. Root decides.

### 6. Furnace energy (for root)

The white diffuse output is about 1.13 pre-tone, roughly 1.6–1.7× the 0.702
environment, while the specular term is consistent with 0.702. That may be a
diffuse-irradiance normalization question; it is nonvisual.

## Appearance

**No appearance plan ran, so there are no PNGs, no matched pairs, and no
appearance judgment.**

For root's context only, I encoded the retained numeric pair `p_off_white` /
`p_ao_white` to temporary PNGs in scratch, viewed them, and deleted them.

- The AO band lies where the geometry predicts: left of the slab edge, a wider
  wedge toward the top, and thin bands along the slab's top and bottom edges.
- It is faint: at most 14 codes of darkening.
- An 8× difference shows visible stepping between discrete ray counts.

This is not an appearance review, and functional checks are not visual
success.

## Pending gates

- **SSAO efficacy:** analyzer correction plus a decision on item 3.
- **Mixed-pane gate.**
- **DRS bloom centroid gate.**
- **The whole appearance review**, with its 16 renders and 11 pairs.
- **Explicitly out of this run:** cooked SSAO and bloom, native default
  exposure, profiling subpass entries, TAA and upscaling, device budgets,
  phone performance and complete Phase 1 rendering.

The two retry slots are unused and are not approval to rerun.

## Changed paths

- `handoffs/0070-ambient-occlusion/result.md`
- `handoffs/0070-ambient-occlusion/evidence/execution-002.json` (identities,
  per-child facts, gate lists and the diagnostic numbers)

Both are committed after release with `Built-by: claude`. No source, helper,
plan or evidence-directory file was changed; the evidence directory remains
root's. Nothing was pushed.
