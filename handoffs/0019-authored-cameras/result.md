# 0019 authored cameras: rendered-pixel review result

## Status

Review complete. Scoped verdict: **pass with evidence gaps**. I found no concrete
rendering defect in the supplied PNGs. Every geometric extent I could predict
matched within one pixel. Several acceptance items are proven only weakly
because the frames are uniform; numerical fixtures for Astra are listed below.

This is a rendered-pixel review of one increment. It does not approve a phase
gate, production engine readiness, an editor camera UI, orthographic cameras or
glTF camera import.

## Model

- Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- The runner invoked this session in the handoff worktree. I cannot independently
  inspect the ACP transport from inside the session.
- No model substitution occurred.

## Packet revisions and director feedback

The current brief has no priority-revision section and no new director feedback
beyond the standing CLAUDE.md decisions. I reviewed the camera packet, not the
earlier lighting packet. No earlier files from an interrupted attempt existed:
`result.md` was absent and the worktree was clean at `3655c30`.

## Evidence provenance

- The runner had not yet copied `artifacts/camera-initial` into this worktree.
- I copied the 28 PNGs from the main checkout's `artifacts/camera-initial`.
- Their timestamps are 2026-10-09 16:35 to 16:36. Commit `3655c30` is 16:36:38 the
  same day. I cannot prove byte-level provenance from that commit beyond timing.
- All images are 8-bit RGBA with alpha 255 on sampled pixels.

## Changed paths

- `handoffs/0019-authored-cameras/result.md` (this file, committed).
- Ignored, not committed: `artifacts/camera-initial/` (copied PNGs),
  `artifacts/review-0019-scripts/stats.py`, `artifacts/review-0019-scripts/predict.py`,
  `artifacts/review-0019-scripts/pose2-480-zoom.png`.

No shaders, matrices, tests, GUI styling or project content were edited.

## Commands run

```sh
shasum -a 256 artifacts/camera-initial/*.png
python3 -I artifacts/review-0019-scripts/stats.py artifacts/camera-initial/*.png
python3 -I artifacts/review-0019-scripts/predict.py
```

The scripts are stdlib-only PNG decoders. `stats.py` reports the non-background
bounding box, sample colors and the maximum luminance second difference.
`predict.py` projects the fixture quad's corners through each authored camera
and compares them with measured extents. It also reports the interior-only
second difference, sampled at least three pixels from any edge.

Not run by me: cargo tests, GPU tests, Clippy and builds. The brief reports those
as Astra's results, with combined verification still in progress. I did not
need a native capture, so CUA was not used.

## Screenshots reviewed

All paths are under `artifacts/camera-initial/`.

| Image | Result |
|---|---|
| camera-fov-80.png | Red quad x 107–213, y 67–173. Predicted 106.6–214.4, 66.6–174.4. Square on a non-square frame. |
| camera-fov-30.png | Uniform red (243,31,31). Predicted quad x −8.1 to 329.1 overfills 321×241. Consistent but weak (gap 1). |
| near-clipped.png, far-clipped.png, camera-facing-away.png | Byte-identical uniform background (20,21,25). Expected: near 9 > distance 8, far 7 < 8, camera rotated 180°. |
| camera-clusters-0 (35°, eye 0,0,8) | 321: x 17–303, predicted 17.2–303.8. 480: x 79–400, predicted 79.4–400.6. Quad overfills vertically as predicted. |
| camera-clusters-1 (75°, eye 3,2,7) | 321: x 108–232 y 52–186, predicted 108.0–233.0, 52.1–187.6. 480: x 181–320 y 58–209, predicted 181.2–321.3, 58.3–210.2. |
| camera-clusters-2 (90°, eye −5,4,10) | 321: x 127–186 y 88–153, predicted 126.8–187.2, 87.9–154.5. 480: x 202–269 y 99–172, predicted 202.3–269.9, 98.5–173.1. |
| camera-clusters-*-oracle.png | All six byte-identical to the culling-free All-light path. |
| camera-flat-parent-reference.png, camera-inherited-pose.png | Byte-identical. Also identical to camera-clusters-2-321x241 because both reuse pose 2. |
| camera-transparent-front.png | Center (188,6,138), red-dominant. Red border x 77–243 matches the near plane's predicted 77.0–244.0. |
| camera-transparent-back.png | Center (137,6,188), blue-dominant. Blue border matches the near plane, now the blue quad. |
| cli-camera-preview / authored / moved | Viewpoint changes coherently (details below). |
| cli-camera-undo.png | Byte-identical to authored. |
| cli-camera-redo / reopened / source-free | Byte-identical to moved. |

**Cluster seams and illumination.** The interior luminance second difference is
at most 2.2 for poses 0 and 1 and 5.6 for pose 2, on an 8-bit scale. A 6× zoom
of pose 2 at 480×270 shows smooth falloff for all 7×7 point lights. The zoom
showed no straight-line discontinuity, cutoff, missing tile or dark cluster
cell. The hue gradient across the light grid is continuous and identical
between the pair sizes. Odd and even sizes keep the same vertical-FOV framing:
the quad width relative to frame height is 1.19 in both pose-0 sizes.

**CLI sphere.** The preview sphere spans x 210–429, about 220 px. The authored
camera is closer, at 274 px, with the highlight up-right. The moved camera
shifts the sphere left by 115 px with the same vertical extent, y 103–376. The
sphere widens from 274 to 282 px, the expected off-axis perspective stretch.
The specular highlight moves with the view direction. The 3968-triangle
silhouette shows no visible faceting at 640×480. I did not treat the titleless
frame as an editor design.

**Pre-existing observation, not a defect of this packet.** Quad edges are
aliased stair-steps, which is visible in the zoom. This predates camera
selection and is out of scope.

## Evidence gaps and fixtures requested from Astra

1. **The fov-30 frame cannot distinguish a correct projection from a full-frame bug.**
   A red clear color or full-screen red draw would produce the same image. The
   test's `red_pixels(narrow) > 2 * red_pixels(wide)` assertion would also pass.
   Add a fov 50 capture, or assert edges. At eye (0,0,8) and 321×241, a ±3 quad is
   predicted at x 63.6–257.4 and y 23.6–217.4.
2. **The clipping and facing-away cases only prove that nothing is drawn.** All
   three are byte-identical background frames. Add positive controls at fov 60:
   near 7.9 / far 8.1 should still show the full quad. A tilted quad crossing
   the near plane should show a straight cut at a predicted pixel row.
3. **Parent inheritance is exercised for translation only.** The fixture moves a
   parent by (2,0,0) and nothing else. Rotation inheritance, scale affecting
   inherited position and aim, and scale not changing clip units have no pixel
   evidence. Add a rotated, non-uniformly scaled parent compared with an
   equivalent flat pose. Add a scaled parent whose near-clip threshold stays at
   world distance 8.
4. **The cluster near/far ranges never intersect the quad.** The three ranges
   differ, but no pose clips visible geometry. Oracle equality covers cluster
   slicing, not visible near/far behavior at those poses. This is acceptable
   if gap 2 is addressed.
5. **The CLI camera poses and FOV are not recorded in the repository.** The
   script that produced the CLI images is absent, so I could only check
   consistency, not predicted framing. Please record the camera transform and
   Camera values beside the evidence.
6. **The agent `view_screenshot` camera test checks routing only.** It uses a mock
   host and does not count as pixel evidence, as the test itself states.

## Open questions

- The inheritance fixture's parent carries a zero-intensity DirectionalLight.
  Is that needed to keep a transform-only parent from rendering as a diagnostic
  cube? If so, camera rigs with empty parent entities would show cubes in
  captures. That is worth an explicit test or a documented limitation.
- Full combined GPU, Clippy and build verification was reported as in progress.
  I did not observe its result.

## Limitations

- I did not edit or run renderer code, tests or builds.
- No native UI capture was needed or taken.
- Screenshot provenance rests on timestamps, as noted above.
- This review does not approve any phase gate.
