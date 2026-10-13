# 0070 SSAO quality repair: design for Astra's numerical implementation

## Status

**Design only.** There is no production code change and no new capture.

- **Who implements:** root and Astra own the numerical implementation. Claude
  reviews the repaired renders.
- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort
  **max**, as verified by root.
- **Source:** my retained build003 appearance verdict
  (`corrected-gpu-result.md`, `evidence/appearance-review/`) and the current
  algorithm:
  - `ambient_occlusion/ao.wgsl`;
  - `prepass.wgsl`;
  - `math.rs` (directions, `Uniform`, `MAX_PIXELS`, `BYTES_PER_PIXEL`);
  - `pipelines.rs` (the diffuse-only substitution);
  - `targets.rs` (the R8 accessibility).

  Beyond these I made targeted reads only:
  - the composed sphere's mesh and placement (`gltf.rs`, `composed.rs`);
  - the reversed-normal fixture (`numeric.rs`, `ssao_reference.py`);
  - the `frame.eye` uniform in `model_material.wgsl`.

  There was no broader survey.
- **New design aid:** `ssao-repair/sphere_rise_bound.py`, stdlib only, CPU,
  about 1 s. It sits outside `tools/`, so the helper digest pinned for build004
  is unchanged.

The numerics are not in question: build003 matched the independent reference
at every pixel. This repair targets **appearance**, so it changes the
documented sampling contract, and the CPU oracle must change with it.

**This corrects my build003 report.** `corrected-gpu-result.md` blamed the
sphere's mottling on the 0.01 bias being smaller than the faceted mesh's depth
error. The bound under A1 rules that out. The report stays unchanged as the
record of that run.

## Failures being repaired (build003, at radius 2.0, bias 0.01, strength 1.0)

| # | Artifact (where) | Cause in the current algorithm |
|---|---|---|
| A1 | Mottled, facet-patterned darkening on the sphere's lower hemisphere (v2, v4) | **Not self-occlusion**; see the bound below. The darkening comes from the floor and the crate, legitimately within radius 2. The mottling is A2's mechanism on a curved receiver: `ao.wgsl` tests each probe against the surface at the center of the pixel that contains it (`vec2i(probe)`). On the grazing floor that depth changes steeply across a pixel, so probes near the floor count or not according to their sub-pixel position. The fixed kernel turns those flips into hard patches whose phase changes at facet edges, because reconstructed positions are piecewise planar. This cause is inferred from the shader and the scene geometry, not measured per pixel |
| A2 | Hard-edged polygonal wedges on the floor and the crate side face (v4) | All 16 directions are identical for every pixel, with no rotation or filter, so each ray's binary result flips along straight screen-space boundaries |
| A3 | Horizontal banding on vertical faces near the floor (v2 pillar, crate front) | Discrete k/16 levels, plus 4 fixed step distances (0.25, 0.5, 0.75 and 1.0 × radius) |
| A4 | Comb pattern on the floor under the MASK stripes (v2) | Thin occluders sampled by an identical kernel at every pixel |
| A5 | Over-broad, heavy darkening (72% of v4) | The art radius 2.0 against props of about 1 m. **This is an art setting, evaluated separately**; see the art tuning section |
| A6 | 2×2 blocks at 50% DRS | Nearest expansion of internal-resolution AO, the accepted canonical treatment. The filter smooths internal AO; expansion stays nearest |

**The A1 bound.** The composed sphere is `gltf::sphere(24, 48)` (2,208
triangles) under entity scale 1.2, so its radius is 0.6. A surface point S
counts as an occluder only if `dot(S - p, n) > bias`.

`sphere_rise_bound.py` maximizes that rise over every mesh point S, with p
sampled at 16, 32 and 64 barycentric steps per triangle and the shading
normal interpolated and normalized as the prepass does.

- For each p, the maximum over S is attained at a vertex, because the rise is
  linear in S over the convex hull.
- One sector column covers the whole mesh, because the mesh is invariant under
  7.5° rotation.

Results, identical at all three densities:

- **Largest rise: 0.0012819**, about an eighth of the 0.01 bias. The plane test
  therefore rejects the sphere as its own occluder.
- **Largest shading-to-face normal angle: 5.30°.** Every ray leaves the
  shading tangent plane at no less than 10.18° (`u_15 = 15.5/16`), so every
  probe also starts above its own facet.

## Repair: algorithm changes evaluated at the original settings

### R1. Per-pixel interleaved kernel rotation and step jitter

Let `(x, y)` be the internal-target integer pixel (`vec2i(pixel.xy)`) and
`B` the 4×4 Bayer matrix, a shader constant:

```
B = [[ 0,  8,  2, 10],
     [12,  4, 14,  6],
     [ 3, 11,  1,  9],
     [15,  7, 13,  5]]
b_rot  = B[y mod 4][x mod 4]          // 0..15
b_step = B[x mod 4][y mod 4]          // transposed: decorrelated from b_rot
```

**Directions.** The `math.rs` direction table is unchanged: `u_i = (i+0.5)/16`,
and `phi_i` is 2π times the 32-bit radical inverse of `i`, so the 16 azimuths
are the multiples of 2π/16. Each pixel rotates its tangent frame, and with it
the whole set, by an azimuth inside one stratum:

```
alpha      = (2*pi/16) * (b_rot + 0.5) / 16
(c, s)     = ao.rotations[b_rot].xy             // f32(cos alpha), f32(sin alpha), from f64 on the CPU
tangent'   = tangent*c + bitangent*s
bitangent' = bitangent*c - tangent*s
direction  = tangent'*k.x + bitangent'*k.y + n*k.z   // k = ao.directions[ray], as today
```

This equals replacing `phi_i` with `phi_i + alpha`. A 4×4 tile therefore
holds 256 distinct azimuths spaced 2π/256 apart.

`ao.rotations` is a new `array<vec4f, 16>` uniform field holding
`(c, s, 0, 0)`. A `vec2f` array is invalid here, because uniform arrays need a
16-byte stride. CPU and GPU then use identical f32 inputs, with no per-pixel
trigonometry. `Uniform` grows from 480 B to 736 B, and its size and offset
assertions change with it.

**Steps.** Each step is jittered within the upper half of its stratum:

```
t = 0.5 + 0.5 * (f32(b_step) + 0.5) / 16                        // (33 + 2*b_step)/64, exact in f32
q = p + direction * (ao.values.x * (f32(step) - 1. + t) / 4.)   // step = 1..4
```

`t = 1` reproduces today's `f32(step)/4` steps exactly. The smallest step is
0.129 × radius, so there are no near-zero probes; the largest reaches
0.996 × radius.

There is no temporal component: the pattern depends only on the pixel
coordinate, so frames stay deterministic.

### R2. Biases: unchanged formulas and values

Both tests keep today's form and the original 0.01:

```
blocked if  surface.z >= q.z + bias                          // the visible surface is in front of the probe
       and  dot(surface - p, n) > bias                       // and above the receiver's tangent plane
       and  dot(surface - p, surface - p) <= radius*radius
```

- **There is no self-occlusion to bias against.** Planar receivers rise 0
  above their own plane, and the composed sphere rises at most 0.0012819.
- **A larger or footprint-scaled bias would not remove A1.** The
  containing-pixel snap decides a flip only inside a band about half a pixel's
  depth step wide around the threshold. Raising the bias moves that band but
  does not narrow it. R1 and R3 average the flips instead.

**Geometric AO normals are not adopted.** Storing the derivative face normal
in the prepass would matter only for coarse smooth-shaded meshes, where
shading and face normals differ by more than the 10.18° ray elevation. It
would also break an existing contract gate: `ssao_core_p_ao_reversed_identity`
expects a camera-facing quad whose authored normal points away to stay
unoccluded, and a viewer-facing geometric normal would occlude it.

If root later wants coarse-mesh robustness, a sign-preserving form keeps that
gate:

```
g  = cross(dpdy(world), dpdx(world))
ng = normalize(g) * sign(dot(g, n_shading))
```

It would need its own coarse-mesh fixture and is not part of this repair.

### R3. Edge-preserving separable filter over exactly one rotation tile

Add two full-screen passes after the AO pass, inside the same viewport rect.

1. The AO pass writes raw accessibility to R8 target `T0`, the existing
   `accessibility`.
2. **H pass:** `T0` → `T1`, a new R8 target of the same size.
3. **V pass:** `T1` → `T0`.

The forward substitution in `pipelines.rs` keeps loading `T0` at
`vec2i(in.position.xy)`; that is unchanged.

**Window.** Five taps at offsets −2..+2 along the pass axis, with weights
`c = [0.5, 1, 1, 1, 0.5]`. Every residue mod 4 then gets total weight exactly
1, the window is symmetric, and there is no half-pixel shift. H×V covers
exactly one 4×4 rotation tile, so a fully flat neighbourhood averages all 256
rays.

**Edge weight** (center p, tap q). Positions are view-space, reconstructed
from the depth attachment with the existing `position()` and `ao.inverse`. The
normal is the center's stored prepass normal, the unchanged shading normal,
moved into view space exactly as `ao.wgsl` does:
`n_p = normalize((ao.view * vec4f(stored, 0)).xyz)`.

```
dist   = abs(dot(P_q - P_p, n_p))                 // distance from p's tangent plane
tau    = max(bias, 0.01 * (-P_p.z))               // 1% of view distance, at least the AO bias
w_q    = c_q * clamp(1 - dist / tau, 0, 1)        // 0 for background, outside-viewport, or a different surface
A_out  = (sum w_q * A_q) / (sum w_q)              // center weight c_0 = 1 and dist = 0, so the sum is > 0
```

**Copied unchanged:** a center that is background (depth ≥ 1), outside the
viewport, or has a zero stored normal. In every such case the AO pass already
wrote 1. The reversed-normal receiver is unaffected, because `abs` ignores
the normal's sign.

Use the plane distance rather than the raw depth difference: grazing floors
have large per-pixel depth steps but zero plane distance, which is exactly
where the wedges of A2 live.

On curved receivers the shading normal is adequate. On the composed sphere,
with pixel footprint `f`, a neighbour 2 px away sits off the tangent plane by
at most:

- `2·f·sin 5.3°` from the normal tilt;
- plus the sag `(2f)²/1.2`.

At a 90° field of view over 480 rows, that sum stays under 0.2% of the view
distance up to 17 units, against `tau` = 1%. The composed views place the
sphere's surface 1.4–7.2 units away.

Quantization: R8 between passes, with an error of at most 0.5/255 per pass.
The oracle must apply the same rounding.

## Measurable controls

**Numeric invariants.** These run on the unchanged fixtures at radius 2.0,
bias 0.01 and strength 1.0, and use the existing analyzer machinery with an
updated oracle.

| ID | Control | Pass criterion | Catches |
|---|---|---|---|
| N1 | Flat receiver, perspective and orthographic | AO identical to off, byte for byte (existing) | Rotation or jitter creating self-occlusion on planes |
| N2 | Reversed normal, far slab, radius 0.5, black-material and strength 0 | Identical to off (existing) | Frame, sign or radius regressions |
| N3 | Silhouette leak: white-slab slab pixels with k_ref = 0 within 3 px of its silhouette | 0 pixels darker than off | A filter bleeding receiver occlusion across depth edges |
| N4 | Oracle agreement: every interior pixel, all 13 reference gates | Full coverage and 0 disagreements (existing analyzer gates) | Any mismatch between the GPU and the updated oracle |
| N5 | Curved convex identity: the composed scene's 24×48 sphere at radius 0.6, alone with no floor, in perspective and orthographic | Every sphere pixel byte-identical to AO off. Precondition: the A1 bound, recomputed by the analyzer for this mesh at the configured bias, is below the bias; otherwise the analyzer refuses N5 rather than passing it | Frame, plane-test or filter regressions on curved, smooth-shaded receivers. It also confirms on the device that the sphere does not shadow itself |
| N9 | DRS 75% and 50% | The 0067 nearest-expansion rule still exact against internal-size references (existing) | The filter running in the wrong space |
| N10 | Determinism and exclusions | Repeated frames byte-identical; the existing direct, emissive, specular, BLEND and pane gates unchanged | Temporal leakage; receiver-set drift |

**Quality metrics, measured exactly in the CPU oracle domain.** Each is
carried to the device by N4, since the GPU must agree with the oracle per
pixel. "Silhouette band" means pixels within 2 px of a depth discontinuity in
the analytic geometry; true occlusion jumps there must survive.

| ID | Metric (fixture) | Current algorithm (exact baseline) | Repaired requirement | Catches |
|---|---|---|---|---|
| N6 | Accessibility along the receiver row y = 97, x 100–171 (`white_slab`, perspective) | 8 distinct levels, steps of 1/16 and 2/16 | Largest adjacent step ≤ 1/64 outside the silhouette band | Banding (A3) |
| N7 | Interior pixels outside the silhouette band with a horizontal or vertical neighbour step ≥ 1/16 | `white_slab` persp 1,397; ortho 1,782; `edge_slab` 2,046 | ≤ 10% of each baseline (≤ 139 / 178 / 204) | Hard-edged wedges (A2) and the same flips behind A1 |
| N8 | Peak-to-peak accessibility across one stripe period, along a receiver row behind a new `stripes` fixture (four 0.1-unit stripes at z = −3, pitch 0.3, in front of the z = −4 receiver) | Measured on the current oracle when the fixture is added | ≤ 25% of the current oracle's value | The comb (A4) |

**Indicative output-byte baselines only.** These come from the composed
build003 renders. The tone shoulder and 8-bit quantization flatten them, which
is why they are not gates:

- v2 pillar: largest adjacent darkening step 2.33 codes, over a 0–7.3-code
  range;
- v4 floor: 2 pixels with a darkening gradient above 2 codes;
- v2 MASK strip: row standard deviation 0.56 codes.

Use them only to confirm direction in the appearance review, together with the
v2 and v4 sphere crops for A1.

**Discriminating mutations.** These run on the CPU oracle, like the runner's
`mutations.py`. Each row names the control expected to fail.

- **Before any GPU run:** Astra runs every row and reports the outcome.
- **If removing a component fails no control:** report it to root, who
  decides whether to keep the component or add a control.
- **No control is weakened** to make a mutation fail.

| Mutation | Control expected to fail |
|---|---|
| Filter off (R1 output straight to forward) | N7: per-pixel dither leaves steps ≥ 1/16 throughout partial occlusion |
| Rotation off (`alpha = 0`) | N8: every pixel probes the stripes at the same azimuths again |
| Jitter off (`t = 1`) | N6: the four fixed step distances return as bands |
| Depth weight off (`w_q = c_q`) | N3 |
| Asymmetric window (−2..+1) | N4: a half-pixel shift disagrees along occlusion gradients |
| Plane bias 0.01 → 0.001 | N5's precondition: the 0.0012819 rise now exceeds the bias, so the analyzer must refuse N5 |

**The oracle update** is required before any GPU run. `ssao_reference.py` and
the `math.rs` CPU test must implement:

- R1's frame rotation, `alpha` table and `t`;
- R3's weights from analytic positions and plane normals;
- R8 rounding after the AO, H and V passes;
- the `stripes` fixture for N8;
- N5's precondition, from the `sphere_rise_bound.py` computation at the
  configured bias.

The analyzer's interval inference then compares filtered accessibility. Its
±0.75-code and R8 slack must be re-derived for the two extra rounding steps.

## Resource and per-pixel cost

This is a static analysis per internal pixel, worst case, and **not a device
measurement**. Operation counts treat a fused multiply-add as one operation.

| Stage | Today | Repaired | Delta |
|---|---|---|---|
| Prepass | Depth 4 B + normal 8 B written | Unchanged | 0 |
| AO pass | Center depth 4 B + normal 8 B; ≤ 64 probes, each a 4 B depth load and about 70 operations (projection and inverse mat4×vec4); 1 B written | Plus one rotation-table load, Bayer indices and `t` (about 10 operations), the frame rotation (12) and one add per probe (64) | About +90 operations (+2%), +0 B |
| H filter | — | Center depth 4 B + normal 8 B + R8 1 B, and 4 taps × (depth 4 B + R8 1 B): 33 B read, 1 B written. About 230 operations: 5 `position()` reconstructions (about 27 each), 4 plane distances and weights, and the center normal's view transform | +34 B, about +230 operations |
| V filter | — | Same as H | +34 B, about +230 operations |
| Forward | 1 R8 load | Unchanged | 0 |
| Passes and pipelines | 3 passes, 17 pipelines | +2 full-screen passes, +2 filter pipelines (19) | Setup only |

**Totals:**

- **Bandwidth:** about +68 B per pixel, against today's worst case of about
  282 B: +24%. At 1920×1080 internal resolution that is about +141 MB per
  frame.
- **ALU:** about +550 operations per pixel, against roughly 4,500 for today's
  64 worst-case probes: about +12%.

**Attachment memory needs root's decision.** `T1` adds 1 B per pixel, so
`BYTES_PER_PIXEL` becomes 14. At `MAX_PIXELS` = 4,194,304 that is
58,720,256 B, or 56 MiB, which **exceeds the documented 52 MiB bound**. Root
must choose explicitly; the bound must not be raised silently.

| Option | Effect |
|---|---|
| (a) Raise the documented bound to 56 MiB | `MAX_PIXELS` unchanged |
| (b) Keep 52 MiB | `MAX_PIXELS = floor(52·2^20 / 14) = 3,894,710`. 2560×1520 still fits; 2560×1600 would then refuse, and the refusal stays typed |

## Art tuning (separate evaluation; not part of stage 1)

A5 is a parameter question. Evaluate it only after stage 1 passes N1–N10 at the
original settings, as its own bounded appearance comparison:

- a candidate art radius of 0.3–0.5 for props of about 1 m;
- optionally, a distance falloff of `1 - (|delta|/radius)^2` per blocked ray.
  It changes the binary-count contract, so the oracle changes again.

The original radius 2.0, bias 0.01 and strength 1.0 remain the regression
baseline for every numeric gate and the build003 comparison. No preset or
default enablement follows from either stage.

## Bounded verification run (proposal; needs root's grant)

- **Stage-1 build:** the existing runner and an unchanged plan structure.
- **Children:** the five numeric cases (73 frames) plus appearance (16 frames,
  16 PNGs). If root accepts it, add a `ssao_quality` numeric case with N5's
  isolated sphere and N8's stripes, each off and on in both projections:
  8 frames, for 97 in total, within the 128 new-frame cap. N6 and N7 use the
  existing `white_slab` and `edge_slab` frames through N4. No retries.
- **Then:** the analyzer, with N1–N10 and the oracle mutations, and my
  appearance review of the same 11 pairs against the build003 crops.

## Files expected to change (Astra)

- **Shaders:** `ao.wgsl` (R1, with `B` as a shader constant) and a new
  `filter.wgsl` (R3, H and V entry points).
- **Rust:** `math.rs` (the rotation table in `Uniform` and its assertions, the
  budget constant and the CPU reference), `pipelines.rs` (+2 pipelines),
  `targets.rs` (`T1`) and `draw.rs` (two passes).
- **Out of scope:** `prepass.wgsl`, the `cooked-materials` refusal, the
  receiver set and the forward substitution stay unchanged.
- **Helpers to update:** the 0070 `ssao_reference.py`, the analyzer intervals
  and N5 precondition, and the fixtures for N5 and N8.
