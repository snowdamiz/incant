# 0035 release-union review: result

## Final focused revision: typed imports and account focus

This answers the packet's priority revision "final typed import reconciliation
(2026-10-11)", acknowledged and applied. The earlier complete review further down stays
valid for its source (a20d258), and its evidence and limits are preserved.

### Status

- **Done, with one narrow presentation change and tests.** Atlas and audio sources now
  read correctly in the asset list, the asset details and the import form, in the
  existing plain style.
  - **Atlas** (a texture whose source is `.atlas.json`):
    - the list tags it "Atlas";
    - its Interpretation shows the saved value, "Color", in a plain value box, with "Sprite
      atlases are always color.";
    - there is no Linear or Normal map choice, and the button only reads "Reimport".
  - **Import rows** name their type: "Color atlas", "WAV audio", "OGG audio". Models keep
    "Model", and images keep their interpretation choice.
  - **Audio** is labelled "Audio" (it was the raw "audio") and has no texture settings.
  - **Ordinary images are unchanged.** Their interpretation stays editable, and "Reimport as
    …" still sends the change.
- **Astra's two fixes, verified in the browser:**
  - **Typed imports (c131ed8).** Atlas, WAV and OGG paths are accepted and sent in one
    request without a texture usage. Generic JSON is refused before sending.
  - **Account focus (aa8ed10).** Use moves focus to Close and Escape closes. A host account
    change clears an open sign-out question and moves focus to Close.
- **One new behaviour finding for Astra (finding 11).** Keyboard focus falls to `<body>`
  after an import or reimport completes. It predates this revision but is now reachable for
  atlas and audio sources.
- **Preserved.** The 14 committed Camera and SpriteRenderer screenshots are pixel-identical
  on the final source. Findings 3–10 involve unchanged code and remain open.
- **Not changed:** Rust, Cargo, schemas, transport, credentials, account requests and path
  validation. No phase gate is claimed. Native final-pixel acceptance stays pending.

### Model and effort

- Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Thinking effort: Max. The session reports reasoning effort "max" to the model. The runner
  selects and verifies both through ACP, which I cannot inspect from inside the session.
- No substitution.

### Source

| Item | Value |
| --- | --- |
| Base | `e52eef4`: Astra's integration of c131ed8 (typed imports), aa8ed10 (account focus) and my first-round commit |
| This commit | Presentation and tests in `editor/ui`. The `editor/ui` tree is `5ad2744806a63f08d62bd0a82bfb53e48ba4f069` |
| Unchanged from the base | `editor/bridge` `4dca2112…`, `schemas` `0c174377…`, `editor/app` `b9add18c…`, `crates` `e9e2c5fa…` |
| Since my first-round source (3a6aacb) | <ul><li>Editor: Astra's import and account changes with their tests.</li><li>Crates: one Rust test, `crates/incant_import/tests/sprite_references.rs`.</li></ul> |

### What changed

| Surface | Before (e52eef4) | Now |
| --- | --- | --- |
| Import row with an atlas or audio source | A generic file glyph, the model tint class, no type line and no "Updates" note | The texture or speaker glyph with its kind class; the type line "Color atlas", "WAV audio" or "OGG audio"; "Updates “name”" when the path matches a project asset |
| Asset list, atlas | Tag "Color" | Tag "Atlas", spoken "Texture, Atlas" |
| Asset list and header, audio | Kind spoken as "audio" | "Audio" |
| Atlas details | <ul><li>An editable Color / Linear / Normal map choice.</li><li>Choosing Linear relabelled the button "Reimport as Linear", although the request sends no usage for an atlas.</li></ul> | <ul><li>The saved value, "Color", in a plain value box, with "Sprite atlases are always color.".</li><li>The button names a change only when `reimportRequest` carries one.</li></ul> |
| Ordinary image, model and audio details | — | Unchanged, apart from the audio label above |

Changed files:
- `src/assets/paths.ts`: labels only (`KIND_LABEL.audio`, `textureTag`, `sourceTypeLabel`).
  Validation is untouched.
- `src/components/assets/AssetBrowser.tsx` and `src/components/assets/AssetInspector.tsx`.
- `src/styles/assets.css`: the new `.asset-value` rule.
- `src/components/assets/Assets.test.tsx`: three tests.
- `DESIGN.md`: one paragraph, "Assets by source type".

### Tests

- **New.** Three presentation tests: typed import rows, atlas details and reimport, and WAV
  audio details. Against the previous presentation they fail (3 failed, 28 passed in the
  file); with this change they pass.
- **Existing.** With the presentation change and the previous test file, all 517 existing
  tests pass, so no existing assertion changed.
- **Full suite.** 30 files and 520 tests pass, the strict typecheck is clean, and the
  production build passes.

### Rendered check

- **Captures.** Headless Chrome 155.0.8059.40 on the final build: 26 captures (sets `f1` and
  `f2`).
- **Results.** 0 axe violations, page errors, remote requests, step errors, window scroll or
  clipped values; every dialog fits. Observations are in `evidence/typed-import-review.json`.

What is real and what is mocked:
- **Real UI logic:**
  - path validation and row labels;
  - the requests the UI builds, recorded exactly as dispatched;
  - focus, and all presentation.
- **Mocked:**
  - every host result from the live double, where `{ ok: true }` stands in for a native cook
    and commit;
  - the asset list it publishes afterwards;
  - the host-side account change.
- **Supported natively but not observed here** (per `incant_import`, the importer):
  - accepts `.atlas.json`, WAV and OGG, and refuses other JSON;
  - cooks atlases as color only;
  - registers audio as kind `audio`.

| Check | Observed |
| --- | --- |
| Typed sources (normal, minimum) | <ul><li>Rows: "Color atlas" (texture glyph); "WAV audio" and "OGG audio" (speaker glyph); the PNG with its interpretation choice.</li><li>"Import 4 files" sent one request: `sprites/dock_props.atlas.json`, `audio/gulls.wav`, `audio/bell.OGG` and `textures/dock_sign.png`, none with a usage.</li><li>The mocked success reads "Imported 4 files. Undo reverts the whole batch."</li><li>The mocked list shows the new atlas ("Atlas"), audio and image rows with the just-imported mark.</li></ul> |
| Generic JSON | <ul><li>`data/level.json`, `sprites/sheet.json` and `sprites/ui.atlas.json.bak` each read "Unsupported file type. Use glTF, GLB, PNG, JPEG, EXR, .atlas.json, WAV or OGG."</li><li>Nothing is sent; the status line says "Fix 3 highlighted paths before importing."; focus moves to Path 1.</li></ul> |
| Atlas details and reimport (normal, minimum, narrow, long name) | <ul><li>Spoken "Texture, Atlas"; shows "Color" and "Sprite atlases are always color."; no radios.</li><li>Reimport sent `{ source: 'sprites/harbor_ui.atlas.json' }`; the mocked result reads "Reimported."</li></ul> |
| Ordinary image (normal, narrow) | <ul><li>Three choices. Linear gives "Saved as Color; reimport to apply." and "Reimport as Linear".</li><li>That sent `{ source: 'textures/props/crate_albedo.png', textureUsage: 'linear' }`.</li><li>After the mocked success the double still reports Color, because it publishes no new usage.</li></ul> |
| OGG audio (normal, minimum) | Spoken "Audio"; no Interpretation; Reimport sent `{ source: 'audio/harbor_loop.ogg' }` |
| The earlier import reproduction | The atlas and OGG rows are now accepted. The request reaches the double, which refuses it as read-only (mocked) |
| Account switch on the real fixture route (normal, minimum) | Enter on "Use ada@example.com" leaves focus on Close, ring visible. Escape closes and returns focus to the account button |
| Host account change (live double) | <ul><li>The question "Sign out of studio@example.org…" opens with focus on Keep signed in.</li><li>The host then publishes ada as active: the question clears, focus moves to Close, and the dialog sends no request.</li><li>Escape closes.</li></ul> |
| The earlier account probe | Use now leaves focus on Close; it used to drop to `<body>` |
| Camera and SpriteRenderer | The 14 committed screenshots, recaptured as set `v2`, are pixel-identical: 7 of 7 and 7 of 7 (`evidence/pixel-final-camera.json`, `pixel-final-sprite.json`) |
| Committed asset and account screenshots | The atlas details and the import reproduction differ as intended; the account question step is identical (`pixel-final-assets.json`, `pixel-final-account.json`) |

### Finding 11: focus after an import or reimport

New, behaviour, owned by Astra; not changed here.
- **What.** Keyboard focus is lost when an import or reimport completes.
- **Repro** (`tools/findings.mjs` §5, live double, mocked results):

  | Action | Result | Focus afterwards |
  | --- | --- | --- |
  | Import, Enter in the path | Success | `<body>` |
  | Reimport, Enter on the button | Success | `<body>` |
  | Reimport | Failure | `<body>` |
  | Import | Failure | Path 1 (kept) |
- **Why.** While the request runs, the progress line replaces the submit or Reimport button.
  A successful import also resets the rows. Either way, the focused element disappears.
- **History.** The same code was in 3a6aacb. Atlas and audio sources can now reach it.
- **Proposal.** Keep the button mounted, with `aria-disabled` and a busy label, or move focus
  to the outcome notice when the request settles. Add a keyboard test.

### Status of all findings

| Finding | Status |
| --- | --- |
| 1, account switch focus | Fixed in aa8ed10; verified above |
| 2, import refuses typed sources | Fixed in c131ed8, with this revision's presentation; verified above |
| 3–10 | Code unchanged; still open, as recorded below |
| 11, focus after an import | New; above |

### Commands and results

```sh
cd editor/ui
npx vitest run src/components/assets/Assets.test.tsx   # new tests on the previous presentation: 3 failed, 28 passed
npx vitest run                                         # presentation change, previous test file: 517 passed
npx tsc -b --noEmit                                    # strict: 0 errors
npx vitest run                                         # 30 files, 520 tests passed
npm run build                                          # built
cd ../..
node handoffs/0035-editor-union-review/tools/capture.mjs f1 --only=t-import-typed,t-import-json,t-atlas-details,t-atlas-long,t-image-reimport,t-audio-details,t-account-switch,t-account-host-change,u-assets,u-asset-atlas,u-asset-atlas-long,u-asset-sheet,u-import-mirror,probe-account   # 26 captures
node handoffs/0035-editor-union-review/tools/capture.mjs f2 --only=t-account-host-change   # adds the "cleared" snapshot
node handoffs/0035-editor-union-review/tools/capture.mjs v2 --only=u-pixel-camera,u-pixel-bad,u-broken-camera,camera-rig,map-camera,closeup-camera,u-sprite-sign,u-sprite-board,u-sprite-coil,u-sprite-lantern,u-sprite-broken,u-sprite-long --sizes=normal,minimum,narrow   # 30 captures
node handoffs/0035-editor-union-review/tools/pixel-diff.mjs handoffs/0035-editor-union-review/screenshots/camera artifacts/0035-editor-union-review/v2 artifacts/0035-editor-union-review/pixel-v2-camera --json=artifacts/0035-editor-union-review/pixel-v2-camera.json   # 7 of 7 identical; sprite likewise
node handoffs/0035-editor-union-review/tools/pixel-diff.mjs handoffs/0035-editor-union-review/screenshots/assets artifacts/0035-editor-union-review/f1 artifacts/0035-editor-union-review/pixel-f1-assets --json=artifacts/0035-editor-union-review/pixel-f1-assets.json   # 2 intended differences; account: 1 identical
node handoffs/0035-editor-union-review/tools/findings.mjs artifacts/0035-editor-union-review/findings-edge-rev2   # §1 fixed; §5 is finding 11
node handoffs/0035-editor-union-review/tools/summarize.mjs f1,f2 --out=artifacts/0035-editor-union-review/summary-typed.json
node handoffs/0035-editor-union-review/tools/curate.mjs
(cd handoffs/0035-editor-union-review && shasum -a 256 -c evidence/screenshots.sha256)   # 60 OK
```

- **The two runs on the previous presentation and test file** temporarily restored the
  committed files, then put the edits back; `git diff` confirmed the restore.
- **Not repeated:** the 102-state set and the 96-state sweep.
- **Not run:**
  - Cargo;
  - the native app, WebKit and VoiceOver;
  - phones.

### Evidence for this revision

`screenshots/final/` holds 12 account-free images; the crops were made with `sips`.

| File | Shows |
| --- | --- |
| `t-import-typed-normal-1440x874--rows.png` | Typed rows before submitting, and the list tags |
| `t-import-typed-minimum-1000x650.png` | The mocked success, the reset form and the new list rows |
| `t-import-json-normal-1440x874-inspector.png` | Generic JSON refused |
| `u-import-mirror-normal-1440x874-inspector.png` | The earlier reproduction, now accepted; it pairs with the file of the same name in `screenshots/assets/` |
| `t-atlas-details-normal-1440x874--details.png` | Atlas details, with Color fixed |
| `t-atlas-details-narrow-1440x874-inspector.png` | The same at 280 px, after the mocked reimport |
| `t-atlas-long-narrow-1440x874-inspector.png` | A long atlas name and path |
| `image-linear-narrow.png` (crop) | An ordinary image with Linear chosen |
| `t-audio-details-minimum-1000x650--details.png` | OGG audio details |
| `switch-close.png` (crop) | Focus on Close after Use |
| `host-change-question.png`, `host-change-cleared.png` (crops) | The question before, and the dialog after, the host account change |

- **Evidence JSON:** `typed-import-review.json` and
  `pixel-final-{camera,sprite,assets,account}.json`.
- **Hashes:** `evidence/screenshots.sha256` lists 60. The first-round entries are unchanged.
- **Logs** (ignored):
  - `final2-vitest.log`: `5086cbc9…`.
  - `f1.log`: `19b78571…`.
  - `v2.log`: `9be99630…`.
  - `findings-rev2.log`: `df099e4c…`.

### Native: still not observed

The earlier checklist below still stands. On an isolated build of this source, also check:
1. **Import into a saved project** of a real `.atlas.json`, a WAV and an OGG file, then:
   - the cooked list: the Atlas tag and the audio rows;
   - atlas details, with Color fixed;
   - reimport.
2. Generic JSON is refused.
3. Focus after Use, and after a host account change.
4. Focus after an import or reimport (finding 11).

### Changed paths in this commit

- **`editor/ui`:**
  - `src/assets/paths.ts`;
  - `src/components/assets/AssetBrowser.tsx` and `AssetInspector.tsx`;
  - `src/styles/assets.css`;
  - `src/components/assets/Assets.test.tsx`;
  - `DESIGN.md`.
- **`handoffs/0035-editor-union-review/`:**
  - `result.md`;
  - `tools/`: the `t-*` states in `capture.mjs`, the live double in `union-double.mjs`, §5 of
    `findings.mjs`, and the `final` group in `curate.mjs`;
  - `screenshots/final/`;
  - five new evidence JSON files, and the updated manifest.

## Earlier complete review: source a20d258 (commit 9fb82e6)

Preserved as recorded. Its evidence and limits hold for that source. Findings 1 and 2 have
since been fixed and verified (above).

### Status

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

### Model and effort

- Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Thinking effort: Max. The session reports reasoning effort "max" to the model. The runner
  selects and verifies the model and effort through ACP. I cannot inspect the ACP transport
  from inside the session.
- No model or effort substitution occurred.

### Packet and boundaries

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

### Source identity

| Item | Value |
| --- | --- |
| Branch head | `3a6aacb` (handoff/0035-editor-union-review) |
| Code union | `a20d258ba75fd00b2c8bea91cf71de8bead78370`. These trees are identical at the head and at a20d258: `editor/ui` `5b13ad49…`, `editor/bridge` `4dca2112…`, `schemas` `0c174377…`, `crates` `76662041…`, `editor/app` `b9add18c…` |
| Union delta over the plain editor (ce06c31), editor side | <ul><li>`bridge/fixture.ts`: the Camera schema gains `pixel_perfect`.</li><li>`CameraFields.test.tsx` and `PixelCameraFields.test.tsx` now use the real schema.</li><li>A comment in `presentation.ts`.</li><li>Schemas: Camera adds `pixel_perfect`; SpriteRenderer and SpriteAtlas are new.</li></ul> |
| What the native host sends | The whole `schema_registry()`: 20 schemas, including Project, SpriteAtlas and SpriteRenderer. SpriteRenderer has no shared `order` annotation, and its tint has no colour widget annotation. `snapshotFromEngine` turns `region`, an untagged `anyOf` of a rectangle or `{frame}`, into `unsupported`. No sprite-specific UI code exists. |
| Browser | Headless Chrome 155.0.8059.40 at device scale 2, on an Apple M5 Pro with 48 GB and macOS 26.6 (the same Chrome as 0033) |

### Method

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

### The union versus the plain design

| Check | Result |
| --- | --- |
| Metrics of the 96 0033 states (`evidence/sweep-compare.json`) | 94 identical. Broken Camera (normal, minimum) has +1 Inspector word from the new message "Expected perspective, orthographic or pixel_perfect" |
| Pixels of the 29 committed plain screenshots (`evidence/pixel-plain.json`) | 27 identical. `dock-paged-problems-minimum`: 420 px at the status dot (the halo cdf0fab removed), plus 22 px. `no-selection-normal`: 7 px |
| Noise check (`evidence/pixel-repeat.json`) | Two captures of the same union build differ by 22 px and 7 px in those same states, at the agent's status dot and at rounded corners (1–16 px clusters). The residuals are noise, not change |
| Sweep page checks | 0 axe violations, page errors, remote requests, step errors or window scroll; at most 102 dock rows mounted |
| 0035 set page checks (`evidence/union-review.json`) | <ul><li>102 captures with 0 page errors, remote requests, step errors or window scroll.</li><li>Every dialog fits the window.</li><li>One axe violation, the inactive-window crumb (finding 9).</li><li>Clipped values are only reference IDs and asset paths in reference boxes. Each is ellipsized with the full value in the tooltip, after the resolved name where one exists: the same pattern 0033 recorded.</li></ul> |

#### The director's five requirements in the union

| Requirement | Observed |
| --- | --- |
| 1. No useless text or rows | <ul><li>No compound paragraph or Entity/ID/Read-only row in any union state.</li><li>The header is still glyph, name, lock and Copy ID.</li><li>The only repeated new sentence is Region's "not supported yet" (finding 3). It is an explicit unsupported-field notice, not decoration.</li></ul> |
| 2. Compact account dialog | <ul><li>All 11 states render 420 px wide and fit the window, including the five also captured at 1000×650 (`screenshots/account/`).</li><li>Switching, adding, Cancel sign-in, the sign-out question and errors with code all work.</li><li>Focus: open puts it on Close; the browser step on Open browser again; Cancel on Close; the question on Keep signed in.</li><li>Escape dismisses the question, then the dialog, and focus returns to the account button.</li><li>**Exception:** Use drops focus (finding 1).</li></ul> |
| 3. Professional shortcut table | <ul><li>One table of 29 entries. macOS reads ⇧⌘Z, ⌥⌘1, ⌘⌫ / ⌦; Windows reads Ctrl+Y or Ctrl+Shift+Z, Delete or Ctrl+Backspace.</li><li>Filter "hierarchy" gives 10 of 29; "ctrl shift" on Windows gives 1 of 29; no match shows the empty state.</li><li>Each key cell is named group, action and keys, as computed: "Hierarchy Delete Command Delete , or Forward Delete".</li><li>Focus is trapped. Escape clears the filter, then closes, and focus returns to the opener.</li></ul> |
| 4. Avatar-only account control | <ul><li>30×28 px in all 102 captures, showing at most one initial.</li><li>The state lives in the accessible name and tooltip.</li><li>The corner dot is amber while signing in and red on a problem.</li><li>It fits beside the macOS inset and beside the Windows controls at 1000×650.</li></ul> |
| 5. No ornament | <ul><li>Pixel-identical to the plain screenshots apart from the halo removal.</li><li>The union states use the same plain components: no tiles, glows, accent edges or pills.</li></ul> |

### Camera: the three real projections

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

### SpriteRenderer and cooked atlases as the union shows them

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

### Interaction probes

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

### Findings

Ranked by user impact. "Pre-existing" means the code is identical at ce06c31 and on the
sprite/atlas line.

1. **Account switch drops keyboard focus.** Pre-existing. Behaviour owned by Astra. **Fixed by Astra in aa8ed10; verified in the final revision above.**
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
2. **The import form refuses paths the union's importer accepts.** Behaviour owned by Astra. **Fixed by Astra in c131ed8, with the presentation reconciled and verified in the final revision above.**
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

### Native: not observed

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

### Commands and results

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

### Evidence

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

### Changed paths

- `handoffs/0035-editor-union-review/result.md`: this file.
- `handoffs/0035-editor-union-review/tools/`: the seven new tools listed in Method.
- `handoffs/0035-editor-union-review/screenshots/` and `evidence/`.

Nothing in `editor/`, `crates/`, `schemas/`, `docs/`, CI or PLAN changed. Not committed are the
raw sets (`u35-sweep`, `u35-repeat`, `r1`, `r2`, `s1`, `s2`, `smoke1`, `fix1`), the scratch
experiments and the logs, all in the ignored `artifacts/`.

### Requests for Astra

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

### Open questions

- Should a sprite authored without `sampling` or flips show "nearest" and "Off" with a Default
  tag (finding 7)? That would apply to every component's boolean and enum defaults.
- Should the sample fixture gain a pixel-perfect camera, now that its schema declares one?
- How should a frame-addressed Region read once frame names exist: the name only, or the name
  and ID?
