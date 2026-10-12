# 0052 native captures

Window-only captures of the granted editor (`incant_editor` `4c1bf1f0…9a1`, PID 62948) on the v2
review fixture, taken with `screencapture -o -l <window>` at 2× on 2026-10-12. They are Display
P3, as captured. In 01–08 the titlebar account avatar is filled with the canvas colour
(`#0b0c0f`); each differs from its raw capture in exactly 2,040 pixels, all inside the avatar's
50 × 50 px box. Raw captures stay local in the git-ignored `artifacts/0052/raw/`; both sets of
hashes are in `../../evidence/hashes.txt`. Eight original captures were taken, the packet's
limit; 09 and 10 are crops of these files.

The viewport is the native surface's hole: x 492–2157 px and y 152–1219 px at 1440 × 875
(1666 × 1068 px), y 152–1217 px at 1440 × 874, and x 492–1277 px, y 152–769 px at 1000 × 650.
The measurements below are `imgtool blob` against the fixture's analytic pose
(`../../tools/native/predict.py`, or the authoring-preview projection); every number is in
`../../evidence/measurements.json`.

| File | Window (pt) | State | What it shows |
| --- | --- | --- | --- |
| `01-authoring-1440x875-rig-selected.png` | 1440 × 875 | Live | Authoring preview with the rig selected. The preview always draws from the renderer's fixed eye (6, 5, 9), so the triangle is seen obliquely; its three corners are within 0.4–2.0 px of that projection of the authored pose (clip 0 at 0 s, the rest pose). Inspector: Animation, Mesh Renderer, Script |
| `02-play-paused-tick39-1440x875.png` | 1440 × 875 | Paused · tick 39 | First native Play, paused by ⇧⌘P. The Stationary camera's face-on view: the joint has moved the triangle +0.633 units (move clip at 38/60 s). Centroid within 0.86 px of tick 39 (1.44 px for tick 38, 2.62 for 40). The Runtime block reads the entity's Translation 0 0 0, Rotation 0 0 0 1 and Scale 1 1 1, so the motion is the joint's, not the entity's |
| `03-paused-tick39-70s-later-1440x875.png` | 1440 × 875 | Paused · tick 39 | 70 s later, after time in the background and 5 s in front. The viewport is bit-identical to 02 (0 of 1,779,288 pixels differ); only the status message "Paused at tick 39." has expired |
| `04-stepped-tick49-1440x875-step-focused.png` | 1440 × 875 | Paused · tick 49 | After nine ⌥⌘P steps and one Space on the Tab-focused Step button (each +1 tick). The triangle moved 19.67 px = 0.170 units (10 ticks = 0.167); centroid within 0.35 px of tick 49. Step shows the focus ring |
| `05-resized-1000x650-paused-tick139.png` | 1000 × 650 | Paused · tick 139 | Resized to 1000 × 650 while Play ran, paused, then one ⌥⌘P step at this size. Rotate clip: 28.1° measured against 27.0° for tick 139 (±1° is this size's measurement limit), pivoting on the world origin. The agent pane is collapsed (its state below 720 pt height), so the whole Animation component shows under the Runtime block |
| `06-rotate-tick209-1440x874-after-resize.png` | 1440 × 874 | Paused · tick 209 | Resumed, resized back while Play ran, paused. Rotate clip at 1.4667 s: 131.5° measured against 132.0°; centroid within 0.70 px. macOS kept the window 1 pt short of 875 from here on. The ring around the agent pane is focus left there by my pointer click on its disclosure |
| `07-stopped-authored-pose-1440x874.png` | 1440 × 874 | Live | After ⌘P (Stop): the authoring preview again with the authored pose, corners within 0.25–1.87 px; no Runtime block. Status line: "Play stopped. Runtime changes were discarded." |
| `08-fresh-run-tick209-1440x874.png` | 1440 × 874 | Paused · tick 209 | A new run started with Space on the Tab-focused Play button, paused with Space at tick 166, then 43 ⌥⌘P steps to 209. The viewport is bit-identical to 06 from the first run (0 of 1,775,956 pixels differ). Resume shows the focus ring |
| `09-crop-viewport-timeline.png` | 400 × 2288 px | — | 400 × 320 px crops centred on each viewport, top to bottom: 01, 02, 04, 05, 06, 07, 08. Same 2× pixels; 05 is drawn at its smaller viewport's scale |
| `10-crop-animation-component-1440-and-1000.png` | 720 × 1040 px | — | The Animation component from 01 (1440 × 875) above the same component from 05 (1000 × 650). The Inspector column is 360 pt at both sizes. Clip shows the generic "This field’s type is not supported yet; raw value: 0" notice (see result.md) |
