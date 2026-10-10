# 0033 full editor design pass: result

## Status

Browser review complete. Native review is pending Astra's integration and captures.

- **Inspector redesign: done in the browser.** The selection's identity moves into the
  panel's header line, components get compact sticky headers, and rows align across
  nesting levels. No numeric value is clipped at 1440×874, at 1000×650, or with the
  Inspector at its 280 px minimum.
- **Whole-app pass: done in the browser.** I reviewed the agent pane, viewport and dock
  copy, kind colours, dialogs and every empty, loading and error state, and changed
  them where needed.
- **Bounded large data (PLAN.md revisions 4 and 5): done in the browser.** The
  hierarchy, the asset list and the Inspector's lists mount only the rows in view. The
  65,536-value navigation grid that stalled the old Inspector now mounts no control
  per cell.
- **Native acceptance: not claimed.** All captures are headless Chrome renders of the
  built UI. Astra builds the native app and supplies native captures for my final
  rendered-pixel review.
- **Performance gate: not claimed.** The 100,000-entity numbers below come from a
  browser on an Apple M5 Pro. They are not the M1 MacBook Air device gate, and they
  are not phone measurements.

No phase gate is approved or claimed.

## Model and effort

- Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Thinking effort: Max. The runner selected and verified it for each of the three
  sessions of this handoff; two plan-revision interrupts started the second and third.
- I cannot inspect the ACP transport from inside a session. No model or effort
  substitution occurred.

## Plan revisions and director feedback acknowledged

The packet changed during the work. I read each revision before continuing and kept
all earlier edits and captures.

| Revision | What changed for this pass | What I did |
| --- | --- | --- |
| 4 (`54dcf1d`) | Authoring stays on the command bus; play runs in a separate native world. Large Inspector arrays, hierarchy rows and asset collections must be bounded. | Windowed every long collection, measured 100,000 entities, and added no Play or Keep Edit control. |
| 5 (`da0ed53`) | All 27 Section 12 decisions are settled; ADR 0009 lists virtualization as required work. | Nothing in the UI implies an unimplemented runtime or browser capability. |
| 6 (`c52a37d`) | Hosted AWS Device Farm phones; store accounts and signing move to Phase 5. | Recorded the browser and hardware for every number; no phone or device claim. |

The director's feedback in this packet: the right sidebar is "messy, cluttered, and
sloppy, header too tall with bad layout", with full creative freedom over the app.
That drove every change below.

## Design

### Inspector

- **Identity in the header line.** The panel's 36 px header names what is inspected: a
  22 px kind tile, the name, and Copy entity ID. The separate 58 px identity block is
  gone. "Inspector" stays the panel's accessible name and returns as the visible title
  when nothing is selected.
- **One meta strip.** Kind, ID and the read-only chip share one 30 px line. The ID is
  middle-truncated in CSS, so both ends stay visible and the DOM keeps every character
  for copying. The chip now says "component values can be inspected here but not
  edited yet" instead of naming the engine bus and the bridge.
- **Problems block.** Entity problems are one rounded block of rows. Each link wraps at
  slashes and keeps the tested accessible name, such as `Collider/shape/parts/3/id`.
- **Component headers.** Headers are 30 px instead of 36 px and stick while their rows
  scroll. They show an icon per component type and a display name ("Rigid Body",
  "Navigation Mesh"). The raw type ID moved from every header into the tooltip and the
  section's accessible description.
- **Aligned columns.** The label column is `clamp(88px, 32cqi, 168px)` of the
  component's width. Every nested group indents 12 px and narrows its label column by
  12 px, so all value columns line up.
- **Quiet values.** Values sit in raised boxes without borders, instead of black wells
  with hover borders. Units follow their number ("0.1 m"), not the far edge of the box.
- **Segmented vectors.** A vector is one box of axis cells. A quaternion wraps to 2 × 2
  when its value column is under 340 px. Below 300 px of component width, vectors and
  notices stack under their label.
- **Lights.** Linear RGB colours lead with an sRGB swatch.
- **References.** An ID that exactly matches a project asset or entity shows that
  thing's icon and name ("crate_stack", "Player Character") before the authored ID.
- **Script props.** Open objects list each key as authored, instead of an
  "Unsupported field type object" notice.
- **Long lists.** Navigation grid costs draw as a pixel map with a legend, and their
  exact values open in a bounded table. Other long number lists use the table
  directly. Material ID lists are a bounded list. Compound parts, navigation sources
  and off-mesh links keep the master/detail list, with profile summaries such as
  "4, 0, 2 ↔ 4, 1.2, 4.5".
- **Native profiles.** Navigation, audio and light components get readable order and
  labels where the schema has none: build settings in struct order, the size as W and H,
  and Playback and Mix captions.

### Rest of the app

- **Agent pane.** The header is a disclosure with a status ("Not signed in", "Ready",
  and so on). While the agent cannot run, the pane is one short note and its next step,
  instead of a 200–240 px empty state with a disabled composer. A window under 720 px
  tall starts collapsed. A ready agent keeps the resizable transcript and composer.
- **Viewport.** "Native viewport not attached" is now "Viewport not attached", and the
  redundant note "Nothing in this area is rendered by the engine" is gone. The fixture
  text no longer mentions wgpu, and the console sample no longer mentions Spike 1.
  Render errors get an error tile.
- **Dock.** In Problems rows the entity name reads as a name, and the component and
  path read as code. The active tab icon is neutral; only its underline uses the accent.
- **Colour.** Meshes and models use a neutral tint, so long hierarchies read calm.
  Scripts moved from violet to a warm orange, and the dialog scrim lost its violet tint.
- **Status messages.** "Rename is unavailable: the connected bridge does not support
  entity.rename" is now "Rename is unavailable in this editor build (entity.rename)".
- **Styles.** The 3,039-line `app.css` is replaced by seven focused files (base, shell,
  hierarchy, dock, inspector, agent, dialogs), plus the updated `assets.css`.
- **Design document.** `editor/ui/DESIGN.md` is rewritten as revision 3.

### Bounded rendering

`src/shell/windowing.ts` tracks a scroller's viewport. It updates only when the offset
crosses a step, and each list mounts its visible rows plus overscan:

| Collection | Always mounted | Keyboard |
| --- | --- | --- |
| Hierarchy, a flat ARIA tree with level, position and set size | active row, renaming row | arrows, Home/End, Page Up/Down, type-ahead |
| Assets, a listbox of labelled folder groups | each visible folder label, active row | arrows, Home/End, Page Up/Down |
| Record lists (parts, sources, links) | selected item | arrows, Home/End, Page Up/Down |
| Value table, rows and grid columns | — | scroll keys in the focused box |

Keyboard moves to an unmounted item reveal it and then focus it. A rename stays mounted
while its row scrolls away; Enter commits through the command bus and Escape cancels.
The hierarchy filter applies through `useDeferredValue`, and lower-cased names are
computed once per tree.

## Measurements

All captures use headless Chrome 155.0.8059.40 at device scale 2. The machine is a
MacBook Pro (Mac17,9) with an Apple M5 Pro (15 cores), 48 GB and macOS 26.6. Full
per-capture data: `evidence/browser-comparison.json`.

### Layout and accessibility

| Measure | Before | After |
| --- | --- | --- |
| Captures | 56 | 63 |
| Axe violations, colour contrast included | 0 | 0 |
| Elements outside the Inspector box | 60 | 0 |
| Clipped numeric values | 39 | 0 |
| Clipped values of any kind | 72 | 32 |
| Page errors and remote requests | 0 | 0 |

The 32 remaining clipped values are long asset paths and IDs in reference boxes. Each
ends in an ellipsis, keeps the full value in its tooltip, and shows the resolved name
first when one exists.

| State | Inspector top to first value | Agent pane height |
| --- | --- | --- |
| 1440×874, 331 px Inspector | 138 → 99 px | 210 → 141 px |
| 1000×650, 280 px Inspector | 138 → 118 px | 200 → 37 px |
| 1440×874, 280 px Inspector | 138 → 118 px | 210 → 158 px |

The 118 px cases are taller only because vectors stack under their labels instead of
clipping.

| Grid | Before | After |
| --- | --- | --- |
| 24 × 16 costs (384 values) | an empty-looking box 1,361 px wider than the Inspector | 0 overflow; map and legend |
| 256 × 256 costs (65,536 values) | 65,536 inputs; 261,983 px overflow; the first capture run stalled | 2 inputs in the Inspector; 345 table cells when opened |

Other checks:
- **Focus.** A keyboard-focused value box draws a solid 2 px `#c3c8ff` ring.
- **Contrast.** Labels measure 7.98:1 and values 13.83:1 on the side column.
- **Reduced motion.** Status dots stop pulsing.

### 100,000 entities (browser, not the device gate)

Interaction times run from the input to React's commit (DOM updated, effects run). The
next paint follows within one frame. "4×" uses Chrome's CPU throttling as a rough
stand-in for slower hardware; it is not an M1 measurement.
Data: `evidence/scale-report.json`.

| Interaction | Unthrottled | 4× CPU throttle |
| --- | --- | --- |
| Rows mounted for 100,101 hierarchy nodes | 34 | 34 |
| Options mounted for 20,000 assets | 35 | 35 |
| Load to first tree, including generating the fixture in the page | 206 ms | 678 ms |
| Filter matching every entity (first keystroke) | 47 ms | 210 ms |
| Filter matching one group of 1,000 | 9 ms | 28 ms |
| Arrow Down, median of 30 | 0.7 ms | 3.0 ms |
| Arrow Down, worst of 30 | 27.2 ms | 15.2 ms |
| End to the last of 1,000 rows | 0.7 ms | 3.1 ms |
| Assets tab open | 14.7 ms | 38.6 ms |
| End to the last of 20,000 assets | 4.3 ms | 13.8 ms |
| Select the 65,536-value grid entity | 11.8 ms | 47.9 ms |
| Open its value table | 3.4 ms | 14.2 ms |
| Rename through a test double, total | 48.5 ms | 205.3 ms |
| of which the double copies the 100,000-entity snapshot | 38.3 ms | 162.2 ms |

Long tasks (50 ms or more):
- The first filter over all 100,000 names takes one 113 ms task unthrottled and
  392 ms at 4×. Typing stays responsive because the input commits first.
- At 4×, every rename has a task of about 200 ms, almost all of it the test double's
  snapshot copy.

The UI side of a rename is about 10 ms unthrottled and 43 ms at 4×. A real bridge that
rebuilds the whole snapshot per edit will dominate the 100 ms budget; see the requests.

## Changed paths

UI code (`editor/ui/src`):
- `components/inspector/InspectorPanel.tsx`: identity header, meta strip, problems
  block, component sections, reference resolution.
- `components/inspector/FieldView.tsx`: rows, groups, value boxes, vectors,
  references, open objects, notices.
- `components/inspector/ObjectList.tsx`: windowed record lists, profile summaries,
  aligned detail card.
- New in `components/inspector`:
  - `ValueLists.tsx`: number tables, grid map, string lists;
  - `IdentityHeader.tsx`;
  - `format.ts`: shared text helpers, re-exported from FieldView.
- `components/inspector/presentation.ts`: display names, icons, native profiles,
  summaries, grid hint, caption rule.
- `components/AgentPanel.tsx` and `App.tsx`: collapsible agent pane and its layout.
- `components/HierarchyPanel.tsx`: windowed tree, Page Up/Down, deferred filter.
- `components/assets/AssetBrowser.tsx`: windowed, grouped listbox.
- `components/assets/AssetInspector.tsx`: header-line identity, panel-level Escape.
- `components/MatchCount.tsx` (new): compact counts from 10,000.
- `components/ViewportPanel.tsx`, `BottomDock.tsx`, `ShortcutsDialog.tsx`: copy and
  Problems rows.
- `shell/windowing.ts` (new) and `shell/layout.ts`: viewport tracking; the old
  compact-agent sizing is removed.
- `hierarchy/tree.ts`: cached lower-cased names for filtering.
- `icons/Icon.tsx`: 13 component icons.
- `bridge/fixture.ts`: the 100,000-entity `stress` variant and fixture copy.
- `bridge/resolve.ts`: plain-language unavailable messages.
- Styles: `tokens.css`, `base.css`, `shell.css`, `hierarchy.css`, `dock.css`,
  `inspector.css`, `agent.css`, `dialogs.css`, `assets.css`. `app.css` is deleted.
- `main.tsx`: the stylesheet imports.

Tests in `editor/ui/src`:
- `components/Windowing.test.tsx` (new, 11 tests):
  - windowing helpers;
  - the 100,000-entity tree: mounted counts, End and Page Down, selection kept while
    scrolling, rename commit and Escape cancel after window changes;
  - the 20,000-asset list;
  - the 65,536-value grid, its exact-cell diagnostic, and a size mismatch.
- Adapted to intentional design changes:
  - `App.test.tsx` and `AgentPanel.test.tsx`: the agent behaviour is replaced with
    equivalent assertions. The pane sizes to its note, collapses to its status, and a
    ready agent stays resizable. No composer is offered while the agent cannot run,
    and the composer is disabled with a reason while it works.
  - `CompoundParts.test.tsx`: the 64-part list asserts the bounded window, the set
    size, the pinned selection, and End reaching part 64.
  - `PhysicsFields.test.tsx`: display names.
  - `bridge/resolve.test.ts`: message wording.
  - `shell/layout.test.ts`: the removed compact-agent height cases. Their behaviour is
    now covered in `App.test.tsx`.

Documents: `editor/ui/DESIGN.md`, revision 3.

Handoff (`handoffs/0033-editor-design-review/`):
- `tools/review-double.mjs`: native-shaped evidence data through the real
  `snapshotFromEngine`.
- `tools/capture.mjs`: browser captures and DOM-generic measurements.
- `tools/measure-scale.mjs`: the 100,000-entity measurements.
- `tools/curate.mjs`: copies the committed subset.
- `screenshots/before/` (18) and `screenshots/after/` (21): unedited PNG captures.
- `evidence/browser-comparison.json`, `evidence/scale-report.json`,
  `evidence/screenshots.sha256`.
- `brief.md`: the runner's packet revisions.

Ignored, not committed, under `artifacts/0033-editor-design-review/`:
- `before`: 56 captures.
- `final`: 63 captures, the source of the committed after set.
- `after`, `after-1`, `iter-1` and `iter-2`: superseded iterations.
- `scale-r1`, `scale-r2` and `scale-r3`. The committed report is r3; r1 and r2 used
  frame-based timing or predate the last two optimisations.

No Rust, SDK, CI, bridge transport (`editor/bridge`), schema, auth storage or account
setting changed. No Cargo command ran.

## Commands and results

```sh
npm ci --ignore-scripts                 # workspace root; editor/ui has no lockfile of its own
cd editor/ui
npx tsc -b --noEmit                     # strict: 0 errors
npx vitest run                          # 17 files, 430 tests passed (baseline: 16 files, 420)
npm run build                           # built
cd ../..
node handoffs/0033-editor-design-review/tools/capture.mjs before      # before any change: 56 captures
node handoffs/0033-editor-design-review/tools/capture.mjs final       # 63 captures, 0 step errors
node handoffs/0033-editor-design-review/tools/measure-scale.mjs r3    # unthrottled and 4x, no page errors
node handoffs/0033-editor-design-review/tools/curate.mjs before final r3
(cd handoffs/0033-editor-design-review && shasum -a 256 -c evidence/screenshots.sha256)   # 39 OK
```

The packet's `npm ci --prefix editor/ui` fails because the lockfile is at the workspace
root, so I used the root install, as handoff 0031 did. The first BEFORE run stalled on
the 65,536-input grid and was stopped and rerun with a per-capture time limit; the rerun
completed all 56 captures. Each capture server used the first free port from 4233 and
was closed afterwards.

Not run: Cargo, Clippy, GPU tests, native capture, native WebKit, phones.

## Screenshots

All are browser captures of the built UI at device scale 2. The "side" crops show the
whole right column, Inspector and agent pane.

| Subject | Before | After |
| --- | --- | --- |
| Shell, 1440×874, Pixel Camera | `screenshots/before/transform-camera-normal-1440x874.png` | `screenshots/after/transform-camera-normal-1440x874.png` |
| Shell, 1000×650 | `screenshots/before/transform-camera-minimum-1000x650.png` | `screenshots/after/transform-camera-minimum-1000x650.png` |
| Camera, 331 px and 280 px | `screenshots/before/transform-camera-*-side.png` | `screenshots/after/transform-camera-*-side.png` |
| Focused Near clip | `screenshots/before/camera-focus-normal-1440x874-side.png` | `screenshots/after/camera-focus-normal-1440x874-side.png` |
| Character: transform, body, capsule, velocity, script props | `screenshots/before/native-character-*-side.png` | `screenshots/after/native-character-*-side.png` |
| Mesh renderer, materials, compound collider | `screenshots/before/native-mesh-normal-1440x874-side.png` | `screenshots/after/native-mesh-normal-1440x874-side.png` |
| Directional light | `screenshots/before/native-sun-normal-1440x874-side.png` | `screenshots/after/native-sun-normal-1440x874-side.png` |
| Navigation mesh with a field error | `screenshots/before/native-navmesh-normal-1440x874-side.png` | `screenshots/after/native-navmesh-normal-1440x874-side.png` |
| Navigation grids, 384 and 65,536 values | `screenshots/before/native-grid*-side.png` | `screenshots/after/native-grid*-side.png` |
| Malformed and unregistered data | `screenshots/before/native-malformed-normal-1440x874-side.png` | `screenshots/after/native-malformed-normal-1440x874-side.png` |
| Compound with three errors | `screenshots/before/railing-compound-normal-1440x874-side.png` | `screenshots/after/railing-compound-normal-1440x874-side.png` |
| Unsupported and mismatched script fields | `screenshots/before/script-unsupported-normal-1440x874-side.png` | `screenshots/after/script-unsupported-normal-1440x874-side.png` |
| Long name at the minimum | `screenshots/before/long-name-minimum-1000x650-side.png` | `screenshots/after/long-name-minimum-1000x650-side.png` |
| Asset details | `screenshots/before/asset-texture-normal-1440x874-side.png` | `screenshots/after/asset-texture-normal-1440x874-side.png` |
| Agent collapsed, and signed in but unavailable | — | `screenshots/after/agent-*-side.png` |
| 100,000 entities filtered to 99,000 matches | — | `screenshots/after/stress-tree-normal-1440x874.png` |

The "Character" state shows the native agent reason, "Use the headless agent command
for the Phase 0 provider spike". That text comes from the bridge, not the UI; see
request 1.

## Requests for Astra

1. **Agent reason copy.** In `editor/bridge/native.ts`, `snapshotFromEngine` sets
   `agent.reason` to "Use the headless agent command for the Phase 0 provider spike."
   The agent note shows it verbatim. Replace it with product copy such as "The agent is
   not available in the editor yet." and regenerate `native.generated.js`.
2. **Entity kinds.** The native kind is only camera, mesh, script or entity. Derive
   `light` from DirectionalLight, PointLight, SpotLight or EnvironmentLight, and
   `audio` from AudioSource, AudioListener or AudioBus. The UI already has those icons
   and tints.
3. **Nullable type arrays.** `type: ["string", "null"]` (AudioSource.bus,
   AudioBus.parent) converts to `unsupported`. Convert it to the base type with
   `nullable: true`, as `anyOf` with null already does. The UI then shows "Not set" or
   the value.
4. **Order and units in the schema registry.** Nested object properties reach the UI in
   alphabetical order, so profiles restore NavigationMesh settings order. Please emit
   a field order for nested objects, and an `order` for NavigationMesh, NavigationGrid,
   AudioSource, Velocity and AngularVelocity. Add `x-incant-unit` where values have one:
   NavigationMesh lengths in m and max slope in °, AudioSource gain in dB and start in s,
   EnvironmentLight rotation in °, Velocity in m/s. Human `title`s such as "Rigid Body"
   would let the UI drop its name map.
5. **Snapshots at 100,000 entities.** `snapshotFromEngine` rebuilds every node and
   entity on each change. Copying a 100,000-entity snapshot alone cost 38 ms (162 ms at
   4×) per edit in my double, against about 10 ms (43 ms) for the UI. To keep edits under
   100 ms on the M1:
   - page entity details by selection, as the contract's comment anticipates;
   - keep unchanged hierarchy nodes as the same objects across revisions;
   - publish a revision number with each snapshot.
6. **Native captures after integration.** I need:
   - the sample Pixel Camera and Rubble Pile at 1440×874 and with the 280 px Inspector;
   - a Tab-focused value box;
   - the agent collapsed and open;
   - a real project with a light, a navigation grid and a script with props;
   - the macOS titlebar, to confirm the unchanged traffic-light alignment.

   I will commit only account-free crops.

## Limitations

- **Browser-only evidence.** Native WebKit rendering, the native titlebar and the
  native viewport were not reviewed.
- **Not the performance gate.** The scale numbers are from one fast desktop machine and
  headless Chrome; 4× throttling is not an M1 or a phone. The stress fixture is
  generated in the page, so its load time includes that work. The rename test double
  stands in for the engine and bridge, whose real edit latency is not measured here.
- **First filter long task.** Filtering all 100,000 names still runs as one long task:
  113 ms unthrottled and 392 ms at 4×.
- **Read-only Inspector.** Every field stays read-only until the bridge offers a
  component command. Value boxes are shaped for later editing.
- **Unsupported until the bridge changes.** Nullable `type` arrays show a notice until
  request 3 lands. Lights and audio show the generic entity icon until request 2.
- **Profile coverage.** Native profiles cover the registered components. A new
  component gets the generic presentation, and a profile applies only when every key
  it captions exists.
- **Grid map orientation.** The map draws engine row 0 at the top and says so. Whether
  authors expect world +Z up is an open design question.
- **Play mode.** There is no Play or live-runtime presentation. Distinguishing observed
  runtime values from authored ones remains future work under PLAN.md section 2.6.

## Open questions

- Should a short window keep the agent collapsed, or should the user's choice persist
  across sessions? It persists for the session now; there is no storage, by design.
- Should component display names come from engine schema titles (request 4)?
- Should the navigation grid map follow world axes once the engine documents its grid
  orientation?
