# 0013 Headless playback: rendered-pixel review result

## Status

- Review complete. Verdict: **pass** for motion, frame coherence, visibility and
  preservation of the diagnostic appearance.
- No appearance regression was found, so no code, dependency, fixture or artifact was
  changed. No correctness finding needs to go to Astra.
- No phase gate is approved or claimed by this result. Production lighting, materials,
  camera controls and performance gates are not assessed.

## Model and transport

- Model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Transport: Claude Code (Claude Agent SDK) session started for this handoff on the
  director's Claude subscription, the same route as handoff 0011. The session cannot
  independently inspect the ACP bridge that launched it. No model substitution occurred.
- Director feedback: the current brief has no priority revisions or new director
  feedback beyond the packet text. No files from an interrupted attempt existed.
- No native UI capture, computer use, credentials or accounts were used.

## Inspected frames

All six PNGs were viewed as images at full size, then measured pixel by pixel with a
standard-library PNG decoder. The artifacts are git-ignored under `/artifacts/` and were
not modified.

| Sequence | Frame | Left instance centroid | Right instance centroid | Lit pixels |
| --- | --- | --- | --- | --- |
| velocity | 000000 | 320.3, 126.5 | 420.2, 145.9 | 4854 |
| velocity | 000030 | 292.8, 121.1 | 383.6, 138.8 | 4208 |
| velocity | 000060 | 267.5, 116.2 | 350.4, 132.4 | 3665 |
| scripted | 000000 | 320.3, 126.5 | 420.2, 145.9 | 4854 |
| scripted | 000030 | 306.2, 123.7 | 401.4, 142.2 | 4515 |
| scripted | 000060 | 292.8, 121.1 | 383.6, 138.8 | 4208 |

Paths:

- `artifacts/playback-review/velocity/frame-000000.png`, `frame-000030.png`, `frame-000060.png`
- `artifacts/playback-review/scripted/frame-000000.png`, `frame-000030.png`, `frame-000060.png`

Every frame is 640x360, 8-bit RGBA, and contains exactly two connected shapes.

## Findings

- **Motion direction is correct.** Both instances move screen-left and slightly up in
  every step of both sequences. They move together and keep their relative arrangement.
- **Net scripted motion is half the velocity motion.** Scripted frame 60 is pixel-identical
  to velocity frame 30. That is the expected result of one unit net versus two units over
  60 ticks. Scripted frame 30 lies at the midpoint between velocity frames 0 and 30,
  within half a pixel.
- **Both sequences start identically.** Frame 0 is pixel-identical across the two
  sequences, matching the claim of a shared starting geometry and position.
- **Shrinking and slowing on screen is expected perspective.** The fixed diagnostic camera
  sits at (6, 5, 9) looking at the origin. World negative X therefore recedes from the
  eye. The instances shrink by about a quarter over two units, and later steps cover
  fewer pixels. Shapes keep their proportions, so this is not a scaling bug.
- **Frames are coherent.** Each frame holds only the background and the fill color. There
  is no ghosting, smearing, partial overdraw, stale previous-frame pixels, torn edges or
  missing triangle. Both instances render in every frame, consistent with the shared
  indexed draw.
- **Visibility holds.** Both instances stay fully inside the frame with wide margins.
  Nothing is clipped by the frame edge or near plane.
- **Diagnostic appearance is preserved.** The background is RGB 20, 21, 25 and the fill is
  RGB 227, 227, 227. Contrast is 14.2:1, the same as the 0011 headless frames. Shading
  stays flat and neutral, with no material, texture or lighting change.

Known appearance limits carried over from handoff 0011, unchanged and not regressions:

- Edges are aliased because there is no multisampling.
- Framing is high and small, covering 1.6 to 2.1 percent of the frame here.
- Both instances share one flat tone, which gives little shape cue.

## Commands run

- Viewed all six PNGs with the image reader.
- Ran a standard-library Python script with `python3 -I` that decodes each PNG, counts
  colors, finds connected components, and reports bounding boxes, centroids and row
  hashes. Result: the table above. No third-party packages were used.
- Read the renderer camera definition and `docs/spikes/headless-playback.md` to interpret
  the perspective change.

## Tests not run

- No builds or tests were run. The brief marks them unnecessary for review-only work.
- The GPU playback test and hosted checks were not rerun here.

## Limitations

- Only ticks 0, 30 and 60 were supplied. Intermediate frames were not reviewed, so
  per-tick smoothness and the absence of single-frame glitches are not proven.
- The capture is a fixed 640x360 headless readback. Native viewport playback, HiDPI
  output and other GPUs were not reviewed.
- Pixel identity is supporting evidence. The verdict rests on the visual review of each
  frame.

## Changed paths

- `handoffs/0013-headless-playback/result.md`

## Open questions

- None blocking. The aliasing and framing items remain for future look-dev and camera
  handoffs.
