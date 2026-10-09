# Incant editor design system, Phase 0 proposal (revision 2, "connected")

Status: proposal from handoff 0006 (Claude). Not approved. Revision 2 keeps revision 1's
neutral palette and components, and replaces its floating "islands" with the connected
panel structure of the landing-page product illustration, at the director's request.
Tokens: `src/styles/tokens.css`. Styles: `src/styles/app.css`. Titlebar host contract:
`TITLEBAR.md`. Native viewport: `NATIVE_VIEWPORT.md`.

## Principles

1. **The viewport is the content.** Chrome is quiet neutral graphite. Colour is spent on
   status, provenance and one periwinkle accent. The viewport is the largest region.
2. **One frame, not cards.** Panels butt together and are divided by 1 px hairlines, as in
   the landing-page illustration. There are no gutters, no rounded panel cards and no panel
   shadows. Breathing room lives inside controls and rows, not between panels.
3. **Calm, not empty.** Every panel has a designed loading, empty, error and "no engine"
   state, each with a one-line explanation.
4. **Honest, but quiet.** Fixture data and read-only limits are labelled as small chips with
   full explanations in tooltips and screen-reader text. Unavailable actions stay focusable
   and explain why.
5. **Keyboard first.** Every action is reachable without a pointer. Focus is a ring and
   selection is a fill.

## Composition

```
┌ titlebar 40 ── wisp Project [Sample data] ·· drag ·· ↶ ↷ │ ◧ ⬓ ◨ ·· drag ·· (● ChatGPT …) ⌨ ┐
├──────────────┬──────────────────────────────────┬──────────────────────┤ ← one ruled header line
│ Hierarchy 17 │ Viewport              ● Attached │ Inspector  Read-only │   (36 px, all three)
├──────────────┼──────────────────────────────────┼──────────────────────┤
│ filter       │                                  │ entity · id · copy   │
│ tree         │   native surface (square hole)   │ components           │
│              ├──────────────────────────────────┤──────────────────────┤
│              │ Problems · Console · History     │ Agent                │
│              │                                  │ composer             │
├──────────────┴──────────────────────────────────┴──────────────────────┤
└ status line 28 ─ source · problems · live messages ············ F6 · ? ┘
```

- **Frame:** the titlebar and status line use the canvas colour. The hierarchy, viewport
  strip and dock use the panel colour. The inspector/agent column is one step lighter, as the
  reference's inspector is. Hairlines separate everything.
- **Separators** are 1 px opaque lines and are also the resize handles. Their drag target
  extends 3 px into each neighbour. Hover lights them grey and focus lights them periwinkle.
- **Titlebar:** identity sits at the start, the history and layout tool cluster is centred in
  the window, and the account chip and help sit at the end. Empty space on either side is a
  drag area. Window controls follow the platform (TITLEBAR.md).
- **Header line:** the hierarchy, viewport and inspector headers share one 36 px height and
  one bottom hairline, so a single ruled line crosses the window. Titles are quiet 13 px
  muted labels, as in the reference.
- **Collapsible panels:** the three layout toggles (and ⌥⌘1/2/3) hide the hierarchy, output
  dock or inspector/agent column. The viewport takes the space.
- **Defaults** (`src/shell/layout.ts`): hierarchy 17 % of width (232–296 px), inspector 23 %
  (320–400 px), dock 27 % of height (160–340 px), agent 33 % (260–340 px). The viewport never
  drops below 360 × 200.

Measured in Chrome over the sample fixture (handoff 0006, `report.json`):

| Window | Viewport host | Hierarchy width | Inspector width |
| --- | --- | --- | --- |
| 1000×650 | 446×369 | 232 | 320 |
| 1280×800 | 726×479 | 232 | 320 |
| 1440×900 | 862×552 | 245 | 331 |

## Color

Dark only for Phase 0. Revision 2 returns to revision 1's neutral graphite after the director
found the violet-tinted first pass "a bit too purple". Every pair below is enforced by
`src/styles/tokens.test.ts`, which also checks that frame surfaces stay low-chroma. Pairs are
re-checked in Chrome by axe-core `color-contrast`.

| Role | Token | Value | Rule |
| --- | --- | --- | --- |
| Canvas | `--color-bg-app` | `#0b0c0f` | titlebar, status line; equals the native window background |
| Panel | `--color-bg-panel` | `#131418` | hierarchy, viewport strip, dock, dialogs |
| Side column | `--color-bg-side` | `#17181c` | inspector and agent |
| Raised | `--color-bg-raised` | `#1e2026` | fields on panels, chips, counts |
| Well | `--color-bg-well` | `#0e0f12` | value fields and composer on the side column |
| Hover | `--color-bg-hover` | `#24272e` | |
| Selection | `--color-bg-selected` | `#262b52` | indigo-tinted pill, the one tinted surface |
| Separator | `--color-border` | `#25282f` | opaque, so it stays solid over the native surface |
| Text | `--color-text` / `-soft` | `#ececf1` / `#d3d6dd` | ≥ 4.5:1 on every surface |
| Muted / subtle | `--color-text-muted` / `-subtle` | `#a9aeb9` / `#959cab` | ≥ 4.5:1 on every surface, incl. selection |
| Accent | `--color-accent` | `#8c95ff` | selection, active tab underline, switches, agent wisp |
| Focus | `--color-focus` | `#c3c8ff` | ring, ≥ 3:1 on every surface |
| Control border | `--color-border-control` | `#676e7b` | ≥ 3:1 |
| Status | danger / warning / success / info | `#ff7a70` / `#f0bd55` / `#5dd39a` / `#7ab8ff` | ≥ 4.5:1 |
| Provenance | user / agent / script / import | blue / orchid / green / amber | always with a word |

Colour is never the only cue: problems carry icons and counts, provenance carries a word,
switches carry "On/Off", and axes carry letters.

## Typography

Inter Variable for the UI and JetBrains Mono Variable for ids, paths, numbers and code. Both
are bundled locally, so nothing is fetched from a network.

- 13 px default and 12 px minimum for content. Status-line key hints use 11 px; that is the
  measured minimum.
- 13 px muted medium for panel titles, 14 px for inputs, chat and state titles, 15 px for the
  inspected entity name, 16 px for dialog titles, 20 px for full-screen errors.
- Sentence case with −0.011 em tracking. Weights are 420, 530 and 620. Tabular numerals are
  used for counts and values.

## Space and size

- Spacing follows a 4 px grid.
- Rows are 28 px, fields 28 px, panel headers and the dock tab strip 36 px, the titlebar 40 px
  (the native traffic lights are centred for it) and the status line 28 px.
- Separators are 1 px with a 7 px drag target. Tree indent is 14 px.
- Radii are 4 and 6 px for chips and controls, and 8 px for the composer. The dialog uses 12 px.
  Panels and the viewport are square.

## Components

- **Titlebar:** a 16 px wisp, then the project name. On macOS the logo box starts at 86 pt, so
  the glyph sits 15 pt after the green traffic light and is centred on the light row. The
  hairline below is an inset shadow so content centres on the full 40 px bar. Layout toggles
  are bright when their panel is shown and recede with a faint outline when hidden. The
  ChatGPT chip is a raised 26 px button like the reference's Play button.
- **Hierarchy:** 28 px rows inset 8 px from the panel edge, with one hairline indent guide per
  ancestor level. Resting names use the soft text colour. Selection is the indigo pill with
  full-white medium text. Problem marks are an unboxed icon and count, and collapsed parents
  show a dot for problems inside.
- **Inspector:** the identity block shows the kind tile, entity name, kind · ULID and copy-ID.
  "Read-only" is a quiet lock chip in the header. Components are disclosure rows separated by
  hairlines, with fields in a 34/66 label/value grid. Values sit in recessed 28 px wells with
  prefixes and suffixes inside: axis letters, link icons, swatches and units.
- **Output dock:** text tabs with a 2 px periwinkle underline on the active tab and counts in
  small raised boxes. Console level filters and history undo/redo are chips.
- **Viewport:** a quiet dotted field with a vignette and a centred tile explaining the state.
  With a surface attached, the hole is transparent and square. It reports corner radii
  `[0, 0, 0, 0]`, so the native surface needs no corner mask. Only the strip above it is painted.
- **Agent:** a periwinkle wisp mark and one honest sentence. The composer is a recessed
  rounded field with the send button inside, and it stays disabled while the bridge cannot
  send. No transcript is ever invented.
- **ChatGPT account (handoff 0003):** unchanged in behaviour. A 460 px modal has one status
  row, host errors verbatim with their code, saved accounts with "In use" only for the signed-in
  one, actions, and three facts. No field ever accepts a token, code or key. Initial focus is
  never destructive: when signed in, the dialog opens on Close, and the inline sign-out question
  opens on "Keep signed in".
- **Notices:** absent, opening and failed states show as one full-width strip under the
  titlebar. A failure uses an alert role and is the only place its full text and code appear.
  `project.*` codes read "The project could not be opened."; other codes read "Lost connection
  to the editor process." Reopening is the retry path, and the strip says so once.
- **Project states (handoff 0009):** while the project opens, nothing claims a result. The
  titlebar and the strip say "Opening project…". The hierarchy shows its skeleton, and the
  inspector, Problems and History show a spinner tile. The viewport pill reads "Waiting", and
  the status bar hides its error and warning counts. When the open fails, the hierarchy,
  viewport, inspector and History say "No project loaded" in one short line. Problems lists
  the engine's diagnostic. Without one, it says "Not validated", never "No problems". Saved
  account metadata in the titlebar chip stays visible in every project state.
- **Asset library (handoff 0010, revision 4).** Assets are content navigation, not output,
  so they live in the left column as a second view beside the Hierarchy. Problems, Console
  and History keep the bottom dock to themselves.
  - **Left column.** The header is a two-tab switcher, "Hierarchy" and "Assets", each with a
    count, in the dock-tab style.
  - **Asset list.** A filter and an Import button sit above the list. Assets are grouped
    under their source folder, and long folders keep their last segments, such as
    "…/surfaces/wood/". Rows reuse the hierarchy's 28 px pill: a kind icon, the name, and
    for textures a quiet interpretation tag on the right.
  - **Inspector.** It follows the left column. Hierarchy shows entity components, and Assets
    shows the selected asset or the import form, so "pick on the left, inspect on the right"
    holds everywhere. Asset details use the entity identity block, then sections for
    Source, Interpretation, Reimport and a collapsed Identifiers disclosure.
  - **Import form.** It sits in the Inspector, with a pinned action row at the bottom.
  - **Notices.** A running or failed import shows one line under the list toolbar, wherever
    the Inspector is. The Assets tab also carries a spinner or a red dot. The full error
    stays in the Inspector until it is dismissed or retried. The cooking note reads "You
    can keep working. Editing now means retrying the import." There is never a percentage
    or a cancel control.
  - **Leaving the asset list.** Revealing a problem from the dock returns the left column
    to the Hierarchy. Styles live in `src/styles/assets.css`.

## Focus and keyboard

- Focus ring: 2 px `--color-focus` (`:focus-visible`), inset inside lists and islands so
  it is never clipped. Field boxes ring on `:focus-within`.
- Keyboard modality (handoff 0009): WebKit draws no `:focus-visible` ring when script moves
  focus after a key press. `src/shell/inputModality.ts` sets `data-focus-visible` on the
  focused element while the last input was a key without ⌘, Ctrl or Alt. A pointer press
  clears it. `:focus-visible` rules have `[data-focus-visible]` twins. Keyboard behavior
  is covered by interaction tests and pixel review. A keyboard-opened dialog rings Close. A pointer-opened dialog
  focuses Close without a ring, as before.
- F6 / Shift+F6 cycle visible panels (hidden panels are skipped). Hierarchy is an ARIA
  tree with roving tabindex, type-ahead, F2 rename, Delete. Tabs use arrow keys. The left
  column's Hierarchy and Assets switcher is a tab list, so arrows move between the views.
  The asset list is an ARIA listbox grouped by folder, with a roving tabindex. Arrows, Home
  and End move the selection, and the Inspector follows it. Enter or Space moves focus to
  the asset's name in the Inspector, ⌘/Ctrl+F filters, and Escape in the Inspector returns
  to the row. Shift+Enter in a path field adds a row. A paste of several lines adds one row per
  line. If the field was empty or wholly selected, the first line replaces it.
  Dividers are `separator`s resizable with arrows (Shift = 64 px). `?` opens the
  shortcut list (modal, focus trapped, Escape returns focus). A skip link is first.

## Motion

90 ms / 160 ms on hover, disclosure, switch and divider highlight; none under
`prefers-reduced-motion`.

## Revision 2 vs. revision 1

| Area | Revision 1 "islands" (`handoffs/0006-connected-editor/screenshots/before/`) | Revision 2 "connected" (`screenshots/after/`) |
| --- | --- | --- |
| Frame | Rounded 10 px cards on a canvas, 6 px gutters | One frame; panels butt together with 1 px hairlines |
| Headers | 40 px, semibold white titles, uneven lines | One 36 px ruled line across all three columns; quiet muted titles |
| Titlebar | Tools pushed right; logo, slash, name | Centred tool cluster; logo and name; 15 pt clear of the lights |
| Inspector column | Same surface as other cards | One step lighter, like the reference; fields in recessed wells |
| Dock | Pill tabs | Underlined text tabs |
| Agent | Gradient spark tile | Periwinkle wisp mark |
| Viewport | Rounded bottom corners, GPU corner mask | Square and flush; zero radii |
| Palette | Neutral graphite | Same graphite (a violet-tinted pass was rejected as too purple) |

## Open design questions

- No light theme and no forced-colors (Windows high contrast) theme yet.
- There is no application menu (File/Edit/View) on Windows/Linux. See TITLEBAR.md.
- Long entity names with two problem marks still truncate at 1280 px in the default
  240 px hierarchy.
- No measured platform performance budgets were supplied for the webview. See the
  handoff 0006 result for the current bundle sizes.
