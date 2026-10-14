# 0073 R5 focused production repair: source and scalar controls

**Status:** source patch complete, with scalar and source controls passing.
- It is **compile-unverified, not cooked and not rendered.** No pixel, compiler or GPU result exists for R5, so no appearance claim is made.
- Root and Astra review, compile and cook next. A separately granted bounded render must then judge the pixels.

**Model:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max; per the packet, root verified both through ACP before prompting. Auto-memory was disabled, and no memory or preferences were read or written.

**Turn bound:** 20 minutes. I started at 22:17:56 local (monotonic 124512.51). The patch, controls and inventory were done by about 22:23.

**Base:**
- Candidate: `825ff047284e84252d00f786a7b14cb6ecb44293` (production-equivalent `72b1ba8ec351e86d66a90c401e0a75a530633616`).
- Every mirror's baseline copy was checked against `baseline.json` and the 825ff Git object by `inventory.py`.

**Patch:** `repair.patch`, sha256 `8e226ad36180561868b899b54188fabfa2a4bcf692bc0ec43df5e7b1e6bc8253`, 7 files.
- It passes `git apply --check` against the exact 825ff copies in `/tmp/incant-0073-r5-*`.
- Applying it reproduces the `src/` mirrors byte for byte (`inventory.json`).

## Narrow review correction (supersedes the rationale below where they conflict)

- **LOD floor −1.5:** an *empirical visual candidate* chosen after the R4 4/16 artefacts. It is not a proven alias-free or optimal band limit. Eight correlated jitter positions imply no sampling theorem, so the earlier √8-per-axis argument is withdrawn from the source comments, controls and rationale.
- **Motion slack r/16:** also an *empirical visual candidate*.
  - It is not a positional-quantization bound, because jitter positions do not quantize reconstructed history.
  - It is not a same-motion proof: distinct same-depth surfaces moving by less than the slack can share one history.
  - The 0.04-px spread in the controls is **synthetic**, not a projected dolly.
- **Reaction model:** the controls use a **constant-reaction linearized scalar model**, not measured aliasing or the full nonlinear shader.
  - The pre-add effective history is `W = min(C, cap)·k`.
  - At |H−fresh| = 0.5: k = 0.9375, C = 16, W = 15, current fraction 1/16, 8-phase transfer amplitude 0.084.
  - At contrast 1: k = 0.875, C = 8, W = 7, fraction 1/8, amplitude 0.172.
  - R5 cap: C = 36, k = 1, W = 36, fraction 1/37, amplitude 0.036.
  - The earlier figures (0.079 and 0.152) used C in place of W and are corrected.
- **Still open, not repaired:**
  - ordinary-UP thin lines over clear support;
  - current-only reconstruction at low scale (4/16 entry, post-cut).
- **No pixel acceptance:** the R4 FAIL stands.
- **Hashes:**
  - patch `8e226ad36180561868b899b54188fabfa2a4bcf692bc0ec43df5e7b1e6bc8253`;
  - Upscale pin `ac7ecdf49b844713e1b35666cbea8bf7a4789edba3b12a1dd1787e88d5845e5d`;
  - UpscaleOwner pin `81eb48402b9d1c92b3b64b92bbae833cb711348152654377d21f3ba5c34645da`.

## What the R4 pixels showed, and what each change addresses

Evidence is the R4 run `974ee69e`. Images are in `0073-r4-quality-gpu-001/*/artifacts/`.

**Stationary OWN versus everything else:**
- Stationary OWN, which uses the static branch (own-texel persistence, no clamp, no reactive mass loss), is close to FULL.
- Stationary ordinary UP (`stationary-up50.png`, detail row 3 column 3) and *moving* UP and OWN (`motion-up50` / `motion-own50`, identical) show the same card-stripe moiré, noisy checker and broken posts.
- Both of those use the shared ordinary branch.

So the defect lies in that shared branch's accumulation, not in the estimator's evidence. Three changes follow.

### 1. Reaction now measures clamp displacement (both programs, ordinary branch)

**Location:** `resolve.wgsl` and `resolve_owner.wgsl`.

**Before (825ff):** `reaction = |H − fresh| / reactive_threshold`.
- `fresh` is one frame's aliased estimate, so at high-contrast texture it differs from the history in every frame.
- With the fixture config (h 0.9, threshold 4, strength 0.5), the history mass is held at 16 at contrast 0.5, and at 8 (one jitter cycle) at contrast 1, instead of the cap of 36.
- The leftover 8-phase aliasing amplitude is 0.079 and 0.152 respectively, against 0.036 at the cap (controls 3–5).

**After:** `clamped = clamp(H, box)` and `reaction = |H − clamped| / threshold`.
- History inside the contributing support's range keeps its mass.
- History outside that range still loses mass and is clamped: a history of 1.0 against the range [0, 0.2] gives reaction 0.2 and is clamped to 0.2.
- Even with reaction 0, the clamp still removes out-of-range stale colour every frame.

**Unchanged:**
- the clamp itself;
- density, cap and reaction ordering (`W′ = min(C·density, cap)·(1 − s·reaction)`);
- associated `N/C`;
- the OWN static branch, which never used reaction;
- the age and provenance rules.

**Failures targeted:** UP stationary texture moiré, and moving texture noise wherever history is admitted.

### 2. Motion-coherence slack is one jitter-lattice step (both programs)

**Before:** `MOTION_SLACK = 1/256` output pixel, the R4 bound. It refused almost all perspective motion (dolly, rotation), so moving content fell back to current-only.

**After:** `motion_slack() = (output/source)/16` output pixels, that is 1/16 internal pixel: 1/16 px at r=1, 1/8 at r=2, 1/4 at r=4. Accumulated history is positioned only to the 1/16-internal-pixel lattice of `UPSCALE_SAMPLES`, so smaller disagreement lies below the history's positional resolution.

**Still enforced:**
- every contributor needs valid motion;
- previous-depth compatibility uses the unchanged tolerance;
- the continuation-002 counterexample (an independently moving coplanar contributor, 192 px apart) is still refused;
- a difference equal to the slack is refused.

**Unchanged:** the OWN static proof keeps its 1/256 anchor test (controls 1, 7–11).

**Failures targeted:** moving UP and OWN current-only aliasing, broken thin features and jagged silhouettes.

### 3. The LOD bias is floored at −1.5 (host only)

**Location:** `temporal/jitter.rs` (`MIN_LOD_BIAS`), `upscaling/owner.rs` (`Pending::lod_bias`) and `motion/retained.rs` (MASK bias). Both use the same floor.

**Why −1.5:** one eight-phase cycle supplies 8 sample positions per internal texel, or √8 per axis. Sharpening past −log2(√8) = −1.5 adds aliasing that accumulation cannot resolve.

**Effect:**
- It binds only at 4/16 (bias −2 → −1.5) and 5/16 (−1.678 → −1.5).
- 6/16 to 16/16 are unchanged, including −1 at 8/16 and 0 at 16/16.
- Output BLEND, depth re-raster, default and fixed TAA keep 0 (controls 12–14).

**Failure targeted:** the first-frame 4/16 DRS entry artefact (partly).

### Synced cfg(test) oracle

`upscaling/evidence_reference.rs` is the f64 mirror of the shader that the evidence and device tests compare against. It now applies the same displacement reaction and the same r/16 slack.
- This keeps the reference equal to the shader. It is not a weakened expectation.
- Its static-proof 1/256 remains.
- The independent cfg(test) quantization correction (`half_bound`) is untouched.

## Inventory and ABI

**Changed files** (sha256 in `inventory.json`):

| File | Kind of change |
|---|---|
| `upscaling/resolve.wgsl`, `upscaling/resolve_owner.wgsl` | Shader |
| `upscaling/evidence_reference.rs` | cfg(test) oracle |
| `upscaling/owner.rs`, `motion/retained.rs`, `temporal/jitter.rs` | Host |
| `incant_shader_cook/src/temporal/mod.rs` | Trusted pins |

**Trusted pins** (common + "\n" + program):

| Program | 825ff | R5 |
|---|---|---|
| Upscale | `de4df804…d3a3` | `ac7ecdf49b844713e1b35666cbea8bf7a4789edba3b12a1dd1787e88d5845e5d` |
| UpscaleOwner | `0897bfd2…eb81` | `81eb48402b9d1c92b3b64b92bbae833cb711348152654377d21f3ba5c34645da` |

Both are written in the existing rustfmt layout, and both match the mirrors.

**ABI:** no uniform, binding, target, format, pass, pipeline or allocation changed. Memory cost is unchanged and the aggregate cap is unchanged. Contract strings are unchanged.

**Unchanged programs:** Composite, MetalFX pack and normalize, Motion, Depth, MotionOwner, Material, Resolve and Copy.

**Unchanged behaviour:** the output-space sprite, text, BLEND and UI route, fixture 650316a, the 15-image protocol, the 24-frame 8-phase schedule and all historical thresholds.

## Not addressed: open questions for Astra and root

1. **Ordinary UP thin lines over clear sky** (`stationary-phases.png` rows 1–2):
   - UP stores no history when the 3×3 support mixes one opaque surface with clear, because `single` requires every contributor to be opaque. Wires over clear sky therefore stay current-only and phase-dependent.
   - A candidate extension: admit one-surface-plus-clear support, carry partial coverage with that surface's motion, and admit history taps with alpha below 1.
   - That changes R4's "ambiguous UP history unusable" contract, so I did not change it unreviewed.
   - **Question:** may UP reproject partial coverage with the single surface's motion when the remaining contributors are clear (alpha 0, no depth)?
2. **The 4/16 entry and post-cut frames** (`transitions-strip.png`, frame 14; row 3 columns 3–4):
   - With no usable history, the current-only Gaussian has ρ = 0.35 + 0.325·(r−2), which is 1.0 output px at r=4, against a 4-px texel spacing. Reconstruction is then close to nearest-neighbour (blocky).
   - Fixing it means changing ρ, or adding a display-only interpolation when no history is admitted. That touches the kernel oracle (`kernel_oracle.rs`, `kernel_tests.rs`) and the estimator contract (no stored prior), so it was not done.
   - The −1.5 LOD floor only reduces the aliasing component.
3. **Exponential persistence in the ordinary branch:** history whose colour stays inside the current range persists with exponential decay rather than finite age, as in 825ff. Out-of-range stale colour is clamped immediately. Finite age retirement applies only to OWN's no-surface static persistence, as before.

## Commands

```sh
cd handoffs/0073-upscaling-review/reconstruction_r5
python3 -I -B inventory.py --checkout /Users/andreyyurlov/.codex/worktrees/owner-quality-repair/incant   # patch applies; pins match
python3 -I -B controls.py --out controls.json                                                              # checks=22 failed=0
```

## Limits

- No Cargo, rustc, clippy, fmt, naga, cook, GPU, capture or image was run.
- The Rust edits were hand-formatted to rustfmt style but not checked by rustfmt.
- The scalar controls transcribe the rules. They cannot judge appearance.
- All historical failures remain open: C7 and C9 (historical definitions), P1, the DRS wipe, UP thin lines and moving detail.
- The R4 rejection stands until R5 pixels exist.
- No screenshots were produced; the only images read were existing R4 artefacts.

**Writes:** only new files in this packet: `src/`, `repair.patch`, `inventory.py/json`, `controls.py/json` and `result.md/json`. The baseline, `baseline.json`, the brief, the old packets and untracked inputs are untouched.
