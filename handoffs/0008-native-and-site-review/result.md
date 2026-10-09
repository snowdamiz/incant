# Result: native editor and landing-page visual review (handoff 0008)

**Status:** review complete for the landing page and the browser fixture. The native review
is partial. The traffic lights are still hidden by the capture indicator in every supplied
native image, so their alignment stays unconfirmed natively (see `native-requests.md`, D1).
No phase gate is approved by this work.

**Model:** Claude Opus 5.5 (`claude-opus-5-5`) through Claude Code, the configured ACP Claude.
No other model was used.

## Director feedback acknowledged

- **Capture permission is restored.** Browser review and screenshots resumed.
- **Native input and capture go through Astra's CUA channel only.** This was added in the
  follow-up packet. See the process disclosure below.
- **Raw native images stay ignored and uncommitted.** No account label appears in this file
  or in any committed image.
- **The design is not restarted.** The neutral charcoal editor, warm paper landing page,
  restrained accent and wisp mark are unchanged. No new purple was added.
- **The middle ground for the landing page is kept.** Each workflow step keeps its icon,
  title and one-sentence explanation. Real platform marks remain.
- **Merges belong to Astra.** These commits are not pushed or merged.

## Commits (all `Built-by: claude`)

| Commit | Change |
| --- | --- |
| `5b395f0` | Landing workflow layout across widths, platform-mark optical sizing, 320 px window bar, and one behavioural test |
| `eef71d3` | Inspector reference values end in an ellipsis instead of a clipped glyph |
| `07a0f59` | F2 rename opens with the current name selected, with a behavioural test |
| this commit | `result.md`, `native-requests.md`, capture scripts and browser screenshots |

## Changed paths

- `website/src/components/WorkflowSection.vue`
- `website/src/components/ModelingStage.vue`
- `website/src/icons/platforms.ts` (viewBox and an `optical` field only; path data unchanged)
- `website/tests/landing.spec.ts`
- `editor/ui/src/styles/app.css`
- `editor/ui/src/components/HierarchyPanel.tsx`
- `editor/ui/src/App.test.tsx`
- `handoffs/0008-native-and-site-review/` (this packet's evidence)

## Landing page: browser review

Rendered in Chrome 155 headless from the `PAGES_BASE_PATH=/incant/` production build. The
widths were 320, 390, 768, 1024, 1280 and 1440 px. 1024 was added because the tablet layout
runs to 1279 px. Images are in `screenshots/before/site/` and `screenshots/after/site/`, as
`workflow-<width>.png`, `create-<width>.png` and `platform-marks-1440@2x.png`.

Defects found and fixed:

1. **Dead band at 768 to 1279 px.** The six steps sat in a left rail about 670 px wide, and
   the right side of the section was empty. They now form a three-by-two grid. A hairline
   connects neighbours in each row. From 1280 px the original six-column wire is unchanged.
2. **Cramped explanations at 320 px.** The 64 px tiles left the text about 196 px. Some
   explanations wrapped to four lines with awkward breaks. Below 640 px the tiles are now
   48 px, and every explanation fits in three lines. The rail still meets the tile centres.
3. **Unbalanced platform marks.** Ink area ranged from 1482 to 2294 px² at 1440 px. The
   Android head sat high in its box, and the globe was the heaviest mark. Each viewBox is
   now cropped to its measured ink box. Each mark is centred in a fixed box at an optical
   height. Ink areas now range from 1543 to 1721 px².
4. **Truncated project name at 320 px.** The modeling window bar showed "Lantern …",
   because the decorative "Geometry graph" label kept its width. That label now hides below
   360 px.

Reviewed and left as they are:

- Copy density stays at a title plus one sentence per step. No paragraph was added.
- At 320 px some tool-strip captions wrap to two lines, such as "Rigs · IK · Timelines".
  They stay legible and aligned. Shortening public copy was out of scope.
- At 768 px the graph panel ellipsizes the Cylinder parameters. This is intentional
  truncation in an illustration.
- Every width has zero horizontal overflow.

**Bundle after the change:** JS is 133.28 kB raw and 48.65 kB gzip, against the 100 kB gzip
budget. CSS is 41.08 kB raw and 8.40 kB gzip. Before, these were 132.77/48.46 and 40.47/8.31.

## Connected editor: browser-fixture review

These images are the built UI over sample fixture data and evidence test doubles. They are not
native evidence. They are in `screenshots/{before,after}/browser-fixture/`, captured at 1440×900,
1280×800 and 1000×650 with a device pixel ratio of 2. A subset is committed, and
`tools/capture-editor.mjs` regenerates the full set.

| Check | Result |
| --- | --- |
| Panel boundaries | Square, flush 1 px joins. The viewport host has radii of 0 at all three sizes. |
| Hierarchy and inspector legibility | Good. The smallest text is 11 px, which is the documented minimum. |
| Focus states | The titlebar tool ring, the separator focus line and the selected tree row are all clearly visible. |
| Keyboard resize and drag | The hierarchy separator moves by drag (+80 px) and by arrow keys |
| Account dialog | Opens focused on Close. The sign-out question focuses "Keep signed in". |
| axe-core | No violations in the three scanned states |
| Console errors | None |

Defect fixed: long asset paths in the inspector were cut mid-glyph at the field border.
They now end in an ellipsis, and the full value stays in the tooltip.

Finding not changed, because it concerns state rather than appearance: in the connection-error
fixture, Problems shows "No problems" and the hierarchy shows a loading skeleton. Meanwhile
the banner says that nothing shown is current. A stale-data treatment for the dock and
hierarchy may be worth a later decision.

## Native observations (Astra's CUA captures)

Source: `artifacts/native-review/01` to `07`, root build `93c3d9b`, which predates this packet's
UI commits. The images are JPEG data at a device pixel ratio of 2, inspected and cropped in
headless Chrome. Measurements carry about ±0.5 pt of JPEG uncertainty. The crops stay in the
ignored `artifacts/native-review/crops/`.

| Check | Result | Image |
| --- | --- | --- |
| Logo clearance | The glyph starts at device x 177, which is 88.5 pt. That is 15 pt after the documented end of the green light, at 73.5 pt. It matches the browser double. | 07 |
| Logo vertical position | The glyph spans 13 to 27.5 pt, with its centre at about 20.25 pt. The documented light centre is 19.75 pt. | 07 |
| Traffic lights | **Not visible.** The CUA indicator covers device x 12 to 143 and y 12 to 50 in 01 to 05 and 07. | all |
| Inactive appearance | 01 was captured inactive. The identity and logo are dimmed to 55%, as designed. The dimmed logo is dark but still legible. | 01 |
| Fullscreen | The logo moves to the leading edge, 14 pt in, at the same vertical position. The white band and right strip look like a capture taken mid-transition (request D2). | 06 |
| After fullscreen | The layout is restored, and the titlebar matches the pre-fullscreen state | 07 |
| Panels and viewport | Square, flush joins. The neutral backdrop shows the white cube. In 01 the viewport is empty, apparently before the first frame. | 01, 07 |
| Native Cmd+Z and Shift+Cmd+Z | Astra observed project undo and redo through the menu path, from 1 of 1 to 0 of 1 applied and back | 02 |
| Native text undo | With a rename draft focused, Cmd+Z restored the text, and history stayed at 1 of 1. This is the intended split. | 03 |
| Account dialog | Opened on Close, not Sign out. It was opened by pointer, so no focus ring shows (request D3). | 04 |
| Divider keyboard resize | The arrow keys moved the hierarchy from 245 to 277 and back | 05 |

Native defects:

- **F2 rename appended to the old name** ("Entity 0Lantern"). Fixed in `07a0f59`. Native
  confirmation is requested in D4.
- **Transform fields arrive in alphabetical order** (Rotation, Scale, Translation). The native
  schema registry sends no `order`. This is a data-binding request for Astra (R1).

**Undo logic review:** I read `menu.rs` and the UI's history-request handler. With a text
field focused, the menu action calls the browser's own text undo and never falls back to
project history. Otherwise, project undo runs unless a dialog is open. I found no logic
defect, and Astra's native observations agree.

## Commands and results

| Command | Result |
| --- | --- |
| `npm ci` and `npm ci --prefix website` | Pass |
| `npm run build --workspace editor/ui` | Pass. JS is 299.65 kB raw and 92.62 kB gzip. CSS is 8.33 kB gzip. |
| `npm test --workspace editor/ui` | 223 of 223 pass. With the F2 fix removed, the new assertion fails (1 of 223). |
| `node tools/build_bridge.mjs --check` | Pass |
| `PAGES_BASE_PATH=/incant/ npm run build --prefix website` | Pass |
| `npm test --prefix website`, with `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH` set to installed Chrome | 24 of 24 pass. The bundled headless shell is not installed. The new width test fails on the old layout at 1024 px, where steps spanned about 72% of the width. |

The site preview for capture ran on 127.0.0.1:4185. The main checkout's server on 4176 was
not touched.

## Process disclosure

Before the follow-up packet banned native capture helpers, I took one passive window-only
capture of an already-running native instance that Astra had launched from the main checkout.
I used `screencapture -l` with a window id found by a Swift CGWindow listing. I sent no input
to it. That capture showed the same indicator over the lights. I have since deleted that
capture and its crops, and the helper scripts. After the follow-up packet, I used only Astra's
supplied images, inspected in headless Chrome. I did no HOME or CODEX_HOME override, sent no
native input, read no credentials, and took no account action.

## Limitations

- **Traffic-light alignment is not confirmed in native pixels.** The logo is confirmed. Request D1.
- **No native evidence for this packet's UI commits yet.** Astra's build predates them. Requests D3 and D4.
- **Minimum window size and settled fullscreen** have no native evidence (D2, D3).
- **Documents-folder project startup stalls**, per Astra. Documents access is not verified.
- **Platforms.** There is no Windows or Linux evidence. Windows caption buttons were seen only in a browser double.
- **Public copy.** The site describes the planned product as shipped, as the director requested.
  The workflow steps, the modeling illustration and the tool vignettes are illustrations, not
  implemented features. This review did not change that copy.

## Open questions

1. Should the dimmed logo in inactive windows stay at 55%, the same as the identity text, or
   stay at full colour like a macOS app icon? I kept 55% for consistency.
2. Does the director want a stale-data treatment for the hierarchy and dock while the editor
   is disconnected?
