# 0053 native captures

Window-only captures of the granted editor (`incant_editor` `677f3955…5f4`, PID 7560) on the
retained v2 review fixture, taken with `screencapture -o -x -l <window>` at 2× on 2026-10-12.
They keep the captures' Display P3 profile.

- **Budget:** three original captures were taken, the packet's limit. 04 is a crop sheet.
- **Redaction:** in 01–03 the titlebar account avatar is filled with the titlebar colour
  (`#0b0c0f`). Each image differs from its raw capture in exactly 2,040 pixels, all inside the
  avatar's 50 × 50 px box.
- **Raw captures:** they stay local in the git-ignored `artifacts/0053/raw/`. Both sets of hashes
  are in `../../evidence/hashes.txt`, and every number below is in
  `../../evidence/measurements.json`.

| File | Window (pt) | State | What it shows |
| --- | --- | --- | --- |
| `01-authoring-1440x875-clip-0.png` | 1440 × 875 | Live | The rig selected in the authoring view. The Animation component's Clip row shows `0` in the same value box as Seconds and Speed. Across the value side of the row, x 1200–1440 pt, the Clip band is pixel-identical to the Seconds band (0 differing pixels). Against Speed, only the 0/1 glyph differs. From Looping through the Script component, the column matches 0052's capture 01, shifted up 41 pt, to within ±1 code value |
| `02-paused-tick10-1440x875-runtime-and-animation.png` | 1440 × 875 | Paused · tick 10 | Native Play paused with ⇧⌘P. The Runtime block (tick 10) sits above the unchanged Animation component, and the status line reads "Paused at tick 10." No focus ring shows: focus had left the Runtime field I put it in (see result.md, observation 1) |
| `03-paused-tick195-1000x650.png` | 1000 × 650 | Paused · tick 195 | The window resized to its minimum size while paused. The whole Animation component shows under the Runtime block. The Inspector column is 360 pt at both sizes, and the component's 720 × 440 px band is bit-identical to 02's (0 differing pixels, largest difference 0) |
| `04-crop-clip-before-after-1440-and-1000.png` | 720 × 1752 px | — | Inspector-column crops, top to bottom. First, 0052's capture 01 (before the fix: "This field’s type is not supported yet; raw value:" and the raw box). Then 0053's 01 (after, 1440 × 875). Last, 0053's 03 (after, 1000 × 650, with the Runtime block). All three are the same 2× pixels |

The agent pane is collapsed in all three captures. It was collapsed, and the window filled to
1512 × 875, while the editor was in the background before my first input. I had sent no input in
that interval. I left the pane as found and restored the window to 1440 × 875 (see result.md).
