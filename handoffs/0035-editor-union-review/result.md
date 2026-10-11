# 0035 release-union review: result

## Status

The browser review of the code union a20d258 is complete. **No source change.** No phase
gate is approved or claimed, and nothing here is native or device evidence.

- **The plain editor survives the union intact.**
  - 94 of the 96 0033 states have identical metrics on the union build. The other 2 are
    Broken Camera, whose notice now lists the third projection (one more word).
  - 27 of the 29 committed plain screenshots are pixel-identical. The rest are the status-dot
    halo that cdf0fab removed, plus rendering noise of 7–22 px: two captures of the same
    union build differ by the same pixels.
- **The director's five requirements hold** in every union state reviewed. There is one
  exception in requirement 2's focus handling: choosing **Use** on another account drops
  keyboard focus (finding 1). That fault predates the union.
- **Camera.** All three projections of the real schema render truthfully, including
  malformed and unknown values: perspective (default and explicit), orthographic and
  pixel-perfect.
- **Sprites and atlases.** Both reach the editor only through the generic schema path, and
  the rendering has visible gaps (findings 3–8):
  - rows are alphabetical;
  - every sprite's Region shows "not supported yet";
  - the RGBA tint and the nine-slice borders are labelled and named X/Y/Z/W;
  - atlases look like plain textures.
- **The union's importer accepts `.atlas.json`, but the import form refuses it** (finding 2).
- **Why no source change.** No finding was introduced by the merge. Each fix belongs to one of:
  - Rust schema annotations, the bridge, or import and focus behaviour, which Astra owns;
  - a scoped sprite-presentation follow-up once those land.

  The packet allows edits only for concrete regressions, so each finding comes with a
  reproduction and a proposed fix instead.
- **Native.** No native app was launched, focused or captured. The a20d258 binary is
  unobserved, and the native checklist below remains open.

## Model and effort

- Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Thinking effort: Max. The session reports reasoning effort "max" to the model. The runner
  selects and verifies the model and effort through ACP. I cannot inspect the ACP transport
  from inside the session.
- No model or effort substitution occurred.

## Packet and boundaries

- I read `brief.md` (the only packet section, at 3a6aacb), CLAUDE.md and the union note
  `docs/spikes/editor-release-union.md`. I added no calendar target and approved no gate.
- The plain design and Astra's fixes are untouched, because nothing in `editor/ui` changed:
  menu and focus, live status, diagnostic reveal, clipboard failure, bounded lists and paging.
- No Rust, Cargo, CI, schema, bridge, credential storage, PLAN or `docs/` edit.
  - No dependency was added. `npm ci --ignore-scripts` installed the pinned lockfile for
    `editor/ui` only.
  - No Cargo command ran.
- **Native safety.** I did not launch, focus, type into, close or capture any native app.
  - I read no account data and caused no system-wide input or focus change.
  - All rendering ran in headless Chrome against `vite preview` on 127.0.0.1, at the first
    free port from 4233; every server was closed.

## Source identity

| Item | Value |
| --- | --- |
| Branch head | `3a6aacb` (handoff/0035-editor-union-review) |
| Code union | `a20d258ba75fd00b2c8bea91cf71de8bead78370`. These trees are identical at the head and at a20d258: `editor/ui` `5b13ad49…`, `editor/bridge` `4dca2112…`, `schemas` `0c174377…`, `crates` `76662041…`, `editor/app` `b9add18c…` |
| Union delta over the plain editor (ce06c31), editor side | <ul><li>`bridge/fixture.ts`: the Camera schema gains `pixel_perfect`.</li><li>`CameraFields.test.tsx` and `PixelCameraFields.test.tsx` now use the real schema.</li><li>A comment in `presentation.ts`.</li><li>Schemas: Camera adds `pixel_perfect`; SpriteRenderer and SpriteAtlas are new.</li></ul> |
| What the native host sends | The whole `schema_registry()`: 20 schemas, including Project, SpriteAtlas and SpriteRenderer. SpriteRenderer has no shared `order` annotation, and its tint has no colour widget annotation. `snapshotFromEngine` turns `region`, an untagged `anyOf` of a rectangle or `{frame}`, into `unsupported`. No sprite-specific UI code exists. |
| Browser | Headless Chrome 155.0.8059.40 at device scale 2, on an Apple M5 Pro with 48 GB and macOS 26.6 (the same Chrome as 0033) |

## Method

1. **Regression sweep.**
   - I ran 0033's unchanged `capture.mjs` and evidence double on the union build: 96 captures
     of 61 states.
   - Its metrics were compared with 0033's committed per-state "after" values.
   - Its PNGs were compared pixel by pixel with the 29 committed plain screenshots.
   - Two states were captured again to measure rendering noise.
2. **Union review**, with the new tools in `tools/`:
   - `union-double.mjs` keeps the 0033 project and adds the union only:
     - the whole native registry, with a tripwire against the Rust source;
     - two pixel-perfect cameras;
     - ten SpriteRenderer entities: atlas frames, a raw rectangle on an atlas, nine-slice,
       flips, required fields only, malformed values, a missing texture, a linear texture, a
       missing frame and a long name;
     - two cooked-atlas texture assets and a sprite sheet.
     The real compiled `snapshotFromEngine` converts it all.
     - Diagnostics are **synthetic**, because the native bridge sends no per-entity
       validation; their text is copied verbatim from the Rust validators.
   - `capture.mjs` captures 60 states, 102 captures in all: normal 1440×874, minimum 1000×650,
     the narrow 280 px Inspector and wide 1920×1080. It covers every account state, host window
     chrome, both key notations, and probes that record focus, reveal, paging and announcements.
   - `findings.mjs` reproduces the findings. `summarize.mjs`, `compare-sweep.mjs`,
     `pixel-diff.mjs` and `curate.mjs` produce the committed evidence.

## The union versus the plain design

| Check | Result |
| --- | --- |
| Metrics of the 96 0033 states (`evidence/sweep-compare.json`) | 94 identical. Broken Camera (normal, minimum) has +1 Inspector word from the new message "Expected perspective, orthographic or pixel_perfect" |
| Pixels of the 29 committed plain screenshots (`evidence/pixel-plain.json`) | 27 identical. `dock-paged-problems-minimum`: 420 px at the status dot (the halo cdf0fab removed), plus 22 px. `no-selection-normal`: 7 px |
| Noise check (`evidence/pixel-repeat.json`) | Two captures of the same union build differ by 22 px and 7 px in those same states, at the agent's status dot and at rounded corners (1–16 px clusters). The residuals are noise, not change |
| Sweep page checks | 0 axe violations, page errors, remote requests, step errors or window scroll; at most 102 dock rows mounted |
| 0035 set page checks (`evidence/union-review.json`) | <ul><li>102 captures with 0 page errors, remote requests, step errors or window scroll.</li><li>Every dialog fits the window.</li><li>One axe violation, the inactive-window crumb (finding 9).</li><li>Clipped values are only reference IDs and asset paths in reference boxes. Each is ellipsized with the full value in the tooltip, after the resolved name where one exists: the same pattern 0033 recorded.</li></ul> |

### The director's five requirements in the union

| Requirement | Observed |
| --- | --- |
| 1. No useless text or rows | <ul><li>No compound paragraph or Entity/ID/Read-only row in any union state.</li><li>The header is still glyph, name, lock and Copy ID.</li><li>The only repeated new sentence is Region's "not supported yet" (finding 3). It is an explicit unsupported-field notice, not decoration.</li></ul> |
| 2. Compact account dialog | <ul><li>All 11 states render 420 px wide and fit the window, including the five also captured at 1000×650 (`screenshots/account/`).</li><li>Switching, adding, Cancel sign-in, the sign-out question and errors with code all work.</li><li>Focus: open puts it on Close; the browser step on Open browser again; Cancel on Close; the question on Keep signed in.</li><li>Escape dismisses the question, then the dialog, and focus returns to the account button.</li><li>**Exception:** Use drops focus (finding 1).</li></ul> |
| 3. Professional shortcut table | <ul><li>One table of 29 entries. macOS reads ⇧⌘Z, ⌥⌘1, ⌘⌫ / ⌦; Windows reads Ctrl+Y or Ctrl+Shift+Z, Delete or Ctrl+Backspace.</li><li>Filter "hierarchy" gives 10 of 29; "ctrl shift" on Windows gives 1 of 29; no match shows the empty state.</li><li>Each key cell is named group, action and keys, as computed: "Hierarchy Delete Command Delete , or Forward Delete".</li><li>Focus is trapped. Escape clears the filter, then closes, and focus returns to the opener.</li></ul> |
| 4. Avatar-only account control | <ul><li>30×28 px in all 102 captures, showing at most one initial.</li><li>The state lives in the accessible name and tooltip.</li><li>The corner dot is amber while signing in and red on a problem.</li><li>It fits beside the macOS inset and beside the Windows controls at 1000×650.</li></ul> |
| 5. No ornament | <ul><li>Pixel-identical to the plain screenshots apart from the halo removal.</li><li>The union states use the same plain components: no tiles, glows, accent edges or pills.</li></ul> |

## Camera: the three real projections

| State | Observed (`screenshots/camera/`) |
| --- | --- |
| Camera Rig, sample, projection omitted | "Perspective", tagged Default; clean at 280 px |
| Close-up Camera, sample | Perspective written out; clean at 1000×650 |
| Map Camera, sample and double | Orthographic, Vertical size in m, field of view tagged Unused |
| Pixel Camera, double | <ul><li>Shows "Pixel perfect", "32 px/m" and "Resolution px W 320 H 180", with field of view tagged Unused.</li><li>Fits at 280 px and at 1000×650 with "px" visible.</li><li>The Pixel density focus ring is visible.</li></ul> |
| Pixel Camera (out of range) | <ul><li>The engine message appears at the top and under Projection.</li><li>Density stays 0.5 px/m.</li><li>Resolution reads "Cannot show as 2 numbers (logical width and logical height); raw value: [320]".</li><li>Field of view is not tagged.</li></ul> |
| Broken Camera, `fisheye` | "Unknown projection kind "fisheye". Expected perspective, orthographic or pixel_perfect; raw value:" |

The sample fixture declares `pixel_perfect` but has no pixel-perfect camera. The double covers
it with the real schema.

## SpriteRenderer and cooked atlases as the union shows them

All of these come from the generic schema path, in `screenshots/sprite/` and
`screenshots/assets/`:
- **Order.** The rows are alphabetical: Flip x, Flip y, Layer, Mode, Order, Pivot, Region,
  Sampling, Size, Texture, Tint. Texture and Region come last.
- **Texture** shows the asset's name (`harbor_ui`) before its ellipsized ID.
- **Region** shows a warning glyph, "This field's type is not supported yet; raw value:" and
  the raw JSON, for valid frames and rectangles alike. The glyph is the same amber triangle as
  a real engine warning.
- **Tint** (RGBA) and the nine-slice **Border** (left, right, top, bottom) are labelled X Y Z W.
  Chrome's accessibility tree names their textboxes "X", "Y", "Z" and "W".
- **Mode** and **Sampling** show raw tags (`simple`, `nine_slice`, `nearest`), as Collider
  shapes and Rigid Body motion already do.
- **Required fields only.** Flip x, Flip y and Sampling read "Not set", while Layer, Order and
  Mode show their default with a Default tag.
- **Malformed values** are all shown raw:
  - `flip_x: "yes"`;
  - unknown mode `tiled`;
  - an extra field `opacity`;
  - the engine messages, with paths.
- **Problems.** A missing texture, a linear texture and a missing frame each show the engine
  message at `/texture` or `/region`.
- **Long names** ellipsize in the tree and the header. An int32 order of 2147483647 shows as
  "2.147e9", with the exact value in the tooltip.
- **Atlas assets** are listed and detailed as ordinary colour textures with the right source
  path. Interpretation still offers Linear and Normal map, which the engine refuses for
  atlases. The snapshot carries no frame data, so no frames are shown.
- **Sprite entities** use the generic entity glyph.

## Interaction probes

All results are in `evidence/union-review.json` → `observations`.

- **Compound End then Tab**, 64 parts, at 1440×874 and 1000×650:
  - Home gives Part 1.
  - End focuses Part 64. Parts 58–64 are visible, 50–64 mounted, with no blank slot, and the
    caption reads "Part 64 of 64".
  - Tab moves to Part 64's Primitive. The accessibility tree lists Parts 50–64 with 64
    selected.
  - PageDown ×2 from Part 1 gives Part 11, with Parts 5–11 visible.
- **District Grid, Show values** (65,536 costs):
  - The value table holds focus throughout.
  - It mounts at most 528 cells (22 rows) through PageDown ×3, End (rows 247–255 visible),
    ArrowRight ×60 and the last column (index 65,535).
  - Home returns to rows 0–7 with the focus ring.
- **Pager** (450 problems):
  - Next announces "Showing problems 101–200 of 450".
  - Hiding and showing warnings resets the page.
  - Next again clears the live text and announces the same range about 20 ms later.
  - Last announces 401–450. Enter on the disabled Last page keeps focus and announces nothing.
  - The status line never echoes "Showing…"; 58 rows are mounted on the last page.
- **Account flow** (three accounts):

  | Step | Focus or result |
  | --- | --- |
  | Open | Close |
  | Use ada | **body** (finding 1) |
  | Add another account | Open browser again |
  | Cancel sign-in | Close |
  | Sign out | Keep signed in |
  | Escape | Close, dialog stays open |
  | Confirm | Continue with ChatGPT, signed out with saved accounts |
  | Escape | The account button |
- **F6** cycles dock, inspector, agent, hierarchy, viewport. **Shift+F6** goes back.
- **Menu.** Shift+F10 opens it and focuses Rename. Rename and Delete stay focusable, explain
  why they are unavailable, and show F2 and ⌘⌫. ArrowDown moves to Copy ID. Escape returns
  focus to the row. The menu stays inside the 1000×650 window.
- **Problems reveal.** A row selects and reveals its entity and announces "Selected …".
  Focus stays in the dock, by design. The part-41 diagnostic opens Rubble Pile with Part 42
  in view.

## Findings

Ranked by user impact. "Pre-existing" means the code is identical at ce06c31 and on the
sprite/atlas line.

1. **Account switch drops keyboard focus.** Pre-existing. Behaviour owned by Astra.
   - **Repro** (`tools/findings.mjs` §1):
     - In signed-in-multi, open the dialog with Enter, Tab to "Use ada@example.com" and press
       Enter. Focus moves to `<body>`.
     - An immediate Escape does nothing: the key handler is on the dialog.
   - **Why.** The Use button disappears, and focus recovery runs only when status or phase
     changes. The native host switches saved accounts from connected to connected
     (`editor/app/src/provider.rs`), so the real app is affected too.
   - **Fix.** Add the active account to `stateKey` in `AccountDialog.tsx`, so the existing
     recovery puts focus on Close. This also clears a stale sign-out question. Add a test:
     switch by keyboard, then focus is Close and Escape closes.
2. **The import form refuses paths the union's importer accepts.** Behaviour owned by Astra.
   - **What.** `assets/paths.ts` claims to mirror `incant_import` but accepts only glTF, GLB,
     PNG, JPEG and EXR. The importer also takes `.atlas.json`, new on the sprite/atlas line,
     and WAV or OGG, which predate the union.
   - **Repro** (`screenshots/assets/u-import-mirror-…`): `sprites/dock_props.atlas.json` and
     `audio/gulls.ogg` are marked "Unsupported file type" and submission is refused ("Fix 2
     highlighted paths before importing.").
   - **Fix.**
     - Add `atlas` (colour only, no usage choice, send no usage) and `audio` (no usage) source
       kinds.
     - Use the importer's own format list in `ACCEPTED_FORMATS`.
     - For a `.atlas.json` texture, stop offering Linear and Normal map under Reimport.
3. **Region always reads "not supported yet".** Bridge owned by Astra, then a Claude
   presentation.
   - **What.** `inspectorField` maps an `anyOf` with two concrete alternatives to
     `unsupported`. Truthful ("unknown capabilities fail explicitly"), but it marks every valid
     sprite with a warning glyph.
   - **Fix.** Add a contract type for untagged unions whose alternatives differ by JSON type.
     The Inspector would then show a rectangle as x, y, width, height in px, and a frame as its
     identifier, named once atlas frames reach the snapshot (finding 6).
4. **SpriteRenderer rows are alphabetical.** Rust schema registry, owned by Astra.
   - **Fix.** Add `("SpriteRenderer", vec!["texture", "region", "size", "pivot", "tint",
     "mode", "sampling", "flip_x", "flip_y", "layer", "order"])` to the shared order annotations
     in `schema_registry()`, read by all clients.
   - **Order.** What is drawn and where, then how, then sorting.
5. **Tint and Border are labelled and named X/Y/Z/W.**
   - **Tint.** Annotate `tint` with `x-incant-widget: "rgba"`, as lights annotate `color` with
     `rgb`. The Inspector then shows R/G/B/A with a preview swatch and names Red, Green, Blue
     and Alpha.
   - **Border.** Needs a SpriteRenderer presentation profile naming Left, Right, Top and Bottom.
     I can do this after findings 3–4, in one scoped sprite pass with variant names ("Simple",
     "Nine-slice"), "Flip X" / "Flip Y" and a component icon.
6. **Atlases are indistinguishable from textures.** Data binding owned by Astra.
   - The snapshot carries no atlas marker or frame list, so the editor cannot show "Atlas, N
     frames", list frames, or name a sprite's frame.
   - **Request.** A bounded atlas summary per texture asset: frame IDs, names and page size,
     from cooked metadata.
7. **Omitted booleans and strings read "Not set"; numbers and unions show the default.**
   Pre-existing. Claude, decision pending.
   - **Why.** `presentableDefault` handles number and tagged-union defaults only.
   - **Effect.** A sprite authored with only its five required fields shows flips and
     sampling as "Not set", although serde applies `false` and `nearest`.
   - **Proposal.** Present boolean and enum-string defaults the same way, as the value with a
     Default tag. This touches every component, so it needs a decision and tests.
8. **Sprite entities use the generic entity glyph.** Bridge `kind`, owned by Astra. It would
   need a `sprite` kind in `snapshotFromEngine`.
9. **Inactive-window scene crumb at 3.35:1.** Pre-existing; Claude follow-up.
   - **What.** `shell.css` dims the titlebar with `opacity: 0.55` when the host reports an
     unfocused window. The muted crumb `#a9aeb9` then renders `#62656d` on `#0b0c0f`, 3.35:1,
     against 8.79:1 when focused. axe flags it (`tools/findings.mjs` §3).
   - **Why it is open.** macOS dims inactive titles too, but WCAG's exemption covers disabled
     components only.
   - **Options.** Dim to about 0.7, or exempt the crumb.
10. **Integers of 1e9 or more show in exponent form.** Pre-existing. `formatNumber` shows
    2147483647 as "2.147e9", with the exact value in the tooltip. Integer-typed fields could
    show every digit.

**Checked, not defects:**
- Specks on the account dialog's top hairline at 1000×650 are the viewport sentence behind the
  dim scrim, seen through the dialog's 7 % white 1 px ring. They vanish when that text is
  hidden (`screenshots/regression/edge-*`).
- Windows minimize, maximize and close render only for advertised `window.*` capabilities. The
  double advertised them in `title-windows-controls`.
- The dock reveal keeps focus in the list, by design.

## Native: not observed

The director's 0d14f4f preview was not touched, and the a20d258 union binary was not opened.
No older capture is labelled as this union. Each item below is still to be checked on an
isolated a20d258 build, with account-free crops for anything committed:
1. **1000×650 minimum:** the Inspector with 64 parts, the pixel-perfect camera, the account
   dialog and the shortcuts dialog.
2. **Compound End then Tab:**
   - trailing rows visible with no blank region, and accessibility listing about Parts 50–64;
   - repeated with the window not frontmost.
3. **District Grid, Show values:** PageDown, End and horizontal scrolling stay bounded and
   focusable.
4. **Account dialog:**
   - two or more accounts;
   - Use, then focus and Escape (finding 1);
   - the sign-out question with focus on Keep signed in;
   - the browser step, then Cancel moving focus to Close;
   - an error state.
5. **Keyboard shortcuts:**
   - WebKit glyphs ⇧⌘Z, ⌥⌘1, ⌘⌫ and ⌦;
   - "hierarchy" gives 10 of 29;
   - VoiceOver reads group, action and keys per cell.
6. **Context menu:** opened by Shift+F10, with ⌘⌫ shown for Delete.
7. **Dock paging:** no "Showing…" echo, and a repeated range is announced again under
   VoiceOver.
8. **Titlebar:**
   - avatar only;
   - the amber and red corner dots;
   - the inactive dimming;
   - the traffic-light inset;
   - the plain status dot after cdf0fab.
9. **Union content natively:**
   - the pixel-perfect camera and SpriteRenderer Inspector, confirming native data writes
     every sprite field;
   - atlas asset details;
   - the import form with a saved project (finding 2).
10. **VoiceOver:** names of the account button and the Inspector header; pager announcements.

## Commands and results

```sh
npm ci --ignore-scripts --workspace editor/ui --no-audit --no-fund    # pinned lockfile, 101 packages
cd editor/ui
npx tsc -b --noEmit                                                   # strict: 0 errors
npx vitest run                                                        # 29 files, 503 tests passed
npm run build                                                         # built
cd ../..
node handoffs/0033-editor-design-review/tools/capture.mjs u35-sweep   # 96 captures; 0 axe, step or page errors
node handoffs/0035-editor-union-review/tools/compare-sweep.mjs u35-sweep --out=artifacts/0035-editor-union-review/sweep-compare.json   # 94 of 96 identical
node handoffs/0035-editor-union-review/tools/pixel-diff.mjs handoffs/0033-editor-design-review/screenshots/plain/after artifacts/0033-editor-design-review/u35-sweep artifacts/0035-editor-union-review/pixel-plain --json=artifacts/0035-editor-union-review/pixel-plain.json   # 27 of 29 identical
node handoffs/0033-editor-design-review/tools/capture.mjs u35-repeat --only=no-selection,dock-paged-problems
node handoffs/0035-editor-union-review/tools/pixel-diff.mjs artifacts/0033-editor-design-review/u35-sweep artifacts/0033-editor-design-review/u35-repeat artifacts/0035-editor-union-review/pixel-repeat --json=artifacts/0035-editor-union-review/pixel-repeat.json   # 7–22 px noise
node handoffs/0035-editor-union-review/tools/capture.mjs r2           # 100 captures; 0 step or page errors; 1 axe (finding 9)
node handoffs/0035-editor-union-review/tools/capture.mjs s1 --only=title-windows-controls
node handoffs/0035-editor-union-review/tools/capture.mjs s2 --only=probe-end-tab   # rerun after correcting the probe's visible-row count
node handoffs/0035-editor-union-review/tools/summarize.mjs r2,s1,s2 --out=artifacts/0035-editor-union-review/summary-final.json
node handoffs/0035-editor-union-review/tools/findings.mjs artifacts/0035-editor-union-review/findings-edge
node handoffs/0035-editor-union-review/tools/curate.mjs
(cd handoffs/0035-editor-union-review && shasum -a 256 -c evidence/screenshots.sha256)   # 43 OK
git diff --check                                                      # clean
```

- **Repeats.** The typecheck, tests and build ran at the start and again at the end, with the
  same results.
- **Earlier sets.** Set `r1` and the `smoke1`/`fix1` sets exposed harness faults, all fixed
  before r2:
  - the tree did not scroll deep rows into its mounted window;
  - the tool clicked an `aria-disabled` button;
  - a status selector was not scoped to the dialog.
- **Not run:**
  - Cargo, Clippy and GPU tests;
  - the native app, WebKit and VoiceOver;
  - phones;
  - live accounts.

## Evidence

Committed under this handoff. The 39 screenshots are all account-free: invented example
addresses, the sample fixture or the evidence double.
- **Screenshots.** Unedited captures, `sips` crops (`title-*-end`, `grid-corner`) and 4×
  enlargements (`edge-*`).
- **Hashes.** `evidence/screenshots.sha256` lists 43 SHA-256 hashes: every screenshot plus the
  four JSON files.
- **Logs.** In the ignored `artifacts/0035-editor-union-review/logs/`.
  - `baseline-vitest.log`: `b2fee5e5…`.
  - `r2.log`: `b2ec2fe7…`.
  - `u35-sweep.log`: `7c93a170…`.

| Path | Shows |
| --- | --- |
| `screenshots/camera/u-pixel-camera-{normal-1440x874,minimum-1000x650}.png` | Pixel-perfect camera, whole window |
| `screenshots/camera/u-pixel-bad-…`, `u-broken-camera-…` | Malformed density and resolution; unknown projection |
| `screenshots/camera/camera-rig-narrow-…`, `map-camera-narrow-…`, `closeup-camera-minimum-…` | Default perspective, orthographic, explicit perspective |
| `screenshots/sprite/u-sprite-sign-{normal-…-inspector,minimum-1000x650}.png` | Atlas-frame sprite: order, Region notice, tint X/Y/Z/W |
| `screenshots/sprite/u-sprite-board-narrow-…`, `u-sprite-coil-narrow-…` | Nine-slice border X/Y/Z/W; raw atlas rectangle with its warning |
| `screenshots/sprite/u-sprite-lantern-…`, `u-sprite-broken-…`, `u-sprite-long-narrow-…` | "Not set" beside "Default"; malformed values; long names and "2.147e9" |
| `screenshots/assets/u-asset-atlas-…`, `u-import-mirror-…` | Atlas as an ordinary texture; refused atlas and audio paths |
| `screenshots/account/*` | Three accounts; saved accounts at minimum; the browser step at minimum; an error while signed in; the sign-out question |
| `screenshots/titlebar/*` | Windows controls and the macOS inset at minimum; inactive dimming; signed-in, error and signing-in ends |
| `screenshots/shortcuts/*` | macOS at minimum; Windows notation; "hierarchy" filter |
| `screenshots/interaction/*` | End at minimum (Part 64); grid table (corner and Home); pager last page; keyboard menu at minimum; reveal of the raw-rectangle warning |
| `screenshots/regression/*` | Pixel-diff mask (status-dot halo plus noise); dialog-edge experiment |
| `evidence/union-review.json` | Per-capture rows, notices, account control, dialogs, live text and probe observations for all 102 captures |
| `evidence/sweep-compare.json`, `pixel-plain.json`, `pixel-repeat.json` | The plain-design comparisons above |

## Changed paths

- `handoffs/0035-editor-union-review/result.md`: this file.
- `handoffs/0035-editor-union-review/tools/`: the seven new tools listed in Method.
- `handoffs/0035-editor-union-review/screenshots/` and `evidence/`.

Nothing in `editor/`, `crates/`, `schemas/`, `docs/`, CI or PLAN changed. Not committed are the
raw sets (`u35-sweep`, `u35-repeat`, `r1`, `r2`, `s1`, `s2`, `smoke1`, `fix1`), the scratch
experiments and the logs, all in the ignored `artifacts/`.

## Requests for Astra

1. Findings 1 and 2, behaviour fixes, each with a test. I can review their rendered result.
2. Findings 4 and 5 (tint), Rust shared schema annotations. Finding 3, the bridge's untagged
   unions. Findings 6 and 8, data bindings.
3. When 3–6 land, a scoped Claude sprite-presentation pass:
   - finding 5's border names, variant names, flip labels and icon;
   - the Region presentation;
   - frame names;
   - a decision on finding 7.
4. An isolated native capture of a20d258 for the checklist above.
5. `docs/spikes/editor-release-union.md` still says this review is pending. That doc is
   outside my allowed paths.

## Open questions

- Should a sprite authored without `sampling` or flips show "nearest" and "Off" with a Default
  tag (finding 7)? That would apply to every component's boolean and enum defaults.
- Should the sample fixture gain a pixel-perfect camera, now that its schema declares one?
- How should a frame-addressed Region read once frame names exist: the name only, or the name
  and ID?
