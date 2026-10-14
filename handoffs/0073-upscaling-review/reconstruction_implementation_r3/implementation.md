# 0073 reconstruction implementation, r3: status and rationale

**Status: PARTIAL, not buildable-verified and not accepted.** In this bounded turn I implemented the corrected evidence estimator end-to-end for the **ordinary (UP) program and its host**. The OWN program, texture LOD, DRS shading and the jitter sequence are **not** implemented here; each is listed in §4 with its exact remaining source path. No Cargo, naga, GPU or render was run.

**Model:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max; per the packet, root verified both through ACP.

**Base and patch:**
- Base: exact `76be9608e99bf089fcb1e0908827330f70c28ac1`.
- Patch: `reconstruction_r3.patch`, sha256 `774c4a683aa4267b8531256642b2a9fcea18669e453ce88e2c8148d72001d609`, 8 files.
- It passes `git apply --cached --check` against 76be (`patch_check.json`).
- Changed source copies are under `src/`.

**Unchanged:**
- the output sprite, text and BLEND route and its sanitation;
- the old packets, brief and runner;
- the three owner-attachment files;
- the top-level `result.md`.

## 1. What the ordinary resolve now does (`upscaling/resolve.wgsl`)

**Current evidence (clear-inclusive):**
- The declared support is the 3×3 internal texels around `floor(source_position(p))`.
- Every in-rect tap with positive weight contributes `w·clean(c)` and `w`. The weight is the in-tree Gaussian at the tap's true jittered output position.
- Clear is zero colour at full weight. Outside-rect taps contribute nothing.
- A malformed tap gains no weight and refuses history for the pixel.

**Fresh output:** `clean(S/m)`. Mixed or clear 2×2 support stays current-only, with depth 0 and evidence 0, and is never history.

**History:**
- Colour, depth and RG16F evidence `(mass, age)` are admitted together per bilinear tap. The tap must have finite colour, `a > 0`, compatible depth, finite mass in [0, 65504], and an integer age in [0, 8).
- The evidence must also be valid for the accepted slot (`reactive.w`).
- Admitted taps give `N = Σ b·W·c` and `C = Σ b·W` from the same support. `H = N/C` only when `C > 0`.

**Update:**
- `W′ = min(C·density, cap)·(1 − strength·reaction)`, where `density = old/new source-rect area` (`reactive.z`, exact rects).
- `H` is clamped to the existing 3×3 box; the clamp is a biased correction and adds no mass.
- `colour = clean((H·W′ + S)/(W′ + m))` and `W_new = min(W′ + m, cap)`. The age stays 0, because a surface is always present when history is stored.

**No prior:** no prior is stored. No-surface history is never stored, so retirement is immediate rather than relying on decay. The global mass (`reactive.y`) is unused by this program.

**Controls:** `controls.py` passes 16/16 checks. It transcribes these equations and includes all three contract defects and the root counterexample.

## 2. Host, transaction and validity

**Targets:** `Output.evidence[2]` is RG16Float at output extent; MRT bytes are 8+4+4 = 16. A startup format check refuses explicitly if RG16Float is unsupported.

**Bindings:** the UP resolve layout grows to 6 bindings (binding 6 = previous evidence). The third attachment is records for OWN and evidence for UP.

**Validity rule:** `evidence_valid` requires:
- the previous accepted frame was an ordinary resolve;
- the same `Output` allocation (`Arc::ptr_eq`);
- `has_previous`;
- not an owner frame.

**Lifecycle:**
- It is published only in `State::accept`, so a failed prepare or submit changes nothing.
- A native MetalFX frame calls `Pending::mark_native()` in `retained.rs`, under the feature gate, before acceptance. Native colour therefore never pairs with stale evidence.
- A cut, resize or owner toggle invalidates it.

**Admission (`plan.rs`, `motion/attachment.rs`):**
- `OUTPUT_PIXEL_BYTES` and `OUTPUT_STRIDE` go from 36 to 44, so the owner path rises from 52 to 60. The ceiling stays 671105024 B.
- Exact recomputed boundaries, after a maximal old set with ceil source extents:

| Case | Admitted | Refused |
|---|---|---|
| Ordinary, full scale | 1694² | 1695² |
| Ordinary, 8/16 | 1939² | 1940² and 2048² |

- `admit_records` vectors: the 2560×1440 ← 1920×1080 and 2560×1600 ← 2560×1440 transitions are now refused. The other totals are recomputed exactly (536,887,296, 640,106,496, 511,721,472, 1320 and 80·2²²).
- These are honest lower refusals. Queued, vendor and phone memory are excluded and unproven.

**Programs:**

| Program | Status | Source sha256 (common + resolve) |
|---|---|---|
| Upscale | changed | `f78bccaa…08f6` → `0d37e9af…5e2c` |
| UpscaleOwner | unchanged | `1b551b92…a280` |
| Composite | unchanged | `81cfe7c5…9b37` |
| MetalFX shaders | unchanged | `common.wgsl` unchanged |

**Cooker:** root must re-pin `Upscale` (new source and ABI: binding 6, RG16F target). The fixed TAA, default and pixel-camera paths are unchanged.

## 3. Rationale and visual intent (ordinary path)

**Detail:** true-position accumulation lets output pixels collect sub-internal-pixel samples, instead of blending interpolations. Static texture detail present in internal frames can exceed DRS, but the clamp still bounds Nyquist-level detail in UP.

**Coverage and edges:** clear-inclusive evidence gives correct partial coverage at antialiased edges.

**Cost:** omitting the prior trades the first-cycle disocclusion transient (sparser and noisier) for honest evidence.

## 4. Not implemented: exact remaining source paths

1. **OWN program (`resolve_owner.wgsl`, bindings 6–9 renumbered).** Port the same estimator, then add:
   - provenance for every positive tap, with conservative overflow and changed-secondary rejection;
   - RG16F age, with `age = 1 + max(contributor ages)` absent an actual anchor surface, retiring at 8;
   - static partial coverage persistence;
   - single-owner coverage reprojection in motion;
   - an OWN evidence attachment, with `writes_evidence` for owner frames.

   Until then OWN runs the unchanged old program, with global mass, and never writes evidence.
2. **Tests that must change before any GPU run:**
   - `upscaling/device_tests.rs` (UP 19 modes): new binding 6, a third attachment, and expectations from an f64 kernel helper.
   - `owner_device_tests.rs` cases 7 and 8: their baseline equality with the old UP resolve no longer holds and must be redefined.

   Both are `#[ignore]` GPU tests. They compile, but would fail at run time as written.
3. **Texture LOD for internal opaque and MASK (D-L):**
   - the models `Frame` grows from 96 to 112 B with `lod_bias = −log2(source.z/output.z)` for internal passes only;
   - `textureSampleBias` in `model_material.wgsl`, and the same bias in motion `mask()`;
   - output-resolution BLEND, sprites, text and the output depth re-raster keep bias 0;
   - cooked material pins change.

   Until then sub-internal-texel texture detail is unrecoverable.
4. **DRS shading:** the ordinary path still clamps every frame, so shading drift on DRS frames stays bounded, but the first-4/16 detail wipe remains. A justified shadow-guard invariant, or conservative rejection on real shadow incompatibility, is not implemented.
5. **Thin sampling:** the upscale-only split jitter (d05ea011 §3E) is not implemented, so the 0.4 px wire stays position-dependent.

## Limitations

- **Unverified:** not compiled, linted, naga-validated or rendered. `controls.py` checks equations only.
- **Open:** all historical failures stay open (ebc7e690, 9843a6c3, a5ae3179, GPU001 case 3, C7, C9, P1, the DRS wipe, thin geometry). Mixed-layer motion remains current-only.
- **Scratch:** `/tmp/incant-0073-reconstruction-r3-budget` (budget arithmetic). No memory was read or written.
