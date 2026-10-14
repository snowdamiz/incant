# 0073 latest: R5 numeric + quality execution (pointer)

- **Result:** `r5_quality/execution-result.md` and `.json`.
- **Model and settings:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max, auto-memory disabled.
- **Run:** one granted run of nine children, accepted and released. Release: `/Users/andreyyurlov/.codex/incant-evidence/0073-r5-numeric-quality-gpu-001/release.json`, sha256 `2a3abbff5d6d54a2366448ccea0fd89d4eca617d869926c31a134364116e26a3`.
- **Numeric:** all four strict GPU tests passed.
- **Appearance: REJECTED.**
  - No practical visual change from R4 in ordinary-UP moiré and thin lines, in moving UP/OWN aliasing, or at the 4/16 entry and post-cut frames.
  - Stationary OWN, clean disocclusion and the overlays are preserved.
- **Images:** all 15 PNGs were read.
- **Still open:** every historical gate (C7, C9, P1). No performance claim.

---

# 0073 result — output upscaling numeric execution, visual review and repair design

## Implementation 4 (current): owner attachment substrate within the existing memory admission

**Status: IMPLEMENTATION PACKET WRITTEN for root, design and CPU arithmetic
only.**
- The packet is in
  [`quality-repair-implementation-4.md`](quality-repair-implementation-4.md).
- The arithmetic control is
  [`repair-model/admission_v4.py`](repair-model/admission_v4.py), with its
  output in [`repair-model/results-4.json`](repair-model/results-4.json).
- No GPU, Cargo, target, image, production edit, account action, subagent,
  memory write or push.
- Addenda 1–3, their models and results, and the sections below are unchanged.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `claude-0073-owner-substrate-acp.log` header reads `Resumed the latest
ACP session scoped to this handoff worktree.` and `Claude session verified:
Opus 5.5; thinking: max`.

**Inputs, all read only:**
- the brief's priority revision (`11944b7a…`, uncommitted);
- root's foundation `temporal-owner-provenance.md` (`9208b343…`);
- the motion and upscaling plan source in `native-agent-play`
  (`codex/temporal-owner-provenance`, uncommitted over 2fe23dce; root's source
  SHA `e1f3cde4…`).

**Outcome:**
- **Slice:** an exact R32Uint owner target, attached to the *existing* motion
  pass as a second colour target.
  - It holds the row's integer word `id | changed << 31`, read through a new
    Uint32 attribute at offset 68 by a new `MotionOwner` kind.
  - The existing `Motion` and `Depth` kinds, and their cooked artifacts, stay
    byte-identical. No float lane carries an id.
- **Consistency:** depth, MASK discard, mirror twins and skinning follow from
  writing the owner in the same fragment as the motion vector, plus the
  foundation's ids.
- **Zero-owner cases:** clear, BLEND, overlays, discards and bypass.
- **Lifecycle:** a startup opt-in and per-scene attachment, off by default, that
  resets motion history when toggled. It fails closed when cooked artifacts are
  missing.
- **Colour:** no consumer yet, so colour output does not change.
- **Memory:** the cap stays 671,105,024 B and every existing cap and default is
  unchanged.
  - Today's preflight counts a full candidate set even when allocations are
    reused. With the owner target it would refuse steady 16/16 at 2560×1600
    and 2048×2048.
  - Proposed instead: count each live allocation once, with checked
    arithmetic. The owner slice is then admitted at every steady state and
    every DRS change. Only near-maximum output resizes refuse; for example,
    2048×2048 → 2048×1920 at 16/16 needs 686,833,664 B. A cut then a retry
    remedies those.
  - The later history slice needs this accounting first; without it, steady
    2560×1440 at 8/16 refuses.
- **Shared substrate:** portable and MetalFX share the substrate, with no extra
  pass. MetalFX attaches nothing until root approves 0074's owner variant.
- **Acceptance controls:** CPU contract, vertex-pulling, layout, admission and
  lifecycle tests, plus one bounded GPU readback test (32×32, at most 6
  frames, ignored without a grant).
- **Blockers and open gates:** later-history blockers are listed separately. The
  a5ae3179 visual FAIL, C7 and C9 as written, and P1 all stay open.

**Commands and results:**

| Command | Result |
|---|---|
| `python3 -I -B handoffs/0073-upscaling-review/repair-model/admission_v4.py --out …/results-4.json` | Exit 0; 3 of 3 checks pass: the cap is reproduced, 973,121,760 B is reproduced, and the grid admits nothing above the cap; under 1 s |
| Second run, compared with `cmp` | Byte-identical |
| `/usr/bin/python3` (3.9.6) run | Identical to 3.14.8 apart from the version field |

**Not run:** Cargo, cooker, renderer and GPU tests. The foundation is not
re-verified here.

**Screenshots:** none.

**Changed paths:**
- `handoffs/0073-upscaling-review/quality-repair-implementation-4.md` (new)
- `handoffs/0073-upscaling-review/repair-model/admission_v4.py` (new)
- `handoffs/0073-upscaling-review/repair-model/results-4.json` (new)
- `handoffs/0073-upscaling-review/result.md` (this section)

Root's `brief.md` is left uncommitted.

**Limits:**
- **Bytes:** logical only. They exclude pitch, tiling, padding, allocator and
  driver bytes, and queued work.
- **Resize sums:** approximate, because attachment rounding up is ignored. The
  refusal predicate itself is exact.
- **Claims:** no visual, performance, phone or phase-gate claim.

## Addendum 3 (preserved): bounded provenance support

**Status: BOUNDED CORRECTION WRITTEN for root review, CPU only.**
- The correction is in
  [`quality-repair-addendum-3.md`](quality-repair-addendum-3.md).
- The witnesses are
  [`repair-model/witness_v3.py`](repair-model/witness_v3.py), with their output
  in [`repair-model/results-3.json`](repair-model/results-3.json).
- No renderer source, GPU, Cargo, build, target or capture was used. No subagents,
  memory writes or pushes.
- **Byte-exact, checked with `git diff --quiet HEAD`:**
  - addendum 2 (`4ece422d…`);
  - `model_v2.py` (`57295562…`) and `results-2.json` (`cb9c256e…`);
  - addendum 1, `model.py` and `results.json`.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `claude-0073-provenance-support-acp.log` header reads `Resumed the latest
ACP session scoped to this handoff worktree.` and `Claude session verified:
Opus 5.5; thinking: max`.

**Inputs:**
- the brief's priority revision (`8157b230…`, root's, uncommitted);
- root's witness `upscaling-full-support-counterexample-20261013.json`
  (`47cf05d8…`);
- 0074's `native-coverage-repair-addendum.md` (`ecb7bba3…`), read only;
- `kernel_oracle.rs` and `common.wgsl` at 2fe23dce.

**Outcome:**
- **Root's decisions applied:**
  - footprints, not signatures;
  - exact provenance;
  - joint palettes compared when available, otherwise always changed;
  - C7, C9 and P1 kept explicit.
- **Root's support counterexample** reproduces bit for bit (0.29309, 0.12633,
  0.06040). With every positive-weight tap tracked, the emitter's removal drops
  the history to 0.
- **Radii separated:**
  - the kernel stays 3×3, untruncated;
  - the box, front owner and admission stay at `R`;
  - provenance covers the full support: owners, changed checks, support depths
    and footprint extent.
- **Exact extent:** every tap lies within 1.5 source texels of P per axis.
  E1 checks this over n = 4–16, fractional origins and all phases (max per-axis
  offset exactly 1.5 texels, max radius 2.1213). That gives support radii of
  2.14, 2.86, 4.29 and 8.57 px at 16, 12, 8 and 4/16.
- **Moving reprojection:** histories admitted along motion carry an unbounded
  flag. They make no "cannot miss" claim and drop on the first static frame
  with a caster rectangle. Static frames never read old-view depths (derived in
  section 6).
- **Overflow:** a 31-class mask; drop whenever authenticity cannot be shown.
  O1: 0.264 → 0. A colliding class causes an unnecessary drop (reported cost).
- **Union provenance:** also required in clip-in-place frames. U1 (an issue
  found in this pass): 0.0117 → 0.
- **Controls:** P1 is recorded as unresolved, separately from the new improved
  controls (S1, O1, U1, B1). C7 and C9 as written stay failed.
- **Shared substrate with 0074:**
  - stable per-placement ids;
  - an R32Uint `id | changed << 31` owner word (R16Uint cannot hold 100,000
    ids);
  - the change witness;
  - the provenance algebra.

  Nothing in MetalFX is redesigned, and no target is authorized.
- **Mechanics:** meta lane semantics change; `Portable` grows to 912 B; total
  973,121,760 B (+32 B).

**Commands and results:**

| Command | Result |
|---|---|
| `python3 -I -B handoffs/0073-upscaling-review/repair-model/witness_v3.py --out …/results-3.json` | Exit 0; 12 cases, 5 expected failures, none unexpected; about 1 s |
| Second run, compared with `cmp` | Byte-identical |
| `/usr/bin/python3` (3.9.6) run | Identical to 3.14.8 apart from the version field |
| `git diff --quiet HEAD` on the retained files | No changes |

**Not run:** Cargo, the cooker, renderer and GPU tests, and the retained and
addendum-2 suites. Root asked for no third full-suite run, so their numbers
describe addendum 2's rules.

**Screenshots:** none. No GPU was used.

**Changed paths:**
- `handoffs/0073-upscaling-review/quality-repair-addendum-3.md` (new)
- `handoffs/0073-upscaling-review/repair-model/witness_v3.py` (new)
- `handoffs/0073-upscaling-review/repair-model/results-3.json` (new)
- `handoffs/0073-upscaling-review/result.md` (this section)

Root's `brief.md` is left uncommitted.

**Limits:**
- **Witnesses:** single pixel, 16/16, with B1 as algebra. Slab containment and
  A1–A5 are derived, not simulated.
- **Not re-measured:** the costs of exactness (halo, overflow drops, rebase)
  and the retained and addendum-2 cases under the new rules.
- **Claims:** no appearance, performance, phone or phase-gate claim.

## Addendum 2 (preserved): shading changes, camera ownership and owner identity

**Status: CORRECTION WRITTEN for root review, CPU only.**
- The correction is in
  [`quality-repair-addendum-2.md`](quality-repair-addendum-2.md).
- The model delta is [`repair-model/model_v2.py`](repair-model/model_v2.py). It
  imports the unchanged `model.py`, and its output is in
  [`repair-model/results-2.json`](repair-model/results-2.json).
- No renderer source, GPU, Cargo, build, target or capture was used. No shader
  or resource change was made, and none should start before root's review.
- **Preserved unchanged:**
  - 7b3dc64c: `quality-repair-addendum.md`, `model.py` (`2c4b0bfc…`) and
    `results.json` (`c88992f9…`);
  - e1b26907 and all earlier evidence;
  - all 50 retained controls.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `claude-0073-admission-review2-acp.log` header reads `Resumed the latest
ACP session scoped to this handoff worktree.` and `Claude session verified:
Opus 5.5; thinking: max`. No subagents, memory writes or background work.

**Inputs, read first:**
- root's feedback `b85631cf…`;
- the readiness audit, found under `~/.codex/incant-evidence/` (no copy was
  under `~/.codex/incant-reviews/` during this session):
  - feedback `faae6df3…`;
  - implementation map `05a0b3fe…`;
  - review `85a05dc2…`;
  - counterexamples `2a7cdb3c…` / `fcb38800…`;
- root's oracle acceptance `935b1662…` and the oracle record at 2fe23dce;
- renderer and cooker source at 2fe23dce, read only.

**Outcome:**
- **Blocker 1, the scalar key.** It is withdrawn.
  - **Required replacement: changed-caster footprints.** The host lists each
    changed caster's swept light-space rectangle per shadowed light. Static
    pixels whose depth range meets one are clipped and reset.
  - **What that avoids:** no visibility is evaluated, so there is no
    quantisation, no collision, and no shadow map or lighting group in the
    resolve.
  - **N1:** lag 0.017, against 0.378 for the scalar key.
  - **Cost:** over-invalidation. K3's shadow-interior error is 0.039, against
    0.021 for a per-light 8-bit signature. That signature is measured as the
    alternative, with three stated gaps.
- **Blocker 2, the camera.**
  - **Definition:** `camera_static` is an exact host comparison with the
    accepted frame: the unjittered view and projection, and the output size and
    rectangle. It is separate from screen displacement.
  - **Rule:** unclipped persistence, and any world point rebuilt from stored
    depth, happen only with a static camera. Zero-motion pixels under a changed
    camera take the moving path, admitted against the expected previous depth.
  - **N2 and N3:** 0 persisted pixels and 0 shading tests while the camera
    changes, against 6,144 and 3,072 under the 7b3dc64c rules.
  - **F10:** current-depth admission gives 0.261, against 0.091.
- **Owners:**
  - **Current contributors:** persistence also requires every one to be
    unchanged. A newcomer, nearer or at the same depth, no longer keeps the
    old background (N5: 0.292 → 0.031 for both).
  - **Drops:** by identity (N4: 0.691 → 0.000).
  - **Moving admission:** by identity. In N6 a tolerance-close wall equals a
    separated one bit for bit; under the 7b3dc64c rules they differ by 0.715.
  - **Provenance:** kept as bookkeeping, with no measured benefit (P1).
- **Retained-scene events:** each one is classified:
  - topology → replacement → reset;
  - transforms and skins → owner change;
  - the camera;
  - light transforms and cascades → frame-global;
  - sprites, text, tilemaps and Blend → current-only overlay.

  This corrects addendum 1: environment and light-list changes force a
  replacement.
- **Mechanics:**
  - stable per-placement ids, because packed rows shift on handedness flips;
  - exact `u32` transport: row lane 12 → flat varying → `R32Uint` target;
  - `Rgba32Uint` meta with canonical reads;
  - an explicit 10-binding resolve layout with its Metal, DX12 and Vulkan map;
  - cooker contract and emitter changes, with negative controls;
  - no-light behaviour;
  - an accepted-state witness;
  - an exact ceiling of 973,121,728 B (671,105,024 B today).
- **Numbers:**
  - root's oracle is cited as definition-specific;
  - the brief's 8 ± 0.07, < 10⁻⁴, C3 activity and C9 claims are corrected;
  - addendum 1's 0.65/0.85 is withdrawn as a target;
  - the original C7 and C9 are recorded as failed;
  - replacement controls await root's review, and no GPU gate changes.

**Commands and results:**

| Command | Result |
|---|---|
| `python3 -I -B handoffs/0073-upscaling-review/repair-model/model_v2.py --out …/results-2.json` | Exit 0; none unexpected. 50 retained cases per detector (2 detectors) and 21 new cases, 10 of them expected failures. About 50 s |
| Second run, compared with `cmp` | Byte-identical |
| `/usr/bin/python3` (3.9.6) run | Identical to 3.14.8 apart from the version field |
| `shasum -a 256` on `model.py` and `results.json` | Unchanged: `2c4b0bfc…`, `c88992f9…` |

**Not run:** Cargo, the cooker, renderer tests and GPU tests. They are outside
this preparation-only packet.

**Screenshots:** none. No GPU was used.

**Changed paths:**
- `handoffs/0073-upscaling-review/quality-repair-addendum-2.md` (new)
- `handoffs/0073-upscaling-review/repair-model/model_v2.py` (new)
- `handoffs/0073-upscaling-review/repair-model/results-2.json` (new)
- `handoffs/0073-upscaling-review/result.md` (this section)

Root's uncommitted `brief.md` is left as found and is not committed.

**Open questions for root:**
1. Footprints (exact, over-invalidating), or the per-light signature with its
   gaps?
2. Keep provenance?
3. Numerical review of the replacement controls.
4. Palette comparison for animated owners, or always changed?

**Limits:**
- **Model:** grey 2D. It has no world points, light space or cascades. The
  footprint slab and assumptions A1–A5 are argued, not modelled. The dolly's
  wrong world point is not modelled, so N3 is a structural check.
- **Claims:** no appearance, performance, phone or phase-gate claim.

## Disocclusion addendum (preserved): static-history correction

**Status: CORRECTION WRITTEN, CPU only.**
- The correction is in
  [`quality-repair-addendum.md`](quality-repair-addendum.md).
- The reproducible model is
  [`repair-model/model.py`](repair-model/model.py), with its output in
  [`repair-model/results.json`](repair-model/results.json).
- No renderer source, GPU, Cargo, target or capture was used.
- `quality-repair-brief.md`, the a5ae3179 evidence and all earlier records are
  unchanged.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `claude-0073-disocclusion-design-acp.log` header reads `Claude session
verified: Opus 5.5; thinking: max`. No subagents, no memory writes.

**Inputs:**
- root's review `9649659c…`;
- its proof `b8677710…` / `27df57b3…`;
- the brief as root left it, uncommitted (not modified or committed here).

**Outcome:**
- **The counterexample holds.** The model reproduces it at both scales, over
  black and over clear. The draft ghosts at up to 0.96 after 1 frame and 0.63
  after 8.
- **Corrected design.** Static pixels keep unclipped history only while every
  stored contributing owner is unchanged. Owner ids and a per-frame changed set
  are explicit metadata on the portable path. Otherwise the pixel drops its
  history (the front owner left) or clips in place against near samples.
  Moving pixels reproject along their own surface's motion and admit only
  same-motion surfaces.
- **Results.** The corrected rules leave zero residual in root's case, in the
  partial-coverage band, after a start-stop, a sweeping occluder, a triple
  junction and a camera pan. Static thin wires still persist in every phase.
- **C7.** C7 as written measured disocclusion convergence, not ghosting. The
  corrected design has 0.000 signed bias and equals a matched-reset floor
  exactly. C7 is replaced by stricter ghost controls (C7a, C7b, C7d), and C7 as
  written is kept as a visible expected failure.
- **Corrected numbers.**
  - C9 recovery is 0.68/0.87, measured against the converged steady state.
  - The reset value is 0.125.
  - The interior mass bound applies only where all 9 taps are valid.
- **Shading key.** The worst case is stated as 72 comparisons plus 9
  exponentials. The gated variant is identical (K2), and the gather variant is
  rejected (K1).
- **Also covered:** exact portable accounting (973,147,152 B, against 671,105,024
  B today; MetalFX unchanged) and coordinate and edge rules.

**Commands and results:**

| Command | Result |
|---|---|
| `python3 -I -B handoffs/0073-upscaling-review/repair-model/model.py --out …/results.json` | Exit 0; 50 cases, none unexpected; about 10 s |
| Second run, compared with `cmp` | Byte-identical output |
| `/usr/bin/python3` (3.9.6) run | Same values as 3.14.8 |

**Screenshots:** none new. No GPU was used.

**Changed paths:**
- `handoffs/0073-upscaling-review/quality-repair-addendum.md`
- `handoffs/0073-upscaling-review/repair-model/model.py`
- `handoffs/0073-upscaling-review/repair-model/results.json`
- `handoffs/0073-upscaling-review/result.md` (this record)

**Decisions applied, as root gave them:**
- one combined review after the full algorithm passes correctness;
- the label fixture is unchanged for that comparison and fixed afterwards;
- meta and owner data exist on the portable path only;
- the ceiling uses exact accounting.

**Limits:**
- The model is grey 2D with analytic scenes. It has no perspective, parallax,
  MASK, HDR, overlays or half-float quantisation.
- It checks the design's logic, not GPU numerics.
- No appearance, performance or phone claim is made.
- No phase gate is claimed or approved.

## Repair brief (preserved): portable reconstruction quality repair

**Status: BRIEF WRITTEN for Astra.**
- The design is in [`quality-repair-brief.md`](quality-repair-brief.md).
- This session did CPU and source analysis only: no GPU, capture, build, target
  use, source edit, fixture edit or helper edit.
- Production stays at `535003d7`; root schedules the repair and a new review.
- The a5ae3179 run, its fixture, thresholds, captures and evidence are unchanged.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `claude-0073-quality-repair-prep-acp.log` header reads `Claude session
verified: Opus 5.5; thinking: max`. No subagents, no memory writes.

**Inputs:**
- brief `d8a99ff9…` (root's `7bb19bcd`), unchanged at commit;
- engine source `535003d7`, read only from git objects;
- the retained a5ae3179 evidence and screenshots.

**What the brief specifies:**
- **Causes, separated by status.** Six are verified against source lines and
  measurements. One is derived. Three remain hypotheses.
- **Sample reconstruction.** Jitter-aware kernel accumulation with a per-pixel
  confidence that maps the existing `history_weight` onto sample density.
- **Edges and coverage.** Premultiplied accumulation that drops the current-only
  edge rule.
- **History admission:**
  - persistence on static pixels;
  - a shading-change key from directional shadow visibility;
  - a removal guard and frame-global suspension;
  - depth-tested, clipped history on moving pixels.
- **Mip selection.** A capped bias, `−min(log2 r, 1)`, that may land only with
  the persistence rule.
- **Also covered:**
  - behaviour at each scale;
  - data paths and a load/byte budget;
  - risks;
  - 18 numerical controls (11 CPU oracle, 4 GPU raw, 3 retained), all in
    existing test files;
  - my acceptance plan for the next review, with FULL/DRS hash preconditions.
- **Fixture label.** A separate one-literal correction for the clipped UI label,
  not applied.

**How the expectations were made.**
- I used a throwaway CPU model with the real jitter table. It reproduced the
  measured a5ae3179 chart contrast for `535003d7`: 0.00 / 0.05 / 0.42 vs
  measured 0.03 / 0.06 / 0.43.
- I then used it to:
  - reject two weaker designs (a colour-ratio shading test, and the bias with the
    existing clamp);
  - set the kernel width, bias cap, confidence cap and reset values.
- It was CPU-only scratch in `/tmp`. It is deleted and not committed. Its numbers
  are expectations, not results.

**Screenshots:** none new. The brief refers to the a5ae3179 screenshots in
`screenshots/`.

**Changed paths:**
- `handoffs/0073-upscaling-review/quality-repair-brief.md`
- `handoffs/0073-upscaling-review/result.md` (this record)

**Not run:**
- GPU;
- builds;
- tests;
- captures.

The repair's numerical controls are for Astra to implement.

**Open questions for root:**
1. **Order.** Should one review cover all three landing steps (2.1–2.3, 2.4–2.5,
   2.6), as the brief recommends, after Astra's controls pass?
2. **Label.** Apply the label correction in the repaired-source review's tree,
   or after that review?
3. **Memory.** Accept the larger logical ceiling (768 MiB + 16 KiB), or allocate
   the meta pair only for the portable path?

**Limits:**
- The expectations come from a simplified grey 2D model of one stage. Perspective,
  sRGB textures and real lighting may shift them.
- No appearance or performance claim is made.
- No phase gate is claimed or approved.

## Execution (preserved): corrected portable visual review

**Status: EXECUTED, and the GPU is released. Hard results PASS; my quality
verdict is FAIL.** Details are in [`visual-result.md`](visual-result.md).
- **Contract:** all 20 contract checks pass, and the fail-closed `analyze.py
  check` passes with no problems.
- **Quality:** the 8/16 upscaler converges to a stable but soft image.
  - Accumulation removes texture detail that the current internal frame still
    has; converged chart contrast is below plain DRS's.
  - Edges and sub-pixel geometry stay phase-unstable.
- **What works:** current-only overlays and physical UI are stable; history
  survives internal-only scale changes; the cut resets exactly; the fractional
  viewport is clean; there is no motion ghosting.
- Production is unchanged; the findings go to coordinated repair. This is not a
  phase completion, performance result or phone result.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `claude-0073-visual-execute-acp.log` header reads `Claude session
verified: Opus 5.5; thinking: max`. No subagents.

**Inputs (all hashes verified before the first GPU step):**
- brief `1fc2219d…` (root's `32d6d6bb`);
- visual tree `7e6d4b08…`;
- `review-ready.json` `ebf34321…`;
- `build/review.json` `b52c6918…`;
- frozen helper `786adfbb…`;
- root build receipt `6109ca8b…`;
- prior 0070 release `aa8fd844…`;
- engine source `535003d7…`, with freeze `bd8295b0…` exact over 2,448 build
  inputs.

This is the default portable WGSL path only; it is not cooked and not native
MetalFX.

**Commands and results.** All ran once, serially, in the foreground. The build
was root's supervised build: no Cargo, build, source edit, helper edit or target
access in this session.

| Command (`python3 -I -B …/visual/tools/`) | Result |
|---|---|
| `gpu.py list` | Exit 0 |
| `gpu.py review probe` | Exit 0, 8/8 checks; Apple M5 Pro, Metal |
| `gpu.py review stationary` | Exit 0, 36 frames, 5/5 checks |
| `gpu.py review motion` | Exit 0, 24 frames, 3/3 checks |
| `gpu.py review transitions` | Exit 0, 29 frames, 4/4 checks |
| `gpu.py release` | 89/96 frames, 13/16 PNGs; no live or owned process, no stop, no refusal |
| `analyze.py check` | Exit 0, verdict pass, complete |
| `analyze.py publish` | 37 receipts and 13 PNGs bound to `summary-2026-10-13T082346…` |

**Screenshots:** 13 PNGs at 960×540 in
`handoffs/0073-upscaling-review/screenshots/`:
- `stationary-{full,drs50,up50,phases,detail,activity}`;
- `motion-{full,up50,detail}`;
- `transitions-{full,up50,strip,cut}`.

**Changed paths:**
- `handoffs/0073-upscaling-review/visual-result.md`
- `handoffs/0073-upscaling-review/result.md`
- `handoffs/0073-upscaling-review/screenshots/` (13 PNGs)
- `handoffs/0073-upscaling-review/evidence/visual/` (manifest plus 37 files)

Outside the repository I added only the create-only `console/` captures in the
evidence directory.

**Not run:**
- retries;
- cooked-temporal or native MetalFX routes;
- other backends;
- regression suites;
- phones;
- performance.

**Open questions for root (coordinated repair):**
1. Should the portable resolve move to output-resolution sample accumulation,
   with an edge history policy and a texture LOD bias? Should the repaired build
   then be reviewed with this unchanged fixture, for a like-for-like comparison?
2. Should a later grant include a longer 16/16 segment, so that the review shows
   where 16/16 settles? Three frames were too short.
3. The fixture's physical UI label clips its own text. Should a later fixture
   revision fix that? It does not affect any contract check.

## Revision 1 (preserved): fail-closed analyzer and corrected-source readiness

**Status: PREPARED, not run.** The root review blocker is fixed, and the stage is
ready for root's supervised CPU build grant. Details are in
[`revision-result.md`](revision-result.md).
- No Cargo build, target, compile, GPU child, render or capture ran in this
  revision.
- No appearance, performance, phone or gate claim is made.

**Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max.
Root's `claude-0073-analyzer-repair-acp.log` header reads `Claude session
verified: Opus 5.5; thinking: max`.

**Inputs:** brief `cd7c45b7…` (root priority revision `15ddda49`) and root review
receipt `d307b665…`; both hashes were verified.

**Applied:**
- **`analyze.py` fails closed.** It validates exact complete plan cases, the
  plan's contract checks, the current ledger, debit, report, PNG and log
  bindings, and every independent comparison. It always retains a diagnostic
  summary and exits nonzero on any problem. `publish` accepts only the latest
  passing summary whose bindings still match.
- **Source pinning comes from root grants.**
  - `build-ready.json` names a commit object and root's freeze by SHA-256.
    Every build input of the exported commit must match that freeze exactly.
  - `review-ready.json` binds the build record.
  - `cooked-*` and `native-metalfx` builds are refused.
- **Corrected source read-only at `9a65b6c8`.** The fixture's contract checks
  are independent of the changed math, so its source is unchanged. Three prose
  expectations are superseded: overlays (now unjittered), motion disocclusion
  and the viewport border.

**Evidence (CPU only, receipts in `visual/preparation/r1-*.json`):**
- the 17 unchanged lifecycle controls pass;
- 17 analyzer controls pass: one positive, the rest negatives (missing
  case/report/PNG, bad hash, replaced build record, false
  contract/tile/statistic/UI/chrome, failed child, stop latch, zero-verdict
  evidence, stale or superseded summary, CLI exit codes);
- 10 source-pinning controls pass;
- selfcheck reports `all_pass`.

**Frozen identity:** commit `9bcd1e0e`, visual tree `09271a62…`; fixture
`f8ec6311…` (unchanged).

**Screenshots:** none; nothing has been rendered.

**Root's next actions:**
1. The final committed corrected source and its freeze, generated from that
   commit. The current generation-2 freeze for `9a65b6c8` includes 11
   uncommitted rows and an uncommitted doc, so it would be refused.
2. `build-ready.json` for the supervised CPU build.
3. After the build, `review-ready.json`.

**Limits:**
- The fixture has not been compiled against the corrected source.
- This covers the portable path only.
- StageA stays historical evidence for the `0452c5f5` binary only.

## Revision 0 (preserved; only its headings are nested one level)

### Status

- **StageA (granted once): executed and passed.** Both exact tests ran once on
  Apple M5 Pro (Metal) and exited 0. The GPU was released with no owned processes.
  - All 28 printed scalar rows match independently derived expectations; worst
    absolute error 1.46e-4.
  - The release, ledger and debit chain re-verifies.
  - Details: [`numerical-result.md`](numerical-result.md).
- **StageB: prepared only, not run.** The fixture, frozen plan and guarded tools
  are committed for a future root grant after source review.
  - No build, compile, GPU child, render or capture ran for this stage.
  - Details: [`preparation-result.md`](preparation-result.md).
- **Not claimed:** appearance, image quality, performance, a reference phone,
  MetalFX, the cooked-native path, or any phase-gate approval. The numeric pass
  is not used as appearance evidence.

### Model, effort, brief

- **Model:** Claude Opus 5.5 (`claude-opus-5-5`).
- **Thinking effort:** max. Root's
  `~/.codex/incant-evidence/claude-0073-upscaling-acp.log` header reads
  `Claude session verified: Opus 5.5; thinking: max`; I read only that header.
- **No auxiliary agents.** No subagent or reviewer was used.
- **Brief:** `handoffs/0073-upscaling-review/brief.md`, SHA-256 `13d7b351…73a1`,
  committed by root in `3caadfee`.
  - It has no priority revision or added director feedback, and it was unchanged
    at the final commit.
  - I applied its constraints: the single StageA launch; preparation only for
    StageB; no push, PR, memory edit or credential access; Built-by: claude.
- **Files left to root:** `numeric.py` and the brief are root's and are
  unmodified.

### Commands and results

| Command | Result |
|---|---|
| `python3 -B handoffs/0073-upscaling-review/tools/numeric.py` (once, console captured create-only in the grant directory) | Exit 0. Child 1: 1.155 s, exit 0. Child 2: 0.391 s, exit 0. Both report `1 passed; 0 failed`. Released: 2 children, `owned_processes: []`, `ledger_pids_alive: []` |
| `python3 -I -B handoffs/0073-upscaling-review/tools/numeric_receipt.py` | 11/11 chain checks true; 28/28 rows within 1.5e-4. 11 raw outputs retained as deterministic gzip; the evidence directory was unchanged |
| `python3 -I -B …/visual/tools/selfcheck.py --write …/preparation/selfcheck.json` | `all_pass: true`. All 12 guarded commands refuse without a grant and touch nothing |
| `python3 -I -B …/visual/tools/test_lifecycle.py --receipt …/preparation/lifecycle.json` | 17/17 (real children; debit, timeout, INT/TERM/HUP, mask, refusals, release) |
| `python3 -I -B …/visual/tools/test_analyze.py --receipt …/preparation/analyze-selftest.json` | Pass, 6/6 |
| `rustfmt --edition 2024` via stdin on the fixture | Parses; formatted; `--check` clean. No Cargo target or compilation |

**Unavailable by design.** These were not run, because the brief forbids them in
this stage:
- compiling the fixture;
- clippy;
- any GPU appearance run;
- regression suites.

### Screenshots

None. StageA reads back scalar rows only, and StageB has not run. The 13 planned
PNGs (960×540) are listed in `visual/plan.json`; after a grant they will be
published to `handoffs/0073-upscaling-review/screenshots/`.

### Changed paths

- `handoffs/0073-upscaling-review/numerical-result.md`
- `handoffs/0073-upscaling-review/tools/numeric_receipt.py`
- `handoffs/0073-upscaling-review/evidence/numeric/` (`receipt.json` and 11 `<sha256>.gz`)
- `handoffs/0073-upscaling-review/visual/` (plan, fixture, 8 tools, 3 preparation receipts)
- `handoffs/0073-upscaling-review/preparation-result.md`
- `handoffs/0073-upscaling-review/result.md`

**Commits:**
- `98b4d266`: StageA record.
- `290c1e35`: StageB visual tree `3e328c47…`.
- The commit adding this file and `preparation-result.md`.

Every commit carries `Built-by: claude`.

**Outside the repository**, I added only the create-only console capture
`runner-console.{stdout,stderr}` beside root's StageA outputs in
`~/.codex/incant-evidence/0073-upscaling-numeric/`. Nothing was pushed.

### Rationale

**StageB visual direction (mine).** One 960×540 physical output is compared
across FULL, DRS 8/16 and UP 8/16, using identical crop rectangles.
- **Why this stage.** It concentrates the cases where output-resolution
  reconstruction differs by design:
  - edges and sub-texel detail that the shader keeps current-only;
  - accumulated interiors;
  - texture minification at internal resolution;
  - current-only overlays;
  - physical UI.
- **Jitter evidence.** The full 8-phase cycle is shown directly through phase
  strips, an activity map and per-phase wire presence.
- **Motion, scale changes and cuts.** These run as short sequences with exact
  FULL and DRS controls.
- **Budget.** 89 of 96 frames leaves a reserve, and 13 of 16 PNGs.
- **What decides a case.** Only deterministic contract facts decide pass or fail;
  image statistics are observations.

**Tooling.** The tools reuse 0068's reviewed helpers rather than building a new
framework. They are tightened where the brief asks:
- the debit is written and fsynced under the packet lock and the shared GPU lock
  that cover the launch;
- interrupts are held pending until the attempt is recorded;
- every child starts with an empty signal mask;
- a cooked-feature build is refused;
- a failure ends the stage.

**StageA observation.** In mode 7, two composite channels are stored one
half-float step below round-to-nearest, which is consistent with
round-toward-zero conversion. This is within the test's 1e-3 tolerance and is
not a defect claim.

### Open questions for root

1. **Target and pins.** Which `~/.codex/*-target` should the grant name, and
   which prior release(s) should it pin? StageA's `gpu-released.json` is
   `fe268a84…6678`.
2. **Compile first?** The fixture is uncompiled. Should the first grant allow
   only `lease`, `checkout`, `toolchain` and `review` (CPU), with GPU steps
   afterwards? As written, a compile error stops the stage before any GPU frame
   and needs a new tree.
3. **UI counting.** Should UI submissions keep counting against the 96 frames?
   This is conservative; there are 10 of them.
4. **Regression suites.** Should the existing ignored lib/`model_gpu` suites run
   with the same frozen inputs, budgeted separately?

### Limits

- **StageA:** one local Metal adapter and scalar readback. Its 22 actually
  submitted frames are derived from source and were not instrumented.
- **StageB:** an uncompiled fixture and no visual evidence. Crop placement is
  projected and unconfirmed until frames exist.
- **Not covered:** phones, performance, MetalFX, cooked-native artifacts and
  other backends. The 640 MiB + 16 KiB logical ceiling is not a phone budget.
