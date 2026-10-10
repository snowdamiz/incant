# 0033 full editor design pass: result

## Status

Browser review of revision 4 ("studio") is complete on the main-based branch. Native
review is pending Astra's rebuild.

- **Main-based integration (packet of 2026-10-10 16:55): done.** This branch is main
  (PR 49) plus my UI commits and Astra's behavioural fixes; it carries no pixel-perfect
  renderer.
  - The browser fixture now mirrors main's `schemas/Camera.schema.json` exactly:
    perspective and orthographic.
  - The sample's third camera is an authored perspective camera.
  - Pixel-perfect presentation is tested only against an explicitly synthetic schema.
  - The native parity tests were not weakened.
  - 498 tests pass in 28 files.
- **Whole-app redesign (director feedback, 2026-10-10 14:32): done in the browser.**
  Every surface was reviewed as one product and changed where it fell short. That
  covers the shell, navigation, hierarchy, viewport chrome, assets, Inspector, dock,
  agent and account, dialogs, menus, states and keyboard behaviour. The coverage matrix
  below names each surface, the change and its evidence. It also names what was kept,
  and why.
- **Cramped compound Collider: resolved.** The selected part is no longer a bordered
  card nested inside the list. Its fields continue at the component's own depth,
  primitive and size first and the stable ID last. Variant fields no longer indent, and
  rows are 28 px tall and 6 px apart. All editing, diagnostics and identity information
  is still shown.
- **Correctness follow-up (Astra's review, top of `brief.md`): done.**
  - Problems, Console and History mount at most one page of 100 rows.
  - Every list says which slice of how many it shows.
  - Copy ID announces a missing or denied clipboard.
- **Native End-then-Tab blank list: cause found and fixed in the UI.** The cause is a
  stale scroll measurement that waited for an animation frame. The fix still needs
  verification in a native rebuild; see request 1.
- **Pre-existing bug fixed: the window could scroll.** In revision 3, 64 of 96 browser
  states could scroll the whole document: by up to 1,235 px in ordinary states, and up
  to 30,392 px with 450 problems listed. Following a problem link could slide the
  titlebar off screen. The document now stays put; 0 of 96 states can scroll it.
- **Native acceptance: not claimed.** All evidence is headless Chrome renders of the
  built UI. Native captures from f78d763 informed the work; they show the previous
  design.
- **Performance gate: not claimed.** All timings were measured in a browser on an
  Apple M5 Pro. They are not M1 MacBook Air numbers and not phone numbers.

No phase gate is approved or claimed.

## Model and effort

- Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Thinking effort: Max, selected and verified by the runner for each session of this
  handoff, including this one. I cannot inspect the ACP transport from inside a
  session. No model or effort substitution occurred.

## Packet revisions acknowledged

| Revision | What it asked | What I did |
| --- | --- | --- |
| Main-based integration, 2026-10-10 16:55 | Match the sample fixture and parity tests to main's Camera schema; no unsupported projection in the sample; a clear split for pixel tests; keep the design | See the next section |
| Plan revisions 4–6 | Bounded large data; play kept out of authoring; hosted Device Farm phones | Every list stays bounded, the dock included; no Play or Keep Edit control; no device claim |
| Director feedback, 2026-10-10 14:32 | A proper design improvement through the whole app, not one sidebar; the shortcuts dialog is one example | Revision 4 across every surface (matrix below) |
| Native evidence from f78d763 | Investigate End then Tab on 64 parts; keep account-bearing captures local | Root cause found, fix and test; captures stayed ignored |
| Correctness follow-up (Astra) | Bound the dock's output rows without losing anything; Copy ID must not fail silently | Paged dock with explicit ranges; clipboard capability handled |

## Main-based integration

Source files and main's native schema take precedence over older screenshots and
pixel-branch evidence. Main's `schemas/Camera.schema.json` declares two projections:
perspective, and orthographic with `vertical_size`. This commit changes the following.

**Fixture and parity:**
- `bridge/fixture.ts` mirrors main's converted Camera schema exactly. The
  `pixel_perfect` variant is gone, and the projection description matches main.
- The parity test is unchanged and strict: the fixture's Camera schema must equal
  `snapshotFromEngine` over `schemas/Camera.schema.json`.
- The native-schema test now asserts exactly `perspective` and `orthographic`, with
  orthographic's unit and description.

**Sample cameras.** The pixel-perfect "Pixel Camera" became "Close-up Camera": the same
place in the tree, perspective written out explicitly, a 35° field of view. The sample's
cameras now show every state main supports:
- the Camera Rig shows the schema default ("Default" tag);
- the Map Camera shows orthographic, with field of view "Unused";
- the Close-up Camera shows authored perspective.

A new test checks every fixture variant for a projection kind the native schema lacks.

**Test split.**
- `CameraFields.test.tsx` covers main's schema only.
- Every pixel-perfect presentation test moved, unchanged in substance, to
  `PixelCameraFields.test.tsx`. It runs against main's native schema plus the variant
  that `handoff/0033-pixel-integration` produces, labelled **SYNTHETIC — NOT ON MAIN**.
  Its first test is a tripwire: it fails once main declares `pixel_perfect`, prompting a
  switch to the native schema, which the parity tests then cover.
- Shared helpers live in `src/test/cameraInspector.tsx`, outside the app's TypeScript
  project. Nothing pixel-related reaches the sample or the parity tests.

**One behaviour fix.** The field of view's "Unused" tag now only names projections the
component's schema declares (`FieldContext` carries the schema). On main, a
`pixel_perfect` value authored on the pixel branch shows "Unknown projection kind
"pixel_perfect". Expected perspective or orthographic; raw value:". The field of view
stays live and never claims a mode main lacks; a new test covers it.

**Agent note after Astra's bridge fix.** The bridge now sends "The agent is not available
in the editor yet." My presentation only recognised the old milestone wording, so the
native note had regressed to "Use your ChatGPT account" above that sentence. That
invited a sign-in that would not enable the agent. The UI now recognises both wordings:
it shows "Not in the editor yet", status "Unavailable", and keeps the real Continue with
ChatGPT action. A test reads the reason from the native bridge conversion itself.

**Tools and docs:**
- The capture states use the Close-up Camera, and there are new Camera Rig and Map
  Camera states.
- The review double's pixel-perfect camera is now a perspective close-up. Broken Camera
  still covers an unknown kind, and the later IDs are stable.
- The DESIGN.md diagram label changed.
- A presentation comment now points at the preserved branch for the pixel-perfect
  limits.

**Rendered review** (browser, ignored sets `i1` and `m2`):
- The Close-up Camera renders at 1440×874, 1000×650, 280 px and 1920×1080, and focused.
- The Map Camera at 280 px shows the Unused tag.
- Broken Camera's notice lists "perspective or orthographic".
- A full re-capture of all 96 states on this branch: 90 states have layout metrics
  identical to the pre-integration branch. The other 6 are 5 new camera states, and
  Broken Camera's notice, which is now one line shorter.
- 0 axe violations, step errors, page errors or window scroll; at most 102 dock rows.
- Astra's menu, reveal, field-ID and paging fixes keep every layout, and their tests
  pass.

**Out of scope here.** No pixel-perfect schema or renderer on main, no Rust, schema or
engine edits, no native capture, and no new redesign. Bridge transport pagination is
still open.

The director's screenshot (`artifacts/0033-native/director-cramped-sidebar.png`,
ignored) shows these problems, all addressed:
- Part card inside the list's box, with a third indent level for the primitive's
  fields.
- The ID as the first, input-like row.
- "Offset m" and "Rotation" stacked.
- Help text and the parts summary wrapping beside narrow labels.

## Coverage matrix

Paths are under `screenshots/whole-app/`. "Before" is revision 3 as committed at
e8ee0d6; "after" is revision 4 on this main-based branch. Both were captured with the same
96 browser states, data and sizes.

| Surface | What changed | Evidence (before/ and after/) |
| --- | --- | --- |
| Window frame | The app contains and clips its content, and every scroller is a containing block, so `scrollIntoView`, focus and hidden text can no longer scroll the window | `evidence/whole-app-comparison.json`: scrollable states 59 → 0 |
| Titlebar | Breadcrumb "Project / Scene" with the project as `h1`; History and panel toggles in two capsules; pressed toggles filled; the ChatGPT pill shows the account's initial with a presence ring | `no-selection-normal-1440x874.png` |
| Left column navigation | Hierarchy and Assets counts in the same pill as the dock's tab counts; the filter toolbar (search icon, match count) restored after a regression in this pass | same |
| Hierarchy | Scene row at heading weight; the selected row adds a 2 px accent edge to the indigo pill; new context menu, described below | `context-menu-normal-1440x874.png` (after only; there was no menu) |
| Hierarchy context menu | See the Hierarchy context menu section below | same |
| Viewport chrome | Status pill with tone dot (Live, Not attached, Waiting, Error, No engine); the empty state is a drawn ground grid and outlined cube, dashed red on error | `no-selection-…`, `project-error-normal-1440x874.png` |
| Inspector: rhythm | 28 px values 6 px apart; 34 px sticky headers; caption groups 10 px apart; 16 px of air below each component; default width 360 px (was 331) | `camera-rig-normal-1440x874.png`, `map-camera-narrow-1440x874-side.png`, `transform-camera-normal-1440x874-side.png` (after only: the Close-up Camera) |
| Inspector: identity | Component marks: icon on a 22 px tile tinted by family (physics mint, light amber, camera sky, audio pink, script orange, navigation lime; transform neutral) | `native-character-normal-1440x874-side.png`, `native-navmesh-normal-1440x874-side.png` |
| Inspector: problems | One card tinted by the worst severity. Locations read "Collider /path" in quiet text instead of periwinkle links, and paths wrap after slashes | `railing-compound-normal-1440x874-side.png`, `native-malformed-normal-1440x874-side.png` |
| Inspector: compound Collider | See the Compound Collider section below | `native-rubble-parts-{normal,narrow,minimum}-*-inspector.png`, `railing-parts-normal-1440x874-inspector.png` |
| Inspector: notices | Unknown fields, missing schemas and version mismatches use the same dashed outline as field notices, not a solid yellow block | `native-malformed-normal-1440x874-side.png` |
| Asset workspace and details | Section rhythm matches the Inspector; the source path sits on the raised value fill; the chosen interpretation is the selection fill; tab count pills | `asset-texture-normal-1440x874.png` |
| Problems | See the dock views section below | `no-selection-…`, `dock-paged-problems-normal-1440x874.png` |
| Console | See the dock views section below | `dock-console-normal-1440x874.png`, `dock-paged-console-minimum-1000x650.png` |
| History | See the dock views section below | `dock-history-normal-1440x874.png`, `dock-paged-history-normal-1440x874.png` |
| Status line | Shows only the problem counts that exist, or a green check and "No problems" once validated. Shows nothing while opening, or after a failure without diagnostics. Key hints are caps | any full capture; `empty-project-normal-1440x874.png` |
| Agent pane | The native bridge's "not in the editor" reason (plain words since Astra's fix, a milestone before) gets its own state: "Not in the editor yet", status Unavailable (presentation only). The no-engine note no longer repeats its title | `native-character-normal-1440x874-side.png`, `no-engine-normal-1440x874.png` |
| ChatGPT account | The state is a card with a tinted badge; same header and paddings as every dialog | `account-signed-in-normal-1440x874.png` |
| Keyboard shortcuts | See the shortcuts reference section below | `shortcuts-dialog-normal-1440x874.png`, `shortcuts-filter-minimum-1000x650.png` (after only) |
| Empty, loading and error states | Pictograms on tone-tinted badges (a pointer for "Nothing selected", the scene mark while no project is open, a check for "No problems"); skeleton rows shaped like tree rows | `project-loading-…`, `project-error-…`, `empty-project-…`, `no-engine-…` |
| Fatal screens | A card with the product mark, an error tile and the problem as the page's `h1` (axe flagged both before). Protocols are compared side by side and fixture names are chips | `bad-fixture-normal-1440x874.png`, `incompatible-normal-1440x874.png` |
| Minimum size and keyboard | Container queries stack vectors, notices and stable IDs below 300 px; the parts summary wraps to its own line; scrollable dock lists are keyboard stops; F6, `?` and ⌘/ keep working | `camera-rig-minimum-1000x650.png`, `native-rubble-parts-minimum-1000x650-inspector.png` |
| Type scale | 12 px minimum everywhere; the 11 px key hints, level badges and menu shortcuts are gone | — |

### Hierarchy context menu

- The menu opens from a right-click, Shift+F10 or the Menu key.
- It offers Rename (F2), Copy ID and Delete (Del, through the command bus).
- An unavailable item stays focusable and explains why when chosen.

### Compound Collider

The compound Collider changes resolve the director's screenshot:
- **Parts header.** "Parts" and the breakdown span the component; the breakdown moves
  to its own line at the minimum width.
- **Part rows.** Each row gets a primitive glyph (box, sphere or capsule).
- **Selected part.** It opens under a hairline caption, "Part 42 of 64
  /shape/parts/41", instead of a bordered card. Its fields follow at the component's
  depth: Primitive, its size, Offset, Rotation, then ID.
- **ID.** The ID is quiet metadata, outlined and muted, and stacks to show all 26
  characters at 280 px.
- **Errors.** A non-unit quaternion outlines the whole vector once, instead of drawing
  a red grid.
- **Variant help.** Variant help such as "One body and material shared…" sits in the
  value column.

### Dock views

**Problems:**
- Grouped by entity, worst first, with severity filter chips and counts.
- Paged at 100 rows. A group that spans pages repeats its heading with a "continued"
  tag, keeps its whole-group counts, and says "N more … on the next pages".
- Rows reveal their entity.

**Console:**
- A table of time, level badge, source and message, with level filters.
- Paged; it follows the newest page as messages arrive, and **Newest** returns there.
- The list is a keyboard stop.

**History:**
- A timeline with origin-coloured dots, origin chips and a fixed time column; times
  no longer truncate.
- The "Now" marker sits between applied and undone edits.
- Paged; it follows Now, and **Now** returns there.
- The list is a keyboard stop (axe flagged it before).

### Shortcuts reference

The plain list became a searchable reference:
- **Header.** A tinted keyboard mark and a subtitle that names the platform.
- **Filter.** Focused on open. It matches actions, groups and key names, and shows
  "N of M". Escape clears it first, then closes the dialog.
- **Layout.** Five category cards in two balanced columns, one column when narrow.
- **Keys.** Platform keys as individual caps (⌘ ⇧ ⌥ on macOS, names elsewhere), with
  "or" between alternatives and "/" between paired directions.
- **Coverage.** All 29 entries are implemented keys. I verified each against the
  handlers.

### Deliberately retained

- **Native titlebar inset and traffic-light alignment.** These are `TITLEBAR.md` host
  contracts. Only a native capture can verify them.
- **Asset list structure.** The windowed, grouped listbox from revision 3 already meets
  the bounded-rendering rules. It now uses the shared counts, search field and colours.
- **Navigation grid map and value table.** Revision 3 layout, inside the new rhythm.
- **Import form.** Structure unchanged; it inherits the new type, spacing and dialog
  vocabulary. With the read-only fixture it only shows its unavailable state.
- **Application menu (File/Edit/View on Windows/Linux).** Still none; this remains an
  open question in DESIGN.md, not something to fake.

## Correctness follow-up

**Bounded output lists.** `src/components/dock/paging.tsx` holds `PAGE_SIZE` (100),
`usePage` and `Pager`. Each view mounts only its page and renders the full text of every
row on it:
- **No silent truncation.** The pager always shows "101–200 of 20,000". Filter chips
  count every entry. A list that fits on one page shows no pager.
- **Keyboard.** First, previous, next and last buttons stay focusable at the ends
  (`aria-disabled`). A page change is announced on the status line and scrolls to the
  page's top.
- **Updates.**
  - The Console follows the newest page while new messages arrive; another page stays
    put and its total grows.
  - History follows the page that holds Now.
  - A list that shrinks clamps to its last page.
  - A filter change returns to the first page.
- **Kept behaviour.** Grouping, entity reveal, applied and undone state, and the Now
  marker are unchanged.

Browser measurements on this main-based branch, with Astra's paging fixes
(`evidence/dock-scale-report.json`, Chrome 155, M5 Pro):

| Scenario | Rows mounted | Unthrottled | 4× CPU |
| --- | --- | --- | --- |
| 20,000 problems on 10,000 entities: load to first page (from navigation start) | 100 | 146 ms | 577 ms |
| Next page / last page | 100 | 11.3 / 13.6 ms | 48.5 / 57.9 ms |
| Hide errors (4,000 left) | 100 | 16.2 ms | 73.7 ms |
| 20,000 problems in one group: next / last page | 100 | 8.0 / 7.1 ms | 39.6 / 40.6 ms |
| Open Console with 100,000 messages (newest page) | 100 | 9.0 ms | 41.5 ms |
| 10 × 500 messages arriving while following | 100 each time | 5.9–8.1 ms each | 26.8–33.4 ms each |
| Open History with 50,000 edits (page of Now) | 100 + Now marker | 4.2 ms | 20.7 ms |
| History first page, then Now | 100 | 3.3 / 2.7 ms | 22.7 / 14.9 ms |

No long tasks occurred in the arrival steps. In the browser captures, the dock mounted up
to 1,234 rows before and at most 102 after (100 rows plus 2 group items).

**Copy ID.** `src/shell/clipboard.ts` checks for `navigator.clipboard.writeText`:
- **Without it,** the hierarchy menu's Copy ID and the Inspector's copy button are
  marked unavailable. They stay focusable and explain the reason ("this editor has no
  clipboard access. Select the ID in the Inspector to copy it").
- **A refused write** announces "Could not copy: clipboard access was denied."
- **Tests** cover the absent API, a rejected permission and a successful copy of the
  exact ID.

## Native End-then-Tab investigation

**Observation (`compound-end-wide.jpg`, AX text).** After End, the detail correctly
showed Part 64, but the list was blank above the pinned last row. Accessibility listed
Parts 1–13 and Part 64.

**Cause.** `useViewport` re-measured the scroller only on an animation frame after a
scroll event. A programmatic reveal (End) scrolled the list, but the window state kept
the old top until a frame arrived. WebKit throttles or pauses frames for hidden,
occluded or non-frontmost views, which is likely during computer-use automation. The
UI therefore kept rows 1–13 mounted (the old window) plus the pinned selection, at the
bottom of a scrolled list.

**Fix (in this commit, with Astra's e8ee0d6 changes kept):**
- `useViewport` returns a synchronous `remeasure`. The tree, the asset list and record
  lists call it right after `revealSpan`, so the target's neighbours mount in the same
  render as the selection.
- Scroll and resize notifications are coalesced on a frame with a 100 ms timer
  fallback.
- Keyboard focus moves after React commits (a layout effect), not on a frame.
- `ThrottledFrames.test.tsx` reproduces the failure with frames disabled: it fails on
  the old code and passes with the fix.

`compound-min.jpg` shows Part 1 and is not a last-row test.

## Measurements

All captures use headless Chrome 155.0.8059.40 at device scale 2. The machine is a
MacBook Pro with an Apple M5 Pro (15 cores), 48 GB and macOS 26.6. Per-state data is in
`evidence/whole-app-comparison.json`.

| Measure, all 96 states | Before (rev. 3) | After (rev. 4, main-based) |
| --- | --- | --- |
| Axe violations, colour contrast included | 6 | 0 |
| States where the window can scroll | 64 (up to 1,235 px; 30,392 px with 450 problems) | 0 |
| Most rows mounted in the dock | 1,234 | 102 |
| Clipped values | 37 (0 numeric) | 36 (0 numeric) |
| Elements outside the Inspector | 0 | 0 |
| Page errors and remote requests | 0 | 0 |
| Step errors | 4 | 0 |

The six "before" axe violations:
- Two fatal screens had no `h1`.
- Four long consoles or histories were not keyboard-scrollable.

The four "before" step errors are pager clicks in the paged states; revision 3 had no
pager. The 36 clipped values are long asset paths and IDs in reference boxes. Each ends
in an ellipsis, keeps the full value in its tooltip, and shows the resolved name first
when one exists.

| Inspector | Before | After |
| --- | --- | --- |
| Default width at 1440×874 | 331 px | 360 px |
| Value boxes, component headers | 26 px, 30 px | 28 px, 34 px |
| Gap between rows | 4 px | 6 px |
| Labels, values (contrast on the side column) | 7.98:1, 13.83:1 | unchanged |
| Keyboard focus ring | solid 2 px `#c3c8ff` | unchanged |

### 100,000 entities (browser, not the device gate)

Re-measured on this main-based branch (`evidence/scale-report.json`).

| Interaction | Unthrottled | 4× CPU |
| --- | --- | --- |
| Rows mounted for 100,101 hierarchy nodes; options for 20,000 assets | 34; 37 | 34; 37 |
| Load to first tree (fixture generated in the page) | 182 ms | 727 ms |
| Filter matching every entity / one group | 67 / 7.0 ms | 217 / 34 ms |
| Arrow Down, median and worst of 30 | 0.8 / 24 ms | 4.3 / 17.9 ms |
| End to the last of 1,000 rows | 0.7 ms | 4.5 ms |
| Assets open / End to the last of 20,000 | 9.9 / 5.0 ms | 49.4 / 23.7 ms |
| Select the 65,536-value grid (2 inputs) / open its table (360 cells) | 12.1 / 5.2 ms | 53.6 / 19.6 ms |
| Rename through the test double, total | 51.2 ms | 261 ms |
| … of which the double rebuilds the snapshot | 40.9 ms | 214.6 ms |

Long tasks:
- The first filter over all names is one 95 ms task unthrottled, 424 ms at 4×.
- Every rename at 4× is a task of about 250 ms, almost all of it the double's snapshot
  rebuild.

## Changed paths

This integration commit:
- `editor/ui/src/bridge/fixture.ts`: the Camera schema mirrors main; the Close-up Camera
  replaces the pixel-perfect sample camera.
- `editor/ui/src/components/inspector/presentation.ts`, `FieldView.tsx` and
  `InspectorPanel.tsx`: a schema-aware field-of-view tag, and the component schema in
  `FieldContext`.
- `editor/ui/src/components/AgentPanel.tsx` and `AgentPanel.test.tsx`: the plain native
  reason is recognised, tested against the bridge conversion.
- `editor/ui/src/components/inspector/CameraFields.test.tsx` (main's schema) and
  `PixelCameraFields.test.tsx` (new, synthetic schema).
- `editor/ui/src/test/cameraInspector.tsx` (new shared helpers).
- `editor/ui/DESIGN.md`: the diagram label.
- `handoffs/0033-editor-design-review/`:
  - tools `capture.mjs`, `review-double.mjs` and `curate-whole-app.mjs`;
  - re-curated `screenshots/whole-app/`;
  - `evidence/whole-app-comparison.json`, `whole-app.sha256`, `dock-scale-report.json`
    and `scale-report.json`;
  - this result.

The whole-app pass (earlier commits on this branch). UI code (`editor/ui/src`):
- **Shell:**
  - `App.tsx`: status line and fatal screens.
  - `components/Titlebar.tsx` and `ProviderChip.tsx`.
  - `components/ViewportPanel.tsx`.
  - `components/StateView.tsx`.
  - `components/Menu.tsx` (new).
  - `components/breakAfterSlashes.tsx` (new).
  - `shell/layout.ts`: the 360 px Inspector default.
  - `shell/clipboard.ts` (new).
  - `shell/windowing.ts`: synchronous `remeasure` and the timer fallback.
- **Hierarchy and assets:** `components/HierarchyPanel.tsx` (context menu, scene rows,
  remeasure, Copy ID) and `components/assets/AssetBrowser.tsx` (remeasure).
- **Inspector** (`components/inspector/`):
  - `InspectorPanel.tsx`: problems card, family marks, pictograms, Copy ID.
  - `FieldView.tsx`: flat variants, help text, variant glyphs, whole-vector errors.
  - `ObjectList.tsx`: flat detail, glyphs, remeasure, layout-effect focus.
  - `presentation.ts`: families, shape glyphs, part order.
- **Dock:** `components/BottomDock.tsx` keeps only the tabs. New in `components/dock/`:
  `ProblemsView.tsx`, `ConsoleView.tsx`, `HistoryView.tsx`, `paging.tsx`, `time.ts`.
- **Other components:**
  - `components/ShortcutsDialog.tsx` (rewritten).
  - `components/AgentPanel.tsx` (milestone wording).
  - `icons/Icon.tsx`: pointer, shape and pager icons.
- **Styles:** `tokens.css` (field and header heights, family tints), `base.css`,
  `shell.css`, `hierarchy.css`, `dock.css`, `inspector.css`, `dialogs.css`,
  `assets.css`, `agent.css`.
- **Documents:** `editor/ui/DESIGN.md`, revision 4.

Tests (`editor/ui/src`):
- New:
  - `components/DockPaging.test.tsx`: 6 tests, covering many groups, one huge group,
    filter and update transitions, a console with arrivals, and history with Now.
  - `components/Clipboard.test.tsx`: 4 tests.
  - `components/ShortcutsDialog.test.tsx`: 4 tests.
  - `components/Dock.test.tsx`: 4 tests.
  - `components/HierarchyMenu.test.tsx`: 3 tests.
  - `components/ThrottledFrames.test.tsx`: 2 tests.
- Extended:
  - `App.test.tsx`: fatal screens, status line, filter focus, the execCommand guard.
  - `AgentPanel.test.tsx`: milestone wording.
- Adapted to intentional changes:
  - `CompoundParts.test.tsx`: part order, and no nested boxes.
  - `ProblemsList.test.tsx`: grouped rows.
  - `ProjectStates.test.tsx`: the "Live" pill.

Handoff (`handoffs/0033-editor-design-review/`):
- **Tools:**
  - `tools/capture.mjs`: 96 states, `--root` for the before build, and window-scroll
    and dock-row measures.
  - `tools/curate-whole-app.mjs` (new).
  - `tools/measure-dock.mjs` (new).
- **Screenshots:** `screenshots/whole-app/before/` (26) and `after/` (29). These are
  unedited, account-free PNGs from the sample fixture, the synthetic provider fixtures
  and the evidence test double.
- **Evidence:**
  - `whole-app-comparison.json`;
  - `whole-app.sha256` (55 OK);
  - `dock-scale-report.json`;
  - `scale-report.json`, re-measured.
- `brief.md`: the runner's packet updates.
- **Unchanged from b7ba9bb:** the earlier pass's evidence (`screenshots/before/`,
  `screenshots/after/`, `evidence/browser-comparison.json`, `evidence/screenshots.sha256`).

Not committed (ignored `artifacts/`):
- `0033-native/`: the native captures, which show the account.
- Every raw browser set:
  - before: `v3-before`, `v3-before-paged` and `v3-before-cams`;
  - after: `m2`, on this branch;
  - pre-integration and iteration sets: `v4-final2`, `m1`, `i1` and `w1`–`w10`;
  - measurements: `dock-m1` and `scale-m1`;
  - the e8ee0d6 source extract `src-e8ee0d6`.

No Rust, SDK, CI, schema, transport, auth storage or account setting changed. No Cargo
command ran.

## Commands and results

```sh
npm ci --ignore-scripts                     # workspace root
cd editor/ui
npx tsc -b --noEmit                         # strict: 0 errors
npx vitest run                              # 28 files, 498 tests passed (main-based branch)
npm run build                               # built
cd ../..
# before: revision 3 from e8ee0d6, extracted into the ignored artifacts directory
git archive e8ee0d6 editor/ui editor/bridge | tar -x -C artifacts/0033-editor-design-review/src-e8ee0d6
(cd artifacts/0033-editor-design-review/src-e8ee0d6/editor/ui && npx vite build)
node handoffs/0033-editor-design-review/tools/capture.mjs v3-before --root=artifacts/0033-editor-design-review/src-e8ee0d6/editor/ui
node handoffs/0033-editor-design-review/tools/capture.mjs v3-before-paged --root=… --only=dock-paged-problems,dock-paged-console,dock-paged-history
node handoffs/0033-editor-design-review/tools/capture.mjs v3-before-cams --root=… --only=camera-rig,map-camera
node handoffs/0033-editor-design-review/tools/capture.mjs m2              # 96 captures, 0 step errors, 0 axe
node handoffs/0033-editor-design-review/tools/curate-whole-app.mjs v3-before,v3-before-paged,v3-before-cams m2
(cd handoffs/0033-editor-design-review && shasum -a 256 -c evidence/whole-app.sha256)   # 55 OK
node handoffs/0033-editor-design-review/tools/measure-dock.mjs m1         # no page errors
node handoffs/0033-editor-design-review/tools/measure-scale.mjs m1        # no page errors
```

Every capture server used the first free port from 4233 and was closed afterwards.
- **Not run:** Cargo, Clippy, GPU tests, native capture, native WebKit, phones.
- **Native:** I took no native captures and drove no native UI in this session. The
  f78d763 captures were read from the ignored directory only.

## Requests for Astra

1. **Rebuild and recheck End then Tab natively.**
   - Steps: in the 64-part compound, select Part 1, press End, then Tab.
   - Expected: the trailing rows are visible with no blank region, and AX lists about
     Parts 52–64 instead of 1–13 plus 64.
   - Repeat while the window is not frontmost, and again after 100 ms.
   - Also follow a Problems link to a deep part path: the titlebar must not move.
2. **Window title: done by Astra** (`17bf782`, "Incant").
3. **Agent reason copy: done by Astra** (`17bf782`). The UI recognises the new plain
   wording; `AgentPanel.test.tsx` reads it from the bridge conversion. Please keep that
   test in mind if the sentence changes again.
4. **Output lists in snapshots: still open,** as Astra noted. The UI pages Problems,
   Console and History, but every snapshot still carries every console message and
   history entry. Consider capping or paging them at the bridge, with totals, as for
   entity details.
5. **When pixel-perfect lands on main,** switch `PixelCameraFields.test.tsx` to the
   native schema, as its tripwire test will prompt, and add a pixel camera back to the
   sample.
6. **Carried over from the earlier pass:**
   - entity kinds for lights and audio;
   - nullable `type` arrays;
   - nested-object order, units and human titles in the schema registry;
   - snapshot reuse at 100,000 entities.
7. **Native captures after the rebuild, for my rendered review:**
   - Rubble Pile compound at 1440×874 and 1000×650;
   - the Close-up and Map cameras;
   - the shortcuts dialog;
   - the hierarchy context menu;
   - a long console;
   - the titlebar.

   I will commit only account-free crops.

## Limitations

- **Browser-only evidence.** Native WebKit rendering, the titlebar and the viewport were
  not reviewed in this revision.
- **Not the performance gate.** All numbers come from one fast desktop machine and
  headless Chrome; 4× throttling is not an M1 or a phone. The rename and dock doubles
  stand in for the engine and bridge.
- **First filter long task.** Filtering 100,000 names is still one long task: 108 ms
  unthrottled, 467 ms at 4×.
- **Read-only Inspector.** Every field stays read-only until the bridge offers a
  component command.
- **Pixel-perfect only synthetic here.** On this branch the pixel-perfect presentation
  runs only against a synthetic schema. The real schema, renderer and evidence remain on
  `handoff/0033-pixel-integration`.
- **Pages, not continuous scrolling.** The dock pages because rows wrap to full text. A
  very long single message still renders in full on its page.
- **Capsule glyph.** At 12 px it is a small solid; the tag text beside it carries the
  meaning.

## Open questions

- Should the dock remember a chosen page per tab across tab switches? Today each tab
  returns to its anchor (first page, newest messages, or Now).
- Should a short window keep the agent collapsed, or should the user's choice persist
  across sessions?
- Should component display names and families come from engine schema metadata
  (request 5)?
