# Actual native window captures (not the browser double)

Every PNG here except the two `alignment-*-crop-2x.png` files is an unmodified window-only capture
(`screencapture -o -x -l <window id>`) of the real editor window. They come from the root's
immutable integrated binary, launched with a fixture path only (no probe flags):

- `/tmp/incant-native-review-0044/incant_editor`, SHA-256
  `718ea75b9d792015fa2af4d939d4af2e2e412c01c706b046a910075cf70bd1b6`
- macOS 26.6 (25G72), arm64, built-in Retina display: device pixels are 2× points (a
  1440×874 pt window is 2880×1748 px).
- `wide-*`, `minimum-*`, `narrow-*` and `defect-splitter-*` use the granted fixture
  `artifacts/native-review-0044/project.json`, whose script moves X by 2 per tick.
- `picker-*` and `error-*` use `artifacts/native-review-0044/input-variant/` (two scenes, two
  cameras, one scene without a camera). `input-*`, `keyboard-*` and the other `defect-*` files
  use `input-variant-2/`, with corrected probes. Both fixtures were authored only through public
  RPC by `../../tools/native/author-input-variant.py`.

The `alignment-*-crop-2x.png` files are 2× nearest-neighbour enlargements of the top-left
720×110 px of `wide-01-stopped.png` and of an unfocused minimum-size capture. They were made by
`../../tools/native/pixels.swift`.

Neither fixture has a mesh. During Play the viewport shows the native surface's empty clear
colour. The cube in the stopped and error captures is the authoring viewport's diagnostic
cube, not game geometry. The browser-double screenshots are in `../normal`, `../minimum`,
`../narrow` and `../regression`. Hashes, measurements and the input steps are in
`../../evidence/native-review.json`.
