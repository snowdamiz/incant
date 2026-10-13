# Portable reconstruction CPU oracle

This is a test-only numerical foundation for sections 2.1–2.3 of Claude 0073's
captured reconstruction proposal. It does not change production WGSL, history
admission, resource layouts, material LOD, cooked shader identities or defaults.
The base is `077f7c39c0f4122aae36ea3a88537be006722822`; the accepted fixed-temporal
coverage branch remains separate. [The packet](evidence/upscaling-kernel-oracle.json)
binds all five source files, the exact proposal, raw logs and measured arrays.

The f64 oracle uses the real `FramePlan`, output plan bounds and eight jitter
phases for every admitted scale numerator 4–16. It inverts each actual internal
sample center into output coordinates and gathers at most nine unique samples.
Clipped samples are omitted, not duplicated at a border. The proposed Gaussian
uses `rho = .35 + .325 * max(r - 2, 0)` and normalization
`r² / (2π rho²)`. It accumulates premultiplied RGBA and scalar kernel mass;
clear samples contribute mass. Confidence is capped by
`h / (1 - h) * min(r², 4)`. Increasing internal resolution clamps old confidence
to the new cap before multiplying by the squared old/new internal width ratio.
Cuts discard prior color and confidence; decreasing resolution retains them.

Controls cover inverse coordinates, fractional/clipped viewports, invalid plans,
finite mass, constants including HDR 65504, fractional coverage, malformed clear,
confidence bounds and cut/scale transitions. The eight-phase capped recurrence
is checked independently as an affine contraction with its exact periodic limit.
The unchanged existing bilinear coordinate/gather oracle supplies a negative
control with fixed 0.9 history blending. That control omits production's depth,
clamp, reactive and LOD behavior, so it is not a recreation of the old GPU path.

## Measured claims and misses

The static chart uses 256 interior output centers, analytic bilinearly sampled
texels and 48 frames: one-texel vertical lines and two-/four-texel checkers.
Contrast is signed white-minus-black response averaged over the final eight
frames; activity is maximum absolute linear channel change. There is no tonemap,
material sampling, camera motion, image measurement or appearance judgment.
All 39 scale/pattern rows and six scale-transition arrays are retained.

| Control | Result |
| --- | --- |
| Interior eight-phase mass | At numerator 7: 7.927397200260–8.072923967236. The prose approximation 8 ± .07 fails; the proposal's explicit [7.9, 8.1] test bound passes. No kernel parameter was changed. |
| Numerator 8 contrast, lines / checker 2 / checker 4 | New: .382540921309 / .471988444565 / .712172674186; bilinear control: approximately 0 / .097256081931 / .430538203188. The quoted old checker-2 value .05 is not reproduced by this explicit control. |
| Numerator 8 activity | Lines improve from .027922853096 to .005049724284. Checker 4 increases from .020982526221 to .021597531668, so “no higher activity” is not universal. |
| 4→16 recovery | Third-frame contrast relative to this experiment's six-cycle numerator-16 mean is about .621 / .718 / .868; fourth-frame .886 / .900 / .946. A universal 75% / 90% recovery claim is not established. |
| 12→4 retention | First-frame contrast retains about 94.25% / 94.98% / 96.78% of the previous frame. |
| Largest omitted individual tap / kernel peak | 1.9563661e-5 on the tested interior output-pixel lattice and actual jitter phases, below 1e-4. This is not a bound on total omitted mass or arbitrary fractional-view alignments. An unrestricted alignment gives 1.02702565e-4 and exceeds 1e-4. |

The first test attempt retained a real failure: independently rounded HDR weighted
sum and mass made a strict equality-derived bound reject a valid constant. The
final test oracle admits a relative `32 * f64::EPSILON` arithmetic allowance for
at most nine products/additions. Reconstruction and quality tolerances were not
changed. That initial source was not separately frozen; its raw failure remains
historical evidence, not an exact final-source execution claim.

## Verification and remaining work

The final frozen run passed 16 CPU tests and left one existing GPU test ignored.
All-target renderer Clippy with warnings denied, formatting, conventions and diff
checks passed. Earlier test runs overlap this final set and are not added to it.
The source freeze is checked before and after each final command. The existing
GPU test was compiled but never executed. No new runtime dependency is added.

Canonical command, using pinned Rust 1.99 and the owned CPU target:

```sh
CARGO_TARGET_DIR=/Users/andreyyurlov/.codex/incant-gpu-timestamps-target \
PATH=/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:$PATH \
./tools/cargo test --locked --offline --profile ci -j2 -p incant_render \
  --lib upscaling:: -- --nocapture
```

Dynamic disocclusion and history ownership remain deliberately unimplemented
pending the corrected design. These scalar tests do not approve raster coverage,
texture LOD, trailing rejection, the complete renderer, GPU performance, visual
quality or any device/phase gate. Independent final packet review is pending.
