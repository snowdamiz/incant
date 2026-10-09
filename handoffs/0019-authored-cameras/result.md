# 0019 authored cameras: rendered-pixel review result

## Status

Final review complete. Final scoped verdict: **pass, with no open pixel-evidence
gaps**. This covers authored perspective camera capture in the renderer, the
public CLI and the agent screenshot tool.

- I found no rendering defect in any of the 36 supplied PNGs.
- Every predictable geometric extent matched its analytic projection within
  about one pixel. The largest gap is 1.6 px at a bottom edge.
- All required byte-identity pairs hold.
- The ledger, the CLI configuration file and the on-disk PNGs agree on every hash.
- The three items left open after the second review are resolved.

This is a rendered-pixel review of one increment. It does not approve a phase
gate, production engine readiness, an editor camera UI, orthographic cameras,
camera rigs as a feature, or glTF camera import.

The report has three review rounds:
- The first review at `c5e02e6`.
- The follow-up at `ddc8645`.
- This final round, which builds on Astra's `fda9745` and `c21b643`.

Earlier rounds are preserved below as history.

## Model

- Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- The runner invoked this session in the handoff worktree. I cannot independently
  inspect the ACP transport from inside the session.
- No model substitution occurred.

## Changed paths

- `handoffs/0019-authored-cameras/result.md` (this file, committed).
- Ignored, not committed: `artifacts/review-0019-scripts/stats.py`,
  `artifacts/review-0019-scripts/predict.py`,
  `artifacts/review-0019-scripts/pose2-480-zoom.png`, plus one-off inline
  measurement commands.

I did not edit shaders, matrices, tests, GUI styling, project content, the
ledger, the documentation or the brief.

## Commands run

```sh
shasum -a 256 artifacts/camera-initial/*.png
python3 -I artifacts/review-0019-scripts/stats.py artifacts/camera-initial/*.png
python3 -I artifacts/review-0019-scripts/predict.py
# Final round, inline python3 -I with the same stdlib PNG decoder:
#  - every PNG hash checked against the ledger's png_sha256 entries
#  - CLI configuration compared with the ledger's public-CLI configuration
#  - the 33 earlier PNG hashes compared with the previous round
#  - skewed-parent quad corners projected with an independent orthonormalization
```

The scripts are stdlib-only PNG decoders. They report the non-background or red
bounding box, per-row red spans, sample colors and luminance second differences.
They also project the ±3-unit fixture quad through each camera for comparison.

Not run by me in any round: cargo, GPU, UI and Clippy tests and builds. All test
counts in this report come from Astra's briefs. I did not need a native
capture, so CUA was not used.

## Final round: reference and skewed-parent evidence

**Director feedback acknowledged.** The current brief integrates `ddc8645`. It
reports these results, which I did not run myself:
- Ledger and documentation updates.
- A post-fix suite of 139 Rust, 32 distinct GPU and 283 UI tests, plus the
  native release build and workspace Clippy.
- A live saved-account agent capture, which the brief says is not pixel review.

The brief asks me to do three things, and I did them first:
- review three extra images,
- verify ledger and configuration consistency,
- give a final verdict while preserving history.

**Provenance.** The runner supplied 36 PNGs. The three new ones are timestamped
2026-10-09 16:46 to 16:47. I compared the other 33 against my hashes from the
previous round, and every byte matches. That supports the brief's statement
that runtime appearance is unchanged since `8b82f43`.

**Ledger and configuration consistency:**
- The committed ledger `docs/spikes/evidence/authored-camera-capture-2026-10-09.json`
  lists 36 PNG hashes. Each one matches the file on disk, and no PNG on disk
  is missing from it.
- The CLI capture configuration in `artifacts/camera-cli-evidence.json` is
  identical to the ledger's public-CLI configuration.
- The seven CLI hashes in that file match the CLI PNGs.
- The ledger's visual-review field names Claude Opus 5.5 over ACP with reviews
  `c5e02e6` and `ddc8645`, and it marks this final follow-up as remaining. That
  is accurate until this report lands.

**camera-tight-clip-unclipped-reference vs camera-tight-clip-positive.** These
are byte-identical. The reference uses the same camera at 60° with range
0.1–100. Its quad spans x 82–238, y 42–198, against a prediction of
82.2–238.8 and 42.2–198.8. This closes round-two item 2. The 7.9–8.1 depth
window removes nothing from a quad at distance 8.

**camera-skewed-parent vs camera-skewed-flat-reference.** These are
byte-identical, with 25,158 red pixels. The rotated child under the scaled,
rolled parent produces a forward vector and up vector that are 6.4° from
perpendicular before orthonormalization.

The test builds its flat reference with the same `look_to_rh` the renderer
uses. That reference alone proves self-consistency, not the convention. So I
projected the quad with my own orthonormalization that keeps forward exact
and re-derives up:

| Corner | Measured | Predicted, forward kept | Alternative, up kept |
|---|---|---|---|
| left | (91, 146) | (90.4, 147.2) | (89.0, 171.3) |
| top | (171, 26) | (171.9, 25.2) | (171.4, 52.1) |
| right | (310, 99) | (310.7, 99.3) | (310.0, 122.6) |
| bottom | leaves frame at (220, 240) | (222.8, 242.6), just below the frame | (227.6, 276.2) |

The capture matches the forward-preserving convention within about one pixel.
It misses the up-preserving alternative by 23 to 27 px. The renderer therefore
aims exactly along the inherited forward axis and corrects roll. That is the
right behavior for a camera, because the subject stays centered when a parent
is sheared. It closes the optional fixture I suggested in round two.

The edge is a straight diagonal with no wobble, and the fill is uniform red
(243,31,31). The ordinary scaled-parent fixture now also asserts illumination,
so a blank frame can no longer satisfy its equality check.

**Round-two remaining items, all resolved:**
1. The ledger now carries all 36 PNG hashes.
2. The tight-clip byte identity is now directly visible in two saved PNGs.
3. The spike doc now states that mesh-less ancestors used as camera-rig frames
   no longer draw diagnostic cubes. It also says explicit ancestor meshes still
   render.

**The agent change is outside the pixel scope.** The agent change adds the
selected camera ID to the tool observation and to the screenshot result it
passes back. I read the diff. Pixels still go to the model only as an image
labelled as untrusted tool data. The live saved-account run is reported, not
observed by me, and is not pixel evidence.

**Final-round limitations.** I did not run cargo, GPU, UI, Clippy or build
commands. The test counts above come from the brief. No native capture was
needed, so CUA was not used. Antialiasing on quad edges remains a pre-existing,
out-of-scope observation.

## Round two: follow-up review (history, `ddc8645`)

Round-two verdict: **pass**. The four evidence gaps from the first review closed
with measured pixels, and the empty rig-parent defect was fixed. Three items
stayed open at that time. All three are resolved in the final round above.

### Round-two director feedback acknowledged

The round-two brief began with a priority follow-up. It integrates my `c5e02e6`
review and asks me to:

- review five new captures,
- confirm the empty-parent fix in the plain parent captures,
- use the new CLI pose record,
- preserve earlier findings and replace closed gaps with a follow-up verdict.

I applied all of these before anything else. The brief text was already
committed by Astra in `8b82f43` and was unchanged in the worktree.

### Round-two evidence provenance

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

### Round-two closed evidence gaps

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

### Findings preserved from the first review

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

### Round-two remaining items, now resolved

All of the items below were open at `ddc8645`. The final round closed each one,
including the optional fixture.

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

### Round-two open questions

- None blocking. The full verification rerun after the ancestor fix was
  reported as in progress, and I have not seen its result.

## Limitations

- I did not edit or run renderer code, tests or builds.
- No native UI capture was needed or taken.
- Provenance rests on timestamps, on the committed ledger's 36 PNG hashes and
  on the CLI configuration file. All three agree with the files I reviewed.
- Quad-edge aliasing is a pre-existing observation outside this packet.
- This review does not approve any phase gate or production engine readiness.
