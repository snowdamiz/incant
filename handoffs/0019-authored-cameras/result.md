# 0019 authored cameras: rendered-pixel review result

## Status

Follow-up review complete. Scoped verdict: **pass**. The four evidence gaps from
the first review are closed by measured pixels. The empty rig-parent defect is
fixed in the captures I reviewed. I found no rendering defect in any supplied
PNG. Every predictable geometric extent matched its analytic projection within
about one pixel.

Three items stay open and are listed under "Remaining items". None of them
changes the pixel verdict.

This is a rendered-pixel review of one increment. It does not approve a phase
gate, production engine readiness, an editor camera UI, orthographic cameras,
camera rigs as a feature, or glTF camera import.

## Model

- Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- The runner invoked this session in the handoff worktree. I cannot independently
  inspect the ACP transport from inside the session.
- No model substitution occurred.

## Director feedback acknowledged

The current brief begins with a priority follow-up. It integrates my `c5e02e6`
review and asks me to:

- review five new captures,
- confirm the empty-parent fix in the plain parent captures,
- use the new CLI pose record,
- preserve earlier findings and replace closed gaps with a follow-up verdict.

I applied all of these before anything else. The brief text was already
committed by Astra in `8b82f43` and was unchanged in the worktree.

## Evidence provenance

- The runner copied 33 PNGs into this worktree's ignored
  `artifacts/camera-initial/`.
- The renderer captures are timestamped 2026-10-09 16:42, matching Astra's
  `8b82f43` at 16:42:52. The seven CLI captures are unchanged from 16:35, as the
  brief states.
- `artifacts/camera-cli-evidence.json` now records the CLI configuration:
  dimensions, authored and moved transforms, projection and default preview
  camera. Its seven SHA-256 values match my hashes of the CLI PNGs.
- The committed ledger `docs/spikes/evidence/authored-camera-capture-2026-10-09.json`
  does not yet list hashes for the five new PNGs. The brief says the final
  ledger will record them. See "Remaining items".

## Changed paths

- `handoffs/0019-authored-cameras/result.md` (this file, committed).
- Ignored, not committed: `artifacts/review-0019-scripts/stats.py`,
  `artifacts/review-0019-scripts/predict.py`,
  `artifacts/review-0019-scripts/pose2-480-zoom.png`, plus one-off inline
  measurement commands.

I did not edit shaders, matrices, tests, GUI styling, project content or the brief.

## Commands run

```sh
shasum -a 256 artifacts/camera-initial/*.png
python3 -I artifacts/review-0019-scripts/stats.py artifacts/camera-initial/*.png
python3 -I artifacts/review-0019-scripts/predict.py
# plus inline python3 -I measurements of the new captures, using the same
# stdlib PNG decoder, and a grep of the committed ledger for each PNG hash
```

The scripts are stdlib-only PNG decoders. They report the non-background or red
bounding box, per-row red spans, sample colors and luminance second differences.
They also project the ±3-unit fixture quad through each camera for comparison.

Not run by me: cargo tests, GPU tests, Clippy and builds. The brief reports the
first combined run passing 138 Rust and 31 GPU tests. A rerun after the
ancestor fix is in progress, and I did not observe it. I did not need a native
capture, so CUA was not used.

## Follow-up review: closed evidence gaps

All new images are 321×241. Eye (0,0,8) looks down −Z at the ±3 quad on z=0
unless a row says otherwise. Each table row closes one earlier gap.

| Earlier gap | New capture | Measured | Predicted | Verdict |
|---|---|---|---|---|
| 1. The fov-30 frame could not show quad edges | camera-fov-50-visible-edges | red x 64–256, y 24–216 | x 63.6–257.4, y 23.6–217.4 | Closed. All four edges are inside the frame. |
| 2. The clip cases proved only an empty frame | camera-tight-clip-positive, near 7.9 / far 8.1 | red x 82–238, y 42–198 | unclipped 60° quad at x 82.2–238.8, y 42.2–198.8 | Closed. The full quad survives a 0.2-unit depth window. |
| 2. (continued) | camera-tilted-partial-clip, quad tilted 30° about X, clip 7.5–8.5 | red rows 96–141; widths 167, 157, 147 at top, middle, bottom | cut rows 96.40 and 141.76; widths 167.0, 156.5, 147.4 | Closed. Both cuts are straight horizontal edges with contiguous rows at the predicted depths. |
| 3. Parent inheritance covered translation only | camera-scaled-roll-parent vs camera-scaled-roll-flat | byte-identical; corners at (61,121), (273,65), (195,200), with the top corner off-frame | inherited eye (0.268, −1, 8); corners (60.6,123.1), (274.4,65.8), (196.1,201.3), (138.8,−12.5) | Closed. Rotation and non-uniform scale move the eye and roll the view. Clip units are not scaled, or the 7.9–8.1 window would show nothing. |
| 5. The CLI poses were not recorded | artifacts/camera-cli-evidence.json | authored eye (0,0,8), moved eye (2,0,8), 60°, 0.1–100, 640×480; preview eye (6,5,9), 50° | see below | Closed. |

**CLI framing now checks numerically.** At 640×480 and 60°, the focal length is
415.7 px. A 2-unit move at distance 8 predicts a 103.9 px leftward shift of the
sphere's center. I measured a bounding-box center shift of 115 px. That box
includes the 8 px off-axis widening from 274 to 282 px. The projected center of
an off-axis sphere also lands farther out than its geometric center. Both
effects make the measured shift larger than the simple prediction, and the
size is plausible. The vertical extent is unchanged at y 103–376, as a pure
+X move requires. The preview's smaller sphere is consistent with its farther
eye, at distance 11.9 versus 8.

**Rig parent fix.** These captures confirm the fix:
- camera-inherited-pose now uses a plain Transform-only parent with no light
  workaround. It is byte-identical to camera-flat-parent-reference.
- The scaled-roll parent is also Transform-only, and its capture equals the
  parentless flat capture.

A visible diagnostic cube at the parent would break either equality. I read
the scene change and the new CPU test. The new rule hides fallback cubes only
for mesh-less ancestors of a Camera. Explicit meshes on ancestors and unrelated
diagnostic entities still count. Document validation already rejects missing
parents and cycles, so the new ancestor walk cannot index a missing entity.

## Findings preserved from the first review

All of these still hold. Hashes are unchanged for every re-captured image.

| Image | Result |
|---|---|
| camera-fov-80 | Red quad x 107–213, y 67–173. Predicted 106.6–214.4, 66.6–174.4. Square on a non-square frame. |
| camera-fov-30 | Uniform red. The predicted quad overfills the frame. Now backed by the fov-50 frame. |
| near-clipped, far-clipped, camera-facing-away | Byte-identical background frames, as intended. Now backed by positive clip controls. |
| camera-clusters-0, 35°, eye (0,0,8) | Within one pixel of prediction at both sizes. The quad overfills vertically, as predicted. |
| camera-clusters-1, 75°, eye (3,2,7) | Within 1.6 px of prediction at both sizes, largest at the bottom edge. |
| camera-clusters-2, 90°, eye (−5,4,10) | Within 1.5 px of prediction at both sizes, largest at the bottom edge. |
| camera-clusters-*-oracle | All six byte-identical to the culling-free All-light path. |
| camera-transparent-front / back | Centers (188,6,138) red-dominant and (137,6,188) blue-dominant. The border matches the near plane's predicted x 77.0–244.0. |
| cli-camera-undo | Byte-identical to authored. |
| cli-camera-redo / reopened / source-free | Byte-identical to moved. |

**Cluster seams and illumination.** The interior luminance second difference is
at most 2.2 for poses 0 and 1 and 5.6 for pose 2, on an 8-bit scale. A 6× zoom
of pose 2 shows smooth falloff for all 7×7 point lights. It shows no seam,
cutoff, missing tile or dark cluster cell. The hue gradient is continuous and
identical between the pair sizes. The rolled scaled-parent capture shows the
same gradient rotated clockwise, as a +30° camera roll requires.

**Not a defect of this packet.** Quad edges are aliased stair-steps. This
predates camera selection and is out of scope. The titleless CLI frame is not
treated as an editor design.

## Remaining items

1. **The new ledger hashes are missing.** The committed ledger lists no SHA-256
   for camera-fov-50-visible-edges, camera-tight-clip-positive,
   camera-tilted-partial-clip, camera-scaled-roll-parent or
   camera-scaled-roll-flat. The brief says the final ledger will add them. Its
   `visual_review` field still reads "pending". My hash prefixes are
   `eca524c77ca7d2fc`, `29a364e13f7f3f4a`, `65813ffd2ae07f3a` and
   `f6f806044e46cb8f`. The last one is shared by the two scaled-roll images.
2. **One byte-identity claim is test-asserted only.** The brief says the
   tight-clip capture equals the original unclipped 60° view. That reference
   frame is not saved as a PNG, so I could not compare the bytes myself. Its
   geometry matches the unclipped prediction to the pixel, so I accept it on the
   test's assertion.
3. **Documentation is behind the new rig-frame rule.** The spike doc still
   says only that camera-only entities lose their cubes. It should also say
   that mesh-less ancestors of a camera do. A user who wanted a placeholder cube
   on a parent of a camera will no longer see it.

**Optional low-priority fixture.** A rotated child under a non-uniformly scaled
parent creates a sheared basis. The code keeps forward exact and
re-orthogonalizes up through `look_to_rh`. That convention is reasonable, but
no capture shows it. One flat-vs-inherited pair would pin it down.

Earlier gap 4 is resolved by the clip controls. The cluster near/far ranges
never clip the quad, but visible clipping is now covered elsewhere. Earlier
gap 6 is unchanged and informational. The agent `view_screenshot` camera test
is a routing-only mock and does not count as pixel evidence.

## Open questions

- None blocking. The full verification rerun after the ancestor fix was
  reported as in progress, and I have not seen its result.

## Limitations

- I did not edit or run renderer code, tests or builds.
- No native UI capture was needed or taken.
- Provenance rests on timestamps and on hashes recorded in the CLI evidence
  file. The five new renderer PNGs are not yet in the committed ledger.
- This review does not approve any phase gate.
