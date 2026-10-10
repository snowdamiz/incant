# Incant editor design system, revision 4 ("studio")

Status: proposal from handoff 0033 (Claude Opus 5.5, Max thinking). Not approved.
Revision 4 keeps the connected frame and neutral palette of revisions 2 and 3, and
carries the design through the whole app as one product. The director found revision 3
still cramped in the Inspector (the compound Collider especially) and limited to that
one sidebar. Bounded rendering for large projects (PLAN.md revisions 4 and 5, ADR 0009)
is unchanged.

Tokens: `src/styles/tokens.css`. Styles, one file per area:

| File | Covers |
| --- | --- |
| `base.css` | Reset, type, focus, buttons, chips, badges, spinners, empty states, menus, fatal screens |
| `shell.css` | Frame, titlebar, notices, workspace grid, separators, panel chrome, search fields, viewport, status line |
| `hierarchy.css` | Tree rows and the entity/asset kind tints |
| `dock.css` | Output tabs, filters, Problems, Console, History |
| `inspector.css` | Inspector identity, problems, components, rows, value boxes, lists |
| `agent.css` | Agent pane and composer |
| `dialogs.css` | Dialog frame, keyboard shortcuts, ChatGPT account |
| `assets.css` | Left-column tabs, asset list and asset details |

Titlebar host contract: `TITLEBAR.md`. Native viewport: `NATIVE_VIEWPORT.md`.

## Principles

1. **The viewport is the content.** Chrome is quiet neutral graphite. Colour is spent on
   status, provenance, kinds and one periwinkle accent.
2. **One frame, not cards.** Panels butt together and are divided by 1 px hairlines.
   Cards appear only where something floats (dialogs, menus, fatal screens) or groups a
   reference (shortcut categories, the account's state).
3. **One product.** Every surface uses the same parts: a 36 px header line, tinted kind
   tiles, the state badge, filter chips, keycaps, the dashed "shown as authored" notice.
4. **Identity costs no extra height.** What a panel shows is named in its header line.
5. **Scan by column.** Labels in one column, values in another. Only real nested
   objects indent; nothing is boxed more than one level deep.
6. **Room to read, not padding for its own sake.** 28 px values 6 px apart; spacing
   grows between groups, not inside them.
7. **Honest, but quiet.** Read-only limits, unsupported fields and malformed values are
   always shown, in words, without alarm colour unless the engine reports a problem.
   Nothing claims a state it has not verified ("No problems" only after validation).
8. **Bounded.** No panel mounts one element per item of a large collection.
9. **Keyboard first.** Every action is reachable without a pointer. Focus is a ring and
   selection is a fill. The window never scrolls; each panel scrolls its own content.

## Composition

```
┌ titlebar 40 ─ ◆ Project / Scene [Sample data] ·· (↶ ↷) (◧ ⬓ ◨) ·· (Ⓐ ChatGPT …) ⌨ ┐
├──────────────────┬──────────────────────────────┬─────────────────────────────┤ ← one ruled header line
│ Hierarchy 25│Assets 7│ Viewport ● Live           │ ▣ Close-up Camera         ⧉ │   (36 px, all three)
├──────────────────┼──────────────────────────────┼─────────────────────────────┤
│ ⌕ Filter      ⇕  │                              │ Camera · 01J9…00B 🔒 Read-only │ meta strip
│ ▾ ◈ Scene        │   native surface             │ ▾ [✥] Transform             │ sticky 34 px headers
│   ▸ ▢ Group      │                              │   Translation [X 0│Y 2│Z 10]│ 28 px values, 6 px apart
│ ▌ ▣ Selected     ├──────────────────────────────┤ ▾ [▭] Camera                │ family-tinted marks
│   (windowed)     │ Problems 9 · Console · History│ ...                         │
│                  │ (Errors 6)(Warnings 2)(Info 1)├─────────────────────────────┤
│                  │ ◈ Crate 03            ⓘ1    │ ▾ ✧ Agent      ○ Not signed in │ collapsible
├──────────────────┴──────────────────────────────┴─────────────────────────────┤
└ status line 28 ─ ● source · ⓘ6 ⚠2 · live messages ············ [F6] Panels [?] Shortcuts ┘
```

- **Defaults** (`src/shell/layout.ts`): hierarchy 17 % of width (232–296 px), inspector
  25 % (340–420 px; 360 px at 1440 px), dock 27 % of height (160–340 px). The viewport
  never drops below 360 × 200. The Inspector's minimum is 280 px.
- **Agent pane.** Only an open, ready agent is a resizable pane with a separator
  (default 33 % of height, 260–340 px). Otherwise the pane is as tall as its content:
  a 36 px status line when collapsed, or the line plus one short note when open.
- **The window never scrolls.** `.app` contains and clips its descendants, and every
  scroller is a containing block, so `scrollIntoView`, focus and visually hidden text
  cannot shift the titlebar off screen.

## Color

Dark only. Every text pair is enforced by `src/styles/tokens.test.ts`, which also keeps
frame surfaces low-chroma.

| Role | Token | Value | Use |
| --- | --- | --- | --- |
| Canvas | `--color-bg-app` | `#0b0c0f` | titlebar, status line, native window background, fatal screens |
| Panel | `--color-bg-panel` | `#131418` | hierarchy, viewport strip, dock, dialogs, menus |
| Side column | `--color-bg-side` | `#17181c` | Inspector and agent |
| Raised | `--color-bg-raised` | `#1e2026` | value boxes, chips, counts, keycaps |
| Sunken | `--color-bg-sunken` | `#131418` | list wells, the grid map and value tables inside the side column |
| Well | `--color-bg-well` | `#0e0f12` | raw values, composer |
| Selection | `--color-bg-selected` | `#262b52` | the one tinted surface |
| Accent | `--color-accent` | `#8c95ff` | selection edge and underline, switches, dialog marks, agent mark |
| Focus | `--color-focus` | `#c3c8ff` | 2 px ring, ≥ 3:1 everywhere |

**Kind tints.** Meshes and models are neutral (`--color-kind-mesh: #b9c0cd`), so a long
hierarchy reads calm; cameras (sky), lights (amber), scripts (warm orange), prefabs and
textures (teal) and audio (pink) stand out. Icons always sit next to a name.

**Component families.** An Inspector component's mark is tinted by what it is about.
Families that match a kind reuse its tint (light, camera, audio, script, render = mesh);
two new tokens cover the rest: `--color-family-physics: #7ed6ae` (Rigid Body, Collider,
Velocity, Angular Velocity) and `--color-family-navigation: #b8d97a`. Transform and
unknown components stay neutral.

**Status.** Danger, warning, info and success tints colour icons, badges, filter chips,
the problems card and state badges. Text, icons and counts always carry the meaning too.

## Typography

Inter Variable for the UI and JetBrains Mono Variable for numbers, IDs, paths and raw
values. Both are bundled locally.

- 13 px default UI text; 12 px for labels, values, meta, captions, key caps and badges.
  12 px is the minimum everywhere (revision 3's 11 px key hints are gone).
- Numbers are monospace, so a unit can follow its number at an exact width (`ch`).
- Sentence case. Weights 420, 530 and 620. Dialog titles 16 px; fatal titles 20 px.

## Shell

**Titlebar (40 px).** Product mark, then a breadcrumb: the project name (the page's
`h1`) and, when the project shows one scene, a slash and the scene's name ("N scenes"
otherwise). History and panel toggles sit in two capsules; a pressed toggle is filled.
The ChatGPT chip is a pill: a status dot, or the account's initial in an avatar with a
green presence ring when signed in. Keyboard shortcuts is the last button.

**Left column.** "Hierarchy" and "Assets" are tabs in the header line; their counts use
the same small pill as the dock's tab counts. The search field has a leading icon and a
live match count.

**Status line (28 px).** The source (with a tone dot), a problems button (only the
counts that exist, or a green check and "No problems" once the project is validated;
nothing while it opens or after it fails without diagnostics), the live message, and
key hints as caps: `F6` Panels, `?` Shortcuts. The button opens Problems.

## Viewport

The header pill says Live, Not attached, Waiting, Error or No engine with a tone dot
(pending pulses, except under reduced motion). The empty state is an illustration: a ground grid
running to the horizon and an outlined cube, drawn in `currentColor`, dashed and red on
error. Its title and one sentence say why there is no picture.

## Hierarchy

Flat ARIA tree, 28 px rows. The scene row is a heading weight; the selected row is the
indigo pill plus a 2 px accent edge. Problem counts sit at the trailing edge.

**Context menu** (right-click, Shift+F10 or the Menu key): Rename (F2), Copy ID, Delete
(Del, danger). Items the bridge cannot do stay focusable, are marked unavailable and
explain why instead of acting; Delete runs through the command bus. Copy ID (here and
the Inspector's copy button) is marked unavailable, with the reason, where the runtime
has no clipboard, and a refused write is announced; neither fails silently. Roving
focus, arrows, Home/End, Enter/Space and Escape (focus returns to the row).

## Inspector

**Header line (36 px).** When something is inspected, the panel's header line names it:
a 22 px kind tile in the kind's tint, the name (13 px semibold, ellipsis, full name in
the tooltip) and actions (Copy entity ID). The visible "Inspector" title returns when
nothing is inspected; the panel's accessible name is always "Inspector". Assets and the
import form use the same line.

**Meta strip.** One line: kind · ID · Read-only. The ID is middle-truncated in CSS, and
the DOM keeps the whole ID for selection and copying.

**Problems on the entity.** One card tinted by the worst severity (danger, warning or
info), lines divided by a faint rule. Each line is the engine's message, then where it
is: the component name and the exact path in quiet monospace, as a link that focuses the
field. Paths wrap after slashes, never mid-name.

**Components.** Hairline-separated sections with 34 px sticky headers: chevron, a 22 px
mark (the component's icon on a tile tinted by its family), the display name and problem
counts. Display names come from the schema title when it has words, otherwise from the
profile or by splitting the identifier. The type ID is in the tooltip and the section's
accessible description.

**Rhythm.** Values are 28 px tall and 6 px apart. Captioned runs (Collider: Material,
Collision) start 10 px below the previous row with a caption and a trailing hairline.
Component bodies end with 16 px of air.

**Rows.** A label column of `clamp(88px, 32cqi, 168px)` of the component's width, an
8 px gap, and the value column. Labels are muted 12 px, wrap to two lines and clamp at a
third. A vector that wraps (a quaternion's X Y / Z W) keeps its label on its first line.

**Nesting.** Only real nested objects (NavigationMesh build settings, script props)
indent 12 px with a guide line, and narrow their label column by the same step, so value
columns align. A union's variant fields continue under the selector at its own depth;
the variant's note sits in the value column under the selector, as help for that value.
A list's selected item also continues at the list's depth (below).

**Value boxes.** Quiet raised boxes (28 px, radius 5) with no border; focus draws the
ring. Units follow the number. Tags ("Default", "Unused") sit at the trailing edge with
a hairline outline. Unset optional values are an outline with "Not set" or "Off". A
stable ID (a part's own ID) is metadata: an outline and muted text instead of a fill.
Union selectors can lead with a glyph (collider box, sphere, capsule, compound).

**Errors.** An invalid value tints its box and draws a red hairline; a whole invalid
vector (a non-unit quaternion) tints every cell and outlines the box once.

**References.** An ID that exactly matches a project asset or entity shows that thing's
icon and name, then the authored ID in quieter monospace; the name is also spoken.

**Notices.** A value, field or component that cannot be presented as its schema says is
one dashed box (warning hairline, faint tint): one sentence and the raw JSON. Field
notices and component notices look alike, so none shouts.

**Lists of records** (compound parts, navigation sources and links) are one Tab stop:
a header with the name on the left and the breakdown on the right ("64 parts · 22 box,
21 sphere, 21 capsule", on its own line when both do not fit), a sunken well of 26 px
rows (index, glyph, tag, compact values, problem mark) and, below a hairline, the
selected item: "Part 42 of 64" with its document path (`/shape/parts/41`), then its
fields at the list's own depth. Compound parts read primitive and size first, then
offset and rotation, then the stable ID.

**Long number lists.** A grid of costs with a matching width × height draws as a pixel
map with a legend and a "Show values" disclosure; values open in a bounded, focusable
table. String lists are a bounded list of read-only boxes.

## Output dock

Tabs with icons and count pills (red for errors). Each view starts with a 40 px toolbar.

**Pages.** The bridge caps none of these lists and their rows wrap to their full text, so
each view mounts at most one page of 100 rows. When a list is longer, the toolbar's
trailing edge says which slice is shown ("101–200 of 20,000") with first, previous,
next and last page buttons (at an end they stay focusable and are marked disabled). A
page change is announced and starts at the page's top. Nothing is hidden: every entry
is one page away, filters count every entry, and a list that fits one page shows no
pager at all.

- **Problems.** Severity filter chips (Errors, Warnings, Info, with counts; on is filled,
  off is an outline). Problems are grouped by entity, worst first, project-level
  problems last; each group has a heading with the entity's kind icon, name and counts,
  and each row the severity icon, the message, and "Component · /path" in monospace,
  wrapping under the message when space runs out. All filtered out, and validated with
  nothing to report, are distinct designed states. A group that spans pages repeats its
  heading with a "continued" tag, keeps its whole-group counts, and ends with "312 more
  problems on Crate 03 on the next pages". A filter change returns to the first page.
- **Console.** A table of time, level badge (info, warn, error, debug), source and
  message, with level filter chips; debug is hidden by default; error rows are tinted.
  It opens on the newest page and follows it as messages arrive; another page stays put
  (only its total grows) until **Newest**, an accent pill, returns to the end.
- **History.** A timeline, oldest first: a dot per transaction coloured by origin (user,
  agent, script, import), the description, an origin chip, the model or script, and the
  time in a fixed last column. A "Now" marker separates applied edits from those Redo
  would restore; undone edits are hollow dots and struck through. The toolbar has Undo,
  Redo and "4 of 5 applied". The list is a keyboard stop so it can scroll. It opens on
  the page that holds Now and follows it; **Now** returns there from any other page.

## Agent pane

The header is a disclosure: chevron, wisp mark, "Agent", and a status with a dot
(Ready, Working…, Signing in…, Sign-in problem, Unavailable, Not signed in, No engine).

- **Ready:** transcript (empty-state hint) and composer, in a resizable pane.
- **Cannot run:** one short note (a title, the host's reason when it adds something,
  "Everything else works offline, no account needed.") and its single next step.
  A reason that names an internal milestone (the native bridge's "headless agent
  command … Phase 0 provider spike") is shown in plain words: "Not in the editor yet".
  Connecting an account stays offered because the bridge really does it.
- **Collapsed:** the 36 px status line. A window shorter than 720 px starts collapsed.

## Dialogs

Floating cards: 14 px radius, a soft entrance (none under reduced motion), focus
trapped, Escape closes and focus returns to the opener.

- **Keyboard shortcuts.** A searchable reference: a header mark and subtitle naming the
  platform, a filter (focused on open; it matches actions, groups and key names, shows
  "N of M", and Escape clears it before closing), and two balanced columns of category
  cards (General, Hierarchy, Panels and layout, Lists, assets and inspector, Agent; one
  column in a narrow window). Each row is the action, an optional "where", and the
  current platform's keys as individual caps (⌘ ⇧ ⌥ on macOS, names elsewhere), "or"
  between alternatives and "/" between paired directions. Every listed key is
  implemented.
- **ChatGPT account.** The state as a card (tinted badge, headline, next step), the
  primary action, saved accounts as rows with avatars, and quiet facts with icons.

## States

Every panel has designed loading, empty, error and "no engine" states. The state badge
is a 48 px (40 px compact) disc tinted by tone (neutral, error, warning, success) with
the state's pictogram (a pointer for "Nothing selected", the scene mark while no project
is open, a check for "No problems"). Loading lists use skeleton rows shaped like tree
rows. **Fatal screens** (an incompatible bridge, an unknown fixture) are a centred card
on the canvas: the product mark, an error tile, the problem as the page's `h1`, and what
to do next; protocol versions are compared side by side and valid fixture names are
chips.

## Bounded rendering

Long collections mount only what is near their scroll position (`src/shell/windowing.ts`):

| Collection | Row | Always mounted | Keyboard |
| --- | --- | --- | --- |
| Hierarchy tree | 28 px, flat ARIA tree with level, position and set size | active row, renaming row | arrows, Home/End, Page Up/Down, type-ahead |
| Asset list | 26 px folder label, 28 px row | each visible folder's label, active row | arrows, Home/End, Page Up/Down |
| Record lists | 26 px | selected item | arrows, Home/End, Page Up/Down |
| Value table | 22 px rows; grid columns too | — | scroll keys in the focused box |
| String list | 26 px, windowed above 48 items | — | Tab through boxes |
| Problems, Console, History | wrapping rows, 100 per page | the page's rows only | Tab through rows and the pager; lists scroll with keys |

Keyboard moves to an unmounted item reveal it first, measure the scroller at once, and
then focus it after React commits, so the rows around the target mount in the same
render. Scroll and resize notifications are coalesced on an animation frame with a
100 ms timer fallback, because WebKit throttles frames for hidden or occluded views.
A rename stays mounted and keeps focus while its row scrolls out of view.

## Focus and keyboard

- Focus ring: 2 px `--color-focus` (`:focus-visible` and its `[data-focus-visible]`
  twin for WebKit script focus). Value boxes ring on `:focus-within`; vector cells, list
  rows and menu items ring inside their box.
- F6 / Shift+F6 cycle visible panels; `?` or ⌘/ opens the shortcut reference.

## Motion

90 ms / 160 ms on hover, disclosure, switch, divider highlight and dialog entrance; none
under `prefers-reduced-motion`. Pending dots pulse; nothing else animates continuously.

## Open design questions

- No light theme and no forced-colors (Windows high contrast) theme yet.
- No application menu (File/Edit/View) on Windows/Linux. See TITLEBAR.md.
- Editing: value boxes are shaped for future editing, but every field is read-only
  until the bridge offers a component command.
- Live runtime values during play (PLAN.md 2.6) will need a visible distinction from
  authored values; nothing in this revision implies play is available.
