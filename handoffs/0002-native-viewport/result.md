# Handoff 0002 result: native viewport review (macOS)

## Status

**Review required.** I have not approved any phase gate, and none is implied.

- I reviewed the real Tauri/wgpu app from this worktree on macOS. I did not use a
  fixture page or a browser for this native evidence.
- The native composition is correct. Rendered geometry shows through the viewport,
  and the chrome is opaque at exact token values. The rounded GPU mask meets the
  island with no seam, and the surface follows every layout change I tried.
- I fixed eight visual and usability issues in `editor/ui` and verified each one in the
  native app. Before and after evidence is listed below.
- The traffic-light alignment the director flagged is fixed. I measured the correct
  value natively, and Astra applied it in `46e3373`. The final build is verified.
- Rust and integration requests are in `native-requests.md`, items R1 to R7. R1 and
  the entity-kind correction are already done by Astra.
- **A 1920×1080 capture was not possible on this Mac.** See Limitations.
- **No Windows or Linux evidence exists.** Everything here is macOS only.

## Model

- Model: Claude Opus 5.5, model ID `claude-opus-5-5`, in Claude Code, working in this
  worktree on branch `handoff/0002-native-viewport`.
- From inside the session I cannot tell whether it was launched through ACP or as the
  manual fallback.

## Director feedback acknowledged

The director asked me to keep improving the UI and fixing small issues while Astra
works on logic, with the traffic-light position as one example. I read
`reference-director-2026-10-08.png`. It shows the lights sitting high, with their
centres above the logo and the project name. I then measured the current worktree
build rather than the director's older main-checkout instance. Even with the newer
14/14 setting, the lights were still 8 pt too high. Details follow.

## Platform

| Item | Value |
| --- | --- |
| OS | macOS 26.6 (25G72) |
| Hardware | Apple M5 Pro, wgpu adapter "Apple M5 Pro" (from the in-app console) |
| Display | Built-in Liquid Retina XDR, 3024×1964 pixels, 1512×982 pt, device pixel ratio 2 |
| App | `artifacts/Incant.app` from `python3 tools/editor-dev.py` (debug build, `custom-protocol`) |
| Project | `artifacts/native.incant.json`, journal-backed, all edits through the command bus |

All captures are window-only (`screencapture -o -l <window id>`) of this worktree's
Incant window. Each window ID was resolved from a PID whose executable path is inside
this worktree. No other app or window appears in any image.

## Traffic lights: measured and fixed

tao 0.37.1 does not treat `y` as a distance from the top. It adds `y` to the
titlebar container's height, and each button keeps its own offset inside that
container. I built a temporary variant that read x and y from environment
variables. That source change was reverted before anything was committed. I
measured the light centres at several values:

| Configured y | Light centre from top | Logo centre |
| --- | --- | --- |
| 14 (previous) | 11.75 pt | 19.75 pt |
| 18 | 15.75 pt | 19.75 pt |
| 20 | 17.75 pt | 19.75 pt |
| **22 (final)** | **19.75 pt** | **19.75 pt** |

Astra committed x=14 and y=22 as `46e3373`. In the rebuilt app the centres are at
19.75 pt, level with the logo to the pixel. They stay there after a resize and after
leaving fullscreen. I also tightened the gap from the green light to the logo, from
16.5 pt to 12.5 pt.

- Before: `screenshots/titlebar/current-build-y14-focused-crop.png` and `screenshots/titlebar/exp-y14-crop.png`
- After: `screenshots/titlebar/final-build-y22-focused-crop.png` and `screenshots/titlebar/exp-y22-crop.png`

## UI fixes in this handoff

Each fix was found in the native app and verified there after a rebuild.

1. **Skip link hidden by the traffic lights.** It appeared at the top left, under the
   native lights, and was unreadable. It is now centred in the titlebar and vertically
   centred in the 40 px bar.
   Before: `focus/before-skip-link-crop.png`. After: `focus/final-skip-link-crop.png`.
2. **Skip link href.** The href now matches the label: `#hierarchy-title`, or
   `#viewport-title` when the hierarchy is hidden. A test covers both cases.
3. **Disabled history buttons looked enabled.** A disabled ghost button picked up a
   border, so the unavailable action looked more prominent than the available one.
   Disabled ghost buttons are now borderless and dimmed.
   Before: `after-07-undone-1280x800.png`. After: `final-04-history-renamed-1280x800.png`.
4. **Times showed UTC.** The console and history showed 00:47 while local time was
   20:47. They now show local 24-hour time. The full instant stays in `<time dateTime>`.
5. **"Transform Transform".** A component's type is no longer repeated when it equals
   the title.
6. **Raw field keys.** Untitled schema fields are humanised, so "translation" reads
   "Translation". A test covers this.
7. **Scene icon looked like the cube.** The scene used an isometric box, while the
   rendered cube used a folder. The scene icon is now a stacked-layers glyph. Astra
   separately changed Transform-only entities from "Group" to the generic "Entity"
   (`d3f01a4`).
8. **Scene copy.** A selected scene now says "Scenes hold entities, not components"
   instead of "This entity has no components".

Also added:

- **macOS app icon.** The bundle showed the generic placeholder icon. I added
  `editor/ui/brand/icon-macos.svg` and `scripts/render-macos-icon.mjs`
  (`npm run icon:macos`), which produce `editor/ui/brand/Incant.icns` on the macOS
  icon grid. I verified it in an ignored experiment bundle. Wiring it into
  `tools/editor-dev.py` is request R2 for Astra.
  Before: `icon/before-running-app-icon.png`. After: `icon/after-running-app-icon.png`.
- **Docs.** `TITLEBAR.md` records the measured traffic-light semantics.
  `NATIVE_VIEWPORT.md` now summarises the native results.

## Native composition evidence

These checks come from the real native window. Pixel values are raw stored values
read from the captures.

| Check | Result | Evidence |
| --- | --- | --- |
| Geometry through the viewport | Cube renders inside the viewport hole | `final-05-entity-selected-1280x800.png` |
| Chrome opaque | Island #131418, canvas #0b0c0f, titlebar #0b0c0f, all exactly the tokens | same |
| Viewport clear | #303030, identical to the headless output | same and `headless/` |
| Corner mask | Rounded bottom corners meet the 1 px island border with no square pixels or gap | zoomed crops, verified while capturing |
| Bounds when panels hide | Surface fills the widened viewport, edges clean on both sides | `after-09-…`, `after-10-all-panels-hidden-1280x800.png` |
| Bounds when the window resizes | Correct at 1280×800 and 1512×873 pt | `final-07-largest-1512x873.png` |
| Modal over the surface | The shortcuts dialog and scrim cover the viewport | `after-08-shortcuts-dialog-1280x800.png` |
| Hierarchy and inspector selection | Scene and entity selection update the inspector | `native-03-…`, `final-05-…` |
| Console | Native adapter message listed, filters work | `final-03-console-1280x800.png` |
| Rename, undo, history | Rename went through the bus and undo reverted it. History survives a restart through the journal | `after-06-…`, `after-07-…`, `final-04-…` |
| Titlebar drag | Moved the window on both axes, clamped only at screen edges | driver log in this session |
| Double-click zoom | Zoomed to the visible frame, and a second double-click restored the exact prior frame | driver log |
| Focus state | Unfocused title dims from #ececf1 to #87878b, and the lights turn grey | `final-06-unfocused-1280x800.png` |
| Fullscreen | Inset drops to 0 and the logo moves to the edge. After exit the inset is 78 again and the lights re-centre | `after-12-fullscreen-1512x982.png`, `after-13-…` |
| Keyboard focus | Titlebar focus rings are clear and distinct from hover | `focus/titlebar-focus-ring-1280x800.png` |

The image from `incant_headless screenshot` is reviewed separately here. At
1280×720 and 1920×1080 it shows the same cube on #303030. The edges are aliased,
which is request R5. Evidence: `screenshots/headless/`.

**Browser transparency evidence is separate and is not repeated here.** The
handoff 0001 Chrome test with an evidence test double measured alpha 0 in the
viewport hole. It remains browser-only evidence. Everything in the table above
comes from the native app.

## Commands and results

| Command | Result |
| --- | --- |
| `npm ci` | passed, 0 vulnerabilities |
| `npm run build --workspace editor/ui` | passed |
| `npm run typecheck --workspace editor/ui` | passed |
| `npm test --workspace editor/ui` | 160 of 160 passed in 6 files: the 158 existing tests plus 2 new |
| `tools/cargo run -p incant_headless -- init artifacts/native.incant.json --name "Native Surface Spike"` | created the project |
| `python3 tools/editor-dev.py` | built and packaged `artifacts/Incant.app`, rerun after each fix |
| `tools/cargo run -p incant_headless -- screenshot artifacts/native.incant.json <out> --width W --height H` | wrote 1280×720 and 1920×1080 images |
| `npm run icon:macos --workspace editor/ui` | wrote `brand/Incant.icns` |
| `tools/cargo test --workspace` | not rerun by me. Astra reports 29 passed. I changed no Rust source in any commit |

The window driver and pixel tools were small Swift helpers in `/tmp/incant0002/`,
outside the repository. Every action was scoped to one PID, and every click was
checked to fall inside that PID's window frame.

## Limitations

- **No 1920×1080 window.** The display is 1512×982 pt, and macOS clamped a requested
  1920×1080 window to the visible 1512×873 pt. The largest capture is
  `final-07-largest-1512x873.png`, which is 3024×1746 device pixels. Fullscreen gives
  1512×949 pt below the notch. I did not change the user's display scaling. A true
  1920×1080-point capture needs an external display or a scaled resolution set by
  the director.
- **macOS only.** There is no Windows or Linux evidence, including the custom caption
  buttons.
- **One DPI.** Only a device pixel ratio of 2 was available. Moving across displays
  with different scale factors was not tested.
- **Synthetic input.** Drag, double-click and keys came from posted CGEvents, not a
  physical mouse. Physical trackpad drag was not tested.
- **Director instance exited.** The director's main-checkout instance (PID 88101) and
  my first test instance both exited between 20:42 and 20:45. Neither left a crash
  report. I killed only the experiment bundle by its path. Astra's commit landed at
  20:44. I did not relaunch the director's instance. This is recorded in R7.
- **One unexplained window jump.** A single (−21, −6) pt jump happened on the first
  titlebar click and did not reproduce. This is recorded in R7.

## Remaining issues and open questions

- **Field order.** Transform reads Rotation, Scale, Translation alphabetically
  until the bridge passes `order` (R3).
- **Viewport background.** The neutral #303030 clear is the loudest surface in the
  window. I recommend #1b1d22 (R4). This is a taste call for the director.
- **Inspector columns.** The four rotation fields are narrower than the
  three-field rows, so the Y and Z columns don't line up. It is legible. A shared
  column grid can come with editable fields.
- **Hierarchy count.** The count badge includes the scene: two items for one entity.
- **Double-click preference.** Honouring the macOS double-click setting needs host
  support (R6). Behaviour is correct on this Mac because the setting is the default.

## Changed paths

- `editor/ui/src/App.tsx`: skip-link href
- `editor/ui/src/App.test.tsx`: two new tests
- `editor/ui/src/styles/app.css`: skip link, titlebar logo gap, disabled ghost buttons
- `editor/ui/src/components/BottomDock.tsx`: local time
- `editor/ui/src/components/inspector/InspectorPanel.tsx`: type de-duplication, scene copy
- `editor/ui/src/components/inspector/FieldView.tsx`: humanised labels
- `editor/ui/src/icons/Icon.tsx`: scene glyph
- `editor/ui/brand/icon-macos.svg`, `editor/ui/brand/Incant.icns`, `editor/ui/scripts/render-macos-icon.mjs`, `editor/ui/package.json`: macOS icon
- `editor/ui/TITLEBAR.md`, `editor/ui/NATIVE_VIEWPORT.md`: native findings
- `handoffs/0002-native-viewport/`: `result.md`, `native-requests.md`, `screenshots/`, and the director's reference image as received
