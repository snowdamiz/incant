# Incant editor design system, revision 3 ("calm inspector")

Status: proposal from handoff 0033 (Claude Opus 5.5, Max thinking). Not approved.
Revision 3 keeps revision 2's connected frame and neutral palette. It redesigns the
Inspector and the agent pane after the director found the right sidebar "messy,
cluttered, and sloppy, header too tall", and it adds bounded rendering for large
projects (PLAN.md revisions 4 and 5, ADR 0009).

Tokens: `src/styles/tokens.css`. Styles, one file per area:

| File | Covers |
| --- | --- |
| `base.css` | Reset, type, focus, buttons, chips, badges, spinners, empty states |
| `shell.css` | Frame, titlebar, notices, workspace grid, separators, panel chrome, viewport, status line |
| `hierarchy.css` | Tree rows and the entity/asset kind tints |
| `dock.css` | Output tabs, Problems, Console, History |
| `inspector.css` | Inspector identity, components, rows, value boxes, lists |
| `agent.css` | Agent pane and composer |
| `dialogs.css` | Keyboard shortcuts and ChatGPT account dialogs |
| `assets.css` | Asset list and asset details |

Titlebar host contract: `TITLEBAR.md`. Native viewport: `NATIVE_VIEWPORT.md`.

## Principles

1. **The viewport is the content.** Chrome is quiet neutral graphite. Colour is spent on
   status, provenance and one periwinkle accent.
2. **One frame, not cards.** Panels butt together and are divided by 1 px hairlines.
3. **Identity costs no extra height.** What a panel shows is named in its header line.
4. **Scan by column.** Labels in one column, values in another, at every nesting level.
5. **Calm, not empty.** Every panel has a designed loading, empty, error and "no engine"
   state, each with a one-line explanation.
6. **Honest, but quiet.** Read-only limits, unsupported fields and malformed values are
   always shown, in words, without alarm colour unless the engine reports a problem.
7. **Bounded.** No panel mounts one element per item of a large collection.
8. **Keyboard first.** Every action is reachable without a pointer. Focus is a ring and
   selection is a fill.

## Composition

```
┌ titlebar 40 ── wisp Project [Sample data] ·· ↶ ↷ │ ◧ ⬓ ◨ ·· (● ChatGPT …) ⌨ ┐
├──────────────┬──────────────────────────────┬─────────────────────────────┤ ← one ruled header line
│ Hierarchy 25 │ Viewport          ● Attached │ ▣ Pixel Camera            ⧉ │   (36 px, all three)
├──────────────┼──────────────────────────────┼─────────────────────────────┤
│ filter       │                              │ Camera · 01J9…00B  🔒 Read-only │ meta strip
│ tree         │   native surface             │ ▾ ✥ Transform               │ sticky 30 px headers
│ (windowed)   │                              │   Translation  [X 0│Y 2│Z 10]│
│              ├──────────────────────────────┤ ▾ ▭ Camera                  │
│              │ Problems · Console · History │ ...                         │
│              │                              ├─────────────────────────────┤
│              │                              │ ▾ ✧ Agent      ○ Not signed in │ collapsible
├──────────────┴──────────────────────────────┴─────────────────────────────┤
└ status line 28 ─ source · problems · live messages ················ F6 · ? ┘
```

- **Defaults** (`src/shell/layout.ts`): hierarchy 17 % of width (232–296 px), inspector
  23 % (320–400 px; 331 px at 1440 px), dock 27 % of height (160–340 px). The viewport
  never drops below 360 × 200. The Inspector's minimum is 280 px.
- **Agent pane.** Only an open, ready agent is a resizable pane with a separator
  (default 33 % of height, 260–340 px). Otherwise the pane is as tall as its content:
  a 36 px status line when collapsed, or the line plus one short note when open.

## Color

Dark only. Revision 3 keeps revision 2's values and adds tokens; every text pair is
enforced by `src/styles/tokens.test.ts`, which also keeps frame surfaces low-chroma.

| Role | Token | Value | Use |
| --- | --- | --- | --- |
| Canvas | `--color-bg-app` | `#0b0c0f` | titlebar, status line, native window background |
| Panel | `--color-bg-panel` | `#131418` | hierarchy, viewport strip, dock, dialogs |
| Side column | `--color-bg-side` | `#17181c` | Inspector and agent |
| Raised | `--color-bg-raised` | `#1e2026` | value boxes, chips, counts |
| Sunken | `--color-bg-sunken` | `#131418` | list boxes, the grid map and value tables inside the side column |
| Well | `--color-bg-well` | `#0e0f12` | raw values, composer |
| Selection | `--color-bg-selected` | `#262b52` | the one tinted surface |
| Accent | `--color-accent` | `#8c95ff` | selection underline, switches, agent mark |
| Focus | `--color-focus` | `#c3c8ff` | 2 px ring, ≥ 3:1 everywhere |

Kind tints are tokens. Meshes and models are neutral (`--color-kind-mesh: #b9c0cd`), so
a long hierarchy reads calm; cameras (sky), lights (amber), scripts (warm orange,
formerly violet), prefabs and textures (teal) and audio (pink) stand out. Icons always
sit next to a name. The dock's active tab icon is no longer accent-coloured; only its
underline is.

## Typography

Inter Variable for the UI and JetBrains Mono Variable for numbers, IDs, paths and raw
values. Both are bundled locally.

- 13 px default UI text; 12 px for Inspector labels, values, meta and captions (the
  minimum for content). Status-line key hints use 11 px.
- Numbers are monospace, so a unit can follow its number at an exact width (`ch`).
- Sentence case. Weights 420, 530 and 620.

## Inspector

**Header line (36 px).** When something is inspected, the panel's header line names it:
a 22 px kind tile in the kind's tint, the name (13 px semibold, ellipsis, full name in
the tooltip) and actions (Copy entity ID). The visible "Inspector" title returns when
nothing is inspected; the panel's accessible name is always "Inspector". Assets and the
import form use the same line; the asset name is the focus target when details open.

**Meta strip.** One 30 px line: kind · ID · Read-only. The ID is middle-truncated in CSS
(the head ellipsizes, the last six characters stay), and the DOM keeps the whole ID for
selection and copying. The read-only chip says, in its tooltip and screen-reader text,
that component values can be inspected but not edited yet.

**Problems on the entity.** One rounded block of rows, tinted by severity. Each row has
the engine's message and a link to the exact field (`Collider /shape/parts/3/id`), which
wraps at slashes.

**Components.** Hairline-separated sections. Headers are 30 px, sticky while their rows
scroll, and show a chevron, a component icon, the display name and problem counts.
Display names come from the schema title when it has words ("Mesh Renderer"), otherwise
from the profile or by splitting the identifier ("RigidBody" → "Rigid Body"). The type
ID is in the header tooltip and the section's accessible description.

**Rows.** A label column of `clamp(80px, 32cqi, 168px)` of the component's width, an
8 px gap, and the value column. Labels are muted 12 px, wrap to two lines and clamp at a
third. Captioned runs (Collider: Material, Collision) use a caption with a trailing
hairline instead of a box.

**Nesting.** Each group (a nested object, a union's variant fields, a part's card)
indents by 12 px with a 1 px guide line that starts under the group's legend, and its
label column shrinks by the same 12 px, so every value column lines up with the rows
above.

**Value boxes.** Quiet raised boxes (`#1e2026`, 26 px, radius 5) with no border; focus
draws the ring. Units follow the number ("0.1 m", "98000 lx"). Tags ("Default",
"Unused") sit at the trailing edge with a hairline outline. Unset optional values are an
outline with "Not set" or "Off".

**Vectors.** One segmented box: axis cells divided by 1 px of the panel colour, each
with its axis letter (X/Y/Z/W tinted; W/H and grid sizes neutral). Linear RGB colours
lead with an sRGB swatch. A quaternion wraps to 2 × 2 when its value column is under
340 px, so "0.2588" is never cut. Below 300 px of component width, vectors and notices
stack under their label; vectors in a part's card stack below 340 px.

**References.** An ID that exactly matches a project asset or entity shows that thing's
icon and name, then the authored ID in quieter monospace; the name is also spoken.
Other ULID-shaped strings show as monospace IDs. Nothing is resolved by guessing.

**Open objects** (a script's props, `additionalProperties`) list each authored key as
written with its value presented by its own type; lists and null show as raw JSON.

**Notices.** A value that cannot be presented as its schema says is a compact dashed
box: one sentence and the raw JSON. Rows with notices read from the top. Unsupported
schema forms say "This field's type is not supported yet".

**Lists of records** (compound parts, navigation sources and links) are one Tab stop:
a sunken list box of 26 px rows (index, tag, compact values, problem mark) and one
outlined card with the selected item's fields and its exact path. Profiles can supply a
one-line summary ("4, 0, 2 ↔ 4, 1.2, 4.5", "mesh · Player Character").

**Long number lists.** A grid of costs with a matching width × height draws as a pixel
map (blocked cells dark red, walkable cost from slate to amber, row 0 at the top), with
a legend and a "Show values" disclosure. Values open in a bounded, focusable table.
Other long number lists open as the table directly. String lists (material IDs) are a
bounded list of read-only boxes.

## Agent pane

The header is a disclosure: chevron, wisp mark, "Agent", and a status with a dot
(Ready, Working…, Signing in…, Sign-in problem, Unavailable, Not signed in, No engine).

- **Ready:** transcript (empty-state hint) and composer, in a resizable pane.
- **Cannot run:** one short note (title, the host's reason, "Everything else works
  offline, no account needed.") and its single next step (Continue with ChatGPT, or the
  sign-in progress button). There is no disabled composer.
- **Collapsed:** the 36 px status line. A window shorter than 720 px starts collapsed;
  after that, the user's choice sticks for the session.

## Bounded rendering

Long collections mount only what is near their scroll position (`src/shell/windowing.ts`):

| Collection | Row | Always mounted | Keyboard |
| --- | --- | --- | --- |
| Hierarchy tree | 28 px, flat ARIA tree with level, position and set size | active row, renaming row | arrows, Home/End, Page Up/Down, type-ahead |
| Asset list | 26 px folder label, 28 px row | each visible folder's label, active row | arrows, Home/End, Page Up/Down |
| Record lists | 26 px | selected item | arrows, Home/End, Page Up/Down |
| Value table | 22 px rows; grid columns too | — | scroll keys in the focused box |
| String list | 26 px, windowed above 48 items | — | Tab through boxes |

Keyboard moves to an unmounted item reveal it first and then focus it. A rename stays
mounted and keeps focus while its row scrolls out of view; Enter commits through the
command bus and Escape cancels. The hierarchy filter applies at a lower priority
(`useDeferredValue`), so typing stays responsive. Measured results are in the 0033
result; browser numbers are not the M1 device gate.

## Focus and keyboard

- Focus ring: 2 px `--color-focus` (`:focus-visible` and its `[data-focus-visible]`
  twin for WebKit script focus). Value boxes ring on `:focus-within`; vector cells and
  list rows ring inside their box.
- F6 / Shift+F6 cycle visible panels; the agent's focus target is its disclosure.
- `?` opens the shortcut list. A skip link is first.

## Motion

90 ms / 160 ms on hover, disclosure, switch and divider highlight; none under
`prefers-reduced-motion`. The pending dots pulse; nothing else animates continuously.

## Open design questions

- No light theme and no forced-colors (Windows high contrast) theme yet.
- No application menu (File/Edit/View) on Windows/Linux. See TITLEBAR.md.
- Editing: value boxes are shaped for future editing, but every field is read-only
  until the bridge offers a component command.
- Live runtime values during play (PLAN.md 2.6) will need a visible distinction from
  authored values; nothing in this revision implies play is available.
