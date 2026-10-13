# 0070 SSAO filter repair: preparation for a bounded filtered comparison

## Status

**Preparation only.** There was no Cargo, target use, GPU, capture, rendering,
production edit or account access. No build or GPU authority follows from this
document.

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort
  **max**. ACP verification is root's.
- **Unchanged:**
  - `result.md` (build002), `corrected-gpu-result.md`,
    `corrected-bloom-preparation.md`, `ssao-quality-repair.md` and
    `ssao-quality-repair-addendum.md`;
  - every grant, failure and receipt;
  - everything under `tools/`, still pinned by build004. The new helpers
    import `tools/analyze.py` and `tools/ssao_reference.py` read-only.
- **New:**
  - this document;
  - `ssao-repair/filter/`: `oracle.py`, `analyze_filter.py`, `controls.py`,
    `oracle_summary.py`, `cases.json` and two evidence files.
- **Root's source, read but not pinned:**
  `/Users/andreyyurlov/.codex/worktrees/local-service-stack/incant`, branch
  `codex/ssao-filter-repair`. Its HEAD is 72a16a9d, with uncommitted changes:
  a new `filter.wgsl`, and edits to `ao.wgsl`, `draw.rs`, `math.rs`,
  `pipelines.rs`, `targets.rs`, `tests.rs` and `quality_tests.rs`. I read the
  work in progress only to match names and pass order; nothing here depends on
  its bytes. What I observed matches R1/R3:
  - rotation `B[y%4][x%4]` and jitter `(33 + 2·B[x%4][y%4])/64` in
    full-target coordinates;
  - an explicit centre weight of 1, with taps skipped outside the viewport or
    on cleared depth, and a plane distance taken from the centre's authored
    normal;
  - the passes AO → `T0`, H → scratch R8, V → `T0`, each cleared to white
    inside the fractional viewport;
  - a 736 B uniform and 14 B/px.

## Root decisions applied

- **N6T** is an additional diagnostic. The original N6 failure stays recorded,
  and N6 stays a gate.
- **N7's exact definition and N8's stripes ROI** are adopted.
- **N8 perspective** still fails. No quality gate is waived.
- **R1/R3, including the jitter,** is a provisional opt-in numerical candidate
  for rendered evaluation. It is not a shipping or default quality acceptance.
- **No** further sampling-cost study and no art-radius change.
- **Memory:**
  - four-megapixel admission;
  - an explicit 56 MiB idle logical payload, and 112 MiB across a transition
    between two cap-sized extents; neither is aggregate memory;
  - a 736 B uniform, unchanged authored normals, two separable R8 passes, and
    no cooked admission.

## 1. Evidence model: filtered continuous accessibility intervals

`oracle.py` layers the candidate on the unchanged scalar probe oracle. Each
stage's uncertainty is accounted for separately; none is folded into a
blanket slack.

| Stage (write) | Decisions and uncertainty | Accounting |
|---|---|---|
| AO pass → `T0` (R8) | Probe outcomes, using `ssao_reference._step` with its unchanged three-valued tolerances (position 2e-3, pixel 4e-3); coverage candidates at each texel centre | Exact integer `[k_lo, k_hi]` per candidate. The texel's code interval is the hull over candidates; cleared depth is 255 |
| R8 write (each of the three) | Round to nearest, either tie direction | Value interval → codes `[ceil(255·lo − 0.5 − ε), floor(255·hi + 0.5 + ε)]`, with ε = 2e-4 codes for the f32 mean and the R8 load. Truncation is not admitted |
| H pass `T0` → scratch | Tap inclusion: the viewport test is exact (centres and rect are f32-exact); cleared depth is excluded; ambiguous coverage takes the hull of its alternatives. Plane weight `c·clamp(1 − dist/τ)` with `dist ± 1e-4` view units for f32 reconstruction | Each tap carries a value interval and a weight interval. The centre's weight is exactly 1. The mean's extremes are found by enumerating the vertices of the weight box, which is exact for a linear-fractional mean |
| V pass scratch → `T0` | The same, on the H codes | The same |
| Forward and output | `T0` is loaded exactly; output inference is unchanged (±0.75 sRGB code, inverse gray tone, 0.02 low-signal floor) plus the f16 radiance term 0.002 | `tools/analyze`'s `AO_SLACK` is `0.5/255 + 0.002`. Its 0.5/255 R8 term is now the explicit V-stage rounding and is not added twice |

**Preconditions are kept as before:**

- every viewport-interior pixel is compared (43,489);
- no invalid inference;
- at least 1,000 *certainly* occluded pixels (final upper code ≤ 254), all of
  them compared;
- absence controls are exactly 255;
- the basis contract is unchanged.

**Slack cannot inflate silently.** Wherever no probe or coverage decision in a
pixel's 5×5 support is ambiguous, the derivation bounds the final width at 3
codes (1 at the raw stage, 2 after H, 3 after V). A wider oracle interval
fails the gate.

**Nothing is dropped.** An ambiguous texel keeps its hull and is counted by
kind.

**Measured oracle decisions** (`oracle-summary-001.json`, 64 s):

| Fixture | Ambiguous-probe texels | Raw ties | H texels spanning ≥ 2 codes | Final width 0 / 1 / 2 / 3 | Certainly occluded |
|---|---|---|---|---|---|
| `white_slab` p (also `g_ao_white`, `mask_equal`, `backface`, `mirrored`, `skin_t1`, `motion_near`) | 192 | 415 | 828 | 37,850 / 4,464 / 1,026 / 149 | 11,013 |
| `white_slab` o | 94 | 145 | 408 | 39,020 / 4,027 / 389 / 53 | 9,605 |
| strength 0.5 p | 192 | 0 | 695 | 39,678 / 3,732 / 79 / 0 | 10,861 |
| `edge_slab` p | 256 | 437 | 1,104 | 35,925 / 5,720 / 1,570 / 274 | 12,726 |
| `mask_cutout` p | 263 | 379 | 1,107 | 36,549 / 5,175 / 1,530 / 235 | 12,613 |
| `stripes` p | 153 | 398 | 753 | 34,779 / 7,924 / 784 / 2 | 17,959 |
| `stripes` o | 51 | 124 | 221 | 40,395 / 2,821 / 259 / 14 | 6,555 |
| `far_slab`, `skin_t0` (absence) | 0 | 0 | 0 | 43,489 / 0 / 0 / 0 | 0 (all exactly 255) |

No fixture has an ambiguous coverage candidate, an uncertain tap inclusion or
a partial plane weight.

**Independent cross-check.** On `white_slab` (both projections), `edge_slab`
and `stripes` (both projections), the f64 filter model from the addendum
(`quality_model.py`, written separately) falls inside the oracle at every
stage and every pixel.

## 2. Proposed staged R8 readback (root, before source freeze)

**Why.** On the white receiver, the tone-inverted accessibility interval is
6.75–10.82 codes wide (median 10.82), while the oracle's final intervals are
0–3 codes.

- **Output-only evidence is too coarse:** it can see only errors larger than
  roughly 5 codes, and it cannot measure N6 (a 4-code bound), N8 (8 codes) or
  N7 at all.
- **Stage planes check each GPU stage to the code** on its own read-back
  inputs. The controls show a single +1 raw code is caught. The raw stage
  also checks the device's R8 rounding rule directly: truncation would fail
  at `k = 9` (0.4375 → 112 versus 111).

**Proposed API.** It is test and diagnostic only, behind a non-default feature
`ssao-stage-readback`, and adds no mutation path:

```rust
#[cfg(feature = "ssao-stage-readback")]
pub struct AmbientOcclusionStages { pub size: [u32; 2], pub raw: Vec<u8>, pub horizontal: Vec<u8>, pub vertical: Vec<u8> }

#[cfg(feature = "ssao-stage-readback")]
impl Renderer {
    /// One-shot: the next encoded SSAO execution (draw_scene or RetainedScene::submit)
    /// copies T0 after the AO pass, the scratch target after H, and T0 after V.
    pub fn arm_ambient_occlusion_stage_capture(&self) -> Result<()>;
    /// After that frame's submission: map, strip 256-byte row padding, return tight planes.
    /// Errors when nothing was captured or the capture is still armed.
    pub fn take_ambient_occlusion_stages(&self) -> Result<AmbientOcclusionStages>;
}
```

**Implementation notes:**

- In `Prepared::encode`'s pass loop, add one `copy_texture_to_buffer` after
  each pass ends, into one buffer of three planes of `align(width, 256) ×
  height` bytes.
- `targets.rs` adds `COPY_SRC` only under the feature.
- Copies are not frames under the plan's frame rule.
- The shipped and default feature sets never enable it, and matched
  baseline/candidate comparisons must not either.

**Helper side** (`tools/effects`, applied after build004 releases):

- enable the feature for the helper's `incant_render` dependency;
- add `Run::draw_with_stages` and `Run::submit_with_stages`, each of which
  arms, draws or submits, takes the planes, then calls `Store::keep_stages`;
- `Store::keep_stages` writes `<label>.raw.r8`, `<label>.h.r8` and
  `<label>.v.r8` with create-new, and an index entry per plane:
  `{"label": "<label>.<stage>", "file": ..., "size": [w, h], "r8_sha256": ...,
  "meta": {"frame": "<label>", "stage": "raw|h|v"}}`;
- `tools/analyze.load_output` already ignores non-`.rgba` entries.

The 15 stage frames are the AO-on frames of the 15 filtered reference gates,
listed in `cases.json`.

**If root declines:**

- the 15 filtered end-to-end gates still run;
- the 15 stage gates and all 6 quality gates fail as missing evidence and are
  not passed;
- the quality metrics would then be unmeasured on the GPU.

## 3. Cases, frames and budgets (exact; `cases.json`)

| Case | Frames | Stage frames | Helper |
|---|---|---|---|
| `ssao_core` | 20 | `p_ao_white`, `o_ao_white`, `p_ao_strength_0_5`, `p_ao_edge` | Existing, unchanged |
| `ssao_materials` | 10 | — | Existing, unchanged |
| `ssao_geometry` | 17 | `g_ao_white`, `g_ao_mask_equal`, `g_ao_mask_cutout`, `g_ao_backface`, `g_ao_mirrored`, `g_ao_skin_t0`, `g_ao_skin_t1`, `g_ao_motion_far`, `g_ao_motion_near` | Existing, unchanged |
| `lifecycle_drs` | 18 | — | Existing, unchanged |
| `ssao_quality` | 10 | `p_ao_stripes`, `o_ao_stripes` | New (section 4) |
| Appearance | 16 renders, 16 PNGs, 11 pairs | — | The existing set, unchanged |

**Totals:** 91 new frames against the 128 cap (37 unallocated), 11 pairs
against 16, and 16 PNGs against 32, all at most 960×540.

**Not re-run:** `bloom_numeric`. The filter does not touch bloom code, and its
gates are not declared here.

**Root's choice:** a lib suite listing and run under the existing-regression
budget, since the renderer binary changes. That makes 8 GPU children in all:
list, suite, 5 cases and appearance. There are no retries without a new
authorization.

## 4. Minimal additions and patches (proposals, not applied)

### a. Plan

In a filter-run `make_plan.py` revision applied after build004 releases:

- **Numeric cases:** add `ssao_quality` with 10 frames and renderers `off` and
  `ao`, and drop `bloom_numeric` for this run.
- **Analyzer:** point the analysis step at
  `ssao-repair/filter/analyze_filter.py`.
- **Budgets:** as in section 3.

The runner and lifecycle code are reused unchanged; the runner looks a case up
generically through `numeric_case`.

### b. Helper

Add `"ssao_quality" => ssao_quality(out)` to `run_case`, plus the
`draw_with_stages` call sites listed above. The new case, uncompiled until
root freezes the source and compiles it:

```rust
pub const SPHERE_CENTRE: [f64; 3] = [0., 0., -3.];
pub const SPHERE_SCALE: f64 = 1.2; // gltf::sphere(24, 48) radius 0.5 -> 0.6
fn stripe(i: u32) -> Place { ([-0.45 + 0.3 * f64::from(i), 0.5, -3.], [0.1, 2., 1.]) }
fn gray() -> Value { gltf::pbr([0.5, 0.5, 0.5, 1.], 0., 0.5) }

fn ssao_quality(out: &Path) -> Result<bool> {
    let off = Device::new("off", Gpu::new()?);
    let ao = Device::new("ao", Gpu::new()?.with_ao(AO)?);
    let mut run = Run::new("ssao_quality", 10, out)?;
    let (body, errors) = scoped!([off, ao], quality_body(&mut run, &off, &ao));
    finish(run, &[&off, &ao], errors, body)
}

fn quality_body(run: &mut Run, off: &Device, ao: &Device) -> Result<()> {
    let assets = Assets::new()?;
    let env = assets.environment()?;
    let quad = gltf::unit_quad(1., 1., false);
    let white_q = assets.model("quad_white", &quad, white(), None, None)?;
    let black_q = assets.model("quad_black", &quad, black(), None, None)?;
    let sphere = assets.model("sphere_24x48", &gltf::sphere(24, 48), gray(), None, None)?;
    let s = shot();
    for orthographic in [false, true] {
        let (tag, proj) = if orthographic { ("o", "orthographic") } else { ("p", "perspective") };
        let world = DMat4::from_translation(DVec3::from_array(SPHERE_CENTRE)) * DMat4::from_scale(DVec3::splat(SPHERE_SCALE));
        let lone = input(vec![camera(orthographic), key_light(0.), mesh("sphere", &sphere, world)], &env, 1.);
        let parts = json!([{"id": "sphere", "mesh": "gltf::sphere(24, 48)", "material": "gray",
            "center": SPHERE_CENTRE, "scale": [SPHERE_SCALE; 3]}]);
        let off_sphere = run.draw(off, &format!("{tag}_off_sphere"), &lone, s, ibl_meta("isolated_sphere", proj, parts.clone(), 1.))?;
        let ao_sphere = run.draw(ao, &format!("{tag}_ao_sphere"), &lone, s, ibl_meta("isolated_sphere", proj, parts, 1.))?;
        check_pair(run, &format!("{proj}: isolated 24x48 sphere unchanged by AO"), &ao_sphere, &off_sphere);
        let stripes = |q: &Arc<RuntimeAsset>, m: &str| {
            let mut nodes = vec![camera(orthographic), key_light(0.), mesh("receiver", q, place(RECEIVER))];
            let mut described = vec![part("receiver", m, RECEIVER)];
            for i in 0..4 {
                nodes.push(mesh(&format!("stripe{i}"), q, place(stripe(i))));
                described.push(part(&format!("stripe{i}"), m, stripe(i)));
            }
            (input(nodes, &env, 1.), json!(described))
        };
        let (white_scene, wp) = stripes(&white_q, "white");
        let (black_scene, bp) = stripes(&black_q, "black");
        let off_white = run.draw(off, &format!("{tag}_off_stripes"), &white_scene, s, ibl_meta("stripes", proj, wp.clone(), 1.))?;
        let ao_white = run.draw_with_stages(ao, &format!("{tag}_ao_stripes"), &white_scene, s, ibl_meta("stripes", proj, wp, 1.))?;
        run.draw(off, &format!("{tag}_off_stripes_black"), &black_scene, s, ibl_meta("stripes_black", proj, bp, 1.))?;
        check_occludes(run, &format!("{proj}: stripes darken the receiver only"), &ao_white, &off_white, 200);
    }
    Ok(())
}
```

### c. Analyzer

This is the new namespace, already written and exercised by the controls.
`tools/analyze.py` stays the build002/003/004 analyzer.

## 5. Sphere and stripes fixtures (exact)

**Isolated sphere:**

- **Fixture:** `gltf::sphere(24, 48)` (2,208 triangles, radius 0.5) under
  scale 1.2, which gives radius 0.6. It sits at view `(0, 0, −3)` in gray
  0.5, with no receiver, IBL only (zero key light), in both projections.
- **Gate:** AO-on is byte-identical to AO-off over the whole frame.
- **Preconditions:**
  - the largest facet rise is 0.0012819, below the 0.01 bias. This comes from
    `sphere_rise_bound.py`, is camera-independent, and is recomputed by the
    gate;
  - the sphere covers its analytic projected area within 10%: 2,824.6 px in
    perspective, 455.2 px orthographic.
- **Non-vacuity:** the same child's stripes gates prove AO is active there.
- **Why gray:** a white dielectric under the uniform furnace renders at the
  environment's radiance, so it could vanish against an environment
  background.

**Stripes:**

- **Geometry:** the receiver, plus quads at centres
  `(−0.45 + 0.3i, 0.5, −3)` scaled `(0.1, 2, 1)`. That is x in
  `[−0.5 + 0.3i, −0.4 + 0.3i)`, y in `[−0.5, 1.5)`.
- **Frames:** white on and off, and the black basis, in each projection.
- **Gates:**
  - the filtered reference gate, end to end and staged;
  - N8 at the adopted ROI: perspective row 124, x 148–158; orthographic
    row 110, x 150–156.
- **Convention:** framebuffer texels, origin at the target's top left, centres
  at `+0.5`; view space is right-handed with `−Z` forward.

## 6. Gates and expected outcomes (candid)

### Numeric gates (43)

- 15 filtered reference gates: the 13 originals plus stripes in both
  projections;
- 15 stage gates;
- 9 identity gates: the 7 originals plus the sphere in both projections;
- the mixed BLEND pane gate;
- 2 DRS nearest-expansion gates and the DRS bloom-centroid gate.

All are reused or adapted with unchanged semantics.

### Quality gates (6), computed on the read-back final codes

The verdicts below are interval-sound predictions (`oracle_summary.py`):
every GPU result that agrees with the oracle must reach them.

| Gate | Prediction | Value |
|---|---|---|
| N6, authored (`white_slab` perspective row 97) | **Fails** | 7–8 codes against 3.98 |
| N7, `white_slab` perspective | Passes | worst case 0 against bound 138 |
| N7, `white_slab` orthographic | Passes | worst case 8 against bound 176 |
| N7, `edge_slab` perspective | Passes | worst case 0 against bound 202 |
| N8, perspective | **Fails** | 0.376–0.439 against 0.25 |
| N8, orthographic | Passes | 0.176 |

**N6T** is a diagnostic only. On the independent model it measures 3.37/255
against 1/64.

The planned run's overall analysis will therefore fail on N6 and N8
perspective, unless the GPU disagrees with the oracle; the numeric gates would
then fail instead. No gate is waived.

### Still pending

- **Numeric:** N4 agreement on the device, N5 sphere identity on the device,
  DRS, and determinism and exclusions on the filter build.
- **Appearance:** my rendered-pair review against the build003 crops (A1–A6):
  - A5, the art radius, is deliberately untouched;
  - N8 perspective predicts a soft comb remaining near thin occluders (v2
    MASK stripes).
- **Bloom:** the build004 rerun, which is separate.
- **Not started:** device budgets and phone measurements, cooked admission
  and default enablement. None is authorized.

## 7. Focused synthetic controls (run)

`analyze_filter.py --selftest` exits 0 in 74 s; its output is in
`selftest-001.json`.

**Passes as intended:**

- the independent model's planes pass the filtered and stage gates on 5
  fixtures, with 6,555–17,959 certainly occluded pixels each;
- the quality gates reproduce the addendum: N6 7.00/255 fails, N7 is 0/0/0,
  N8 is 0.376 (fails) and 0.176 (passes), and N6T is 3.37/255.

**Fails as intended:**

- **Perturbed codes:** a raw code +1 (`raw_outside 1`), and a final code below
  its intervals (`v_outside 1`, `v_outside_final 1`).
- **Ambiguous evidence:**
  - conflicting RGB (empty inference);
  - no diffuse signal (coverage);
  - every ray ambiguous (no certain occlusion);
  - inflated decisive widths;
  - an absence mismatch.
- **Missing evidence:** a stage plane, all planes (every quality gate), a frame.
- **Empty evidence:** an empty evidence tree (every case missing, every gate
  failing), an empty index and a zero-length plane (cases invalid).
- **CLI:** it writes the failing report (exit 1) and never replaces it
  (exit 2).

The scratch evidence trees are removed. No child process remains.

## 8. Return record

**Commands,** each run from the worktree root under
`env -i PATH=/usr/bin:/bin LC_ALL=C /opt/homebrew/bin/python3 -I -B`
(Python 3.14.8):

- `handoffs/0070-ambient-occlusion/ssao-repair/filter/analyze_filter.py --selftest`
  → exit 0;
- `handoffs/0070-ambient-occlusion/ssao-repair/filter/oracle_summary.py`
  → exit 0;
- after a future granted run: `analyze_filter.py <evidence-dir>`, which
  writes `filter-analysis.json` once, exit 0/1/2.

**File sha256** in `ssao-repair/filter/`:

| File | sha256 |
|---|---|
| `oracle.py` | `bdc81363…` |
| `analyze_filter.py` | `5b06c7cb…` |
| `controls.py` | `669d245d…` |
| `oracle_summary.py` | `6ecf0bda…` |
| `cases.json` | `fa6d1076…` |
| `selftest-001.json` | `1b2871fb…` |
| `oracle-summary-001.json` | `3220f1e4…`; it records its `seconds` |

**Screenshots:** none. Nothing was rendered or captured.

**Limitations:**

- an f64 CPU oracle with documented tolerances, covering planar fixtures plus
  the sphere identity;
- the Rust additions are uncompiled proposals;
- the stage readback needs root's renderer feature before source freeze;
- the evidence format and the gates are proposals until root freezes the plan.

**Open questions for root:**

1. Add the stage readback feature before freeze?
2. Accept the plan in `cases.json`, with `bloom_numeric` not re-run?
3. Confirm R8 write rounding on the target backend. The raw stage will test
   it in any case.
