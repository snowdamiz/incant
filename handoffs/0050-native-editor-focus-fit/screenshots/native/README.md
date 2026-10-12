# 0050 native captures

Actual window captures of the granted rebuilt editor (`incant_editor`, SHA-256 `89f6b0d9…23fd`,
source `4589c2b`) in its WKWebView, on macOS 26.6, built-in display at 2×. Each is
window-only (`screencapture -o -x -l` on the owned window): 2880 px wide at 1440 pt and 2000 px
at 1000 pt. Five window captures and one crop: the packet's limit of six.

**Redaction.** In the five full-window images, the titlebar account avatar (28 pt, plus 2 pt
around it) is filled with the titlebar's own canvas colour, `#0b0c0f`. No other pixel was
changed: each image differs from its raw capture in exactly 2,040 pixels, all over the avatar
(`tools/native/imgtool.swift diff`). All images keep the captures' Display P3 profile. The
unredacted raw captures stay local in the git-ignored `artifacts/0050/`. Hashes of
both sets are in `../../evidence/native-screenshots.sha256`, and measurements are in
`../../evidence/native-review.json`.

**Pointer artefact.** In `play-01`, `play-02` and `composer-01`, the Undo button looks filled.
That is its hover state, not an app state: I had parked the pointer at (680, 20) pt, inside the
button. Later captures park it on empty titlebar.

| File | Shows |
| --- | --- |
| `play-01-1440x875-cmd-p-from-picker-focus-on-pause.png` | 1440 × 875, column 380. ⌘P was pressed with the picker focused on Main · Stationary Camera. Playing · Main, with the focus ring on Pause and both imported cubes rendering. The composer shows `gpt-6-astra` in full |
| `play-02-1440x875-no-camera-refusal-focus-on-play.png` | Same size. ⌘P was pressed with the picker focused on Spatial Without Camera. "Play failed", the notice with Dismiss, the picker shown again, and the focus ring on Play. In 0048 (`play-06`), this flow left no element focused |
| `composer-01-1440x875-column-280.png` | 1440 × 875, column 280 |
| `composer-02-1000x650-column-280.png` | 1000 × 650, column 280 |
| `composer-03-1000x650-column-380.png` | 1000 × 650, column 380 |
| `composer-04-crop-four-layouts-2x.png` | 2× crops of the composer and allowance line. Top to bottom: 1440 · 380 (from `play-01`), 1440 · 280, 1000 · 380, 1000 · 280. Each shows the whole `gpt-6-astra`, Ask before edits and Send inside the composer. Grey 8 px rules separate the crops |

In `composer-01` to `composer-03`, the right-column separator shows its focus ring because it
was last used from the keyboard (Home). In 0048, the composer read "gpt-6-astr…" at every
width (`../../../0048-native-editor-integration/screenshots/native/defect-model-name-clipped-crop.png`).
