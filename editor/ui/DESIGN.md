# Incant editor design system, revision 5 ("plain")

Status: proposal from handoff 0033 (Claude Opus 5.5, Max thinking). Not approved.
Revision 5 keeps revision 4's connected frame, neutral palette, layout, rhythm and
bounded rendering, and removes its ornament. The director rejected revision 4 as
ornamental: explanatory paragraphs, a metadata row, a boxy account dialog, a card grid
of keycaps, a long account pill, tinted icon tiles, glowing badges behind empty-state
icons and accent edges. Revision 5 spends pixels only on content, state and
interaction. Bounded rendering for large projects (PLAN.md revisions 4 and 5, ADR 0009)
is unchanged.

Tokens: `src/styles/tokens.css`. Styles, one file per area:

| File | Covers |
| --- | --- |
| `base.css` | Reset, type, focus, buttons, badges, spinners, empty states, menus, fatal screens |
| `shell.css` | Frame, titlebar, account button, notices, workspace grid, separators, panel chrome, search fields, viewport, status line |
| `hierarchy.css` | Tree rows and the entity/asset kind tints |
| `dock.css` | Output tabs, filters, pager, Problems, Console, History |
| `inspector.css` | Inspector identity, problems, components, rows, value boxes, lists |
| `agent.css` | Agent pane and composer |
| `dialogs.css` | Dialog frame, keyboard shortcuts, ChatGPT account |
| `assets.css` | Left-column tabs, asset list and asset details |
| `play.css` | Play state and transport in the viewport header, the Play notice, the focused game view, the Inspector's runtime block |

Titlebar host contract: `TITLEBAR.md`. Native viewport: `NATIVE_VIEWPORT.md`.

## Principles

1. **The viewport is the content.** Chrome is quiet neutral graphite. Colour is spent on
   status, provenance, kinds and one periwinkle accent.
2. **One frame, not cards.** Panels butt together and are divided by 1 px hairlines.
   Only floating things (dialogs, menus) have a surface of their own. Nothing is boxed
   more than one level deep, and nothing is boxed for decoration.
3. **No ornament.** No icon tiles, glows, badge discs, accent edges, illustrations,
   gradients or entrance animations. An icon appears only beside the words it marks.
4. **Say it once.** A fact appears in one place: the header line names what is shown,
   a banner states the project's condition, a panel states only what is its own. No
   sentence explains what the layout already shows. Spoken-only text (visually hidden)
   keeps assistive tech complete where the screen no longer repeats a fact.
5. **Identity costs no extra height.** What a panel shows is named in its header line.
   Kind and stable ID are the header's accessible description, not a visible row.
6. **Scan by column.** Labels in one column, values in another. Only real nested
   objects indent.
7. **Room to read, not padding for its own sake.** 28 px values 6 px apart; spacing
   grows between groups, not inside them. Removed ornament is not replaced by air.
8. **Honest, but quiet.** Read-only limits, unsupported fields and malformed values are
   always shown, in words, without alarm colour unless the engine reports a problem.
   Nothing claims a state it has not verified ("No problems" only after validation).
9. **Bounded.** No panel mounts one element per item of a large collection.
10. **Keyboard first.** Every action is reachable without a pointer. Focus is a ring and
    selection is a fill. The window never scrolls; each panel scrolls its own content.

## Composition

```
┌ titlebar 40 ─ ◆ Project / Scene  Sample data ·· ↶ ↷  ◧ ⬓ ◨ ·············· Ⓢ ⌨ ┐
├──────────────────┬──────────────────────────────┬─────────────────────────────┤ ← one ruled header line
│ Hierarchy 25 Assets 7 │ Viewport ● Live   ▶ ⏭ ■ │ ▭ Close-up Camera      🔒 ⧉ │   (36 px, all three)
├──────────────────┼──────────────────────────────┼─────────────────────────────┤
│ ⌕ Filter      ⇕  │                              │ ▾ ✥ Transform               │ sticky 34 px headers
│ ▾ ◈ Scene        │   native surface             │   Translation [X 0│Y 2│Z 10]│ 28 px values, 6 px apart
│   ▸ ▢ Group      │                              │ ▾ ▭ Camera                  │
│   ▣ Selected     ├──────────────────────────────┤   Projection [Perspective  Default]
│   (windowed)     │ Problems 9  Console 5  History│ ...                         │
│                  │ Errors 6  Warnings 2  Info 1 ├─────────────────────────────┤
│                  │ ◈ Crate 03               ⓘ 1 │ ▾ ✧ Agent      ○ Not signed in │ collapsible
├──────────────────┴──────────────────────────────┴─────────────────────────────┤
└ status line 28 ─ ● source  ⓘ 6 ⚠ 2  live message ┘
```

- **Defaults** (`src/shell/layout.ts`): hierarchy 17 % of width (232–296 px), inspector
  25 % (340–420 px; 360 px at 1440 px), dock 27 % of height (160–340 px). The viewport
  never drops below 360 × 200. The Inspector's minimum is 280 px.
- **Agent pane.** An open agent that is ready, re-reading its session, or holding turns
  is a resizable pane with a separator (default 33 % of height, 260–340 px). Otherwise the
  pane is as tall as its content: a 36 px status line when collapsed, or the line, one
  sentence and the allowance line when open.
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
| Raised | `--color-bg-raised` | `#1e2026` | value boxes, pressed filters |
| Sunken | `--color-bg-sunken` | `#131418` | list wells, the grid map and value tables inside the side column |
| Well | `--color-bg-well` | `#0e0f12` | raw values, composer |
| Selection | `--color-bg-selected` | `#262b52` | the one tinted surface; avatars |
| Accent | `--color-accent` | `#8c95ff` | tab underlines, switches, the chosen segment, mask cells, primary and send buttons, the Now line, agent mark |
| Focus | `--color-focus` | `#c3c8ff` | 2 px ring, ≥ 3:1 everywhere |

**Kind tints.** Meshes and models are neutral (`--color-kind-mesh: #b9c0cd`), so a long
hierarchy reads calm; cameras (sky), lights (amber), scripts (warm orange), prefabs and
textures (teal) and audio (pink) stand out. Icons always sit next to a name. Component
marks in the Inspector are not tinted (revision 4's family colours are gone).

**Status.** Danger, warning, info and success colour icons, counts and words; they never
fill a decorative surface. Text, icons and counts always carry the meaning too.

## Typography

Inter Variable for the UI and JetBrains Mono Variable for numbers, IDs, paths and raw
values. Both are bundled locally.

- 13 px default UI text; 12 px for labels, values, meta and captions. 12 px is the
  minimum everywhere.
- Numbers are monospace, so a unit can follow its number at an exact width (`ch`).
- Sentence case. Weights 420, 530 and 620. Dialog titles 14 px; fatal titles 16 px.

## Shell

**Titlebar (40 px).** Product mark, then a breadcrumb: the project name (the page's
`h1`) and, when the project shows one scene whose name differs from the project's, a
slash and the scene's name ("N scenes" for several). A sample project adds "Sample data"
as plain amber words. History and panel toggles are two plain groups; a pressed toggle
is filled. The account is one 28 px button showing only the signed-in account's initial
in a ringed avatar, or a profile glyph when nobody is signed in; a small corner dot
marks signing in (amber) or a sign-in problem (red). Its accessible name says the state
("ChatGPT account, signed in as …") and its tooltip adds an error's message. Keyboard
shortcuts is the last button.

**Left column.** "Hierarchy" and "Assets" are tabs in the header line with plain
counts. The search field has a leading icon and a live match count.

**Status line (28 px).** The source (with a tone dot), a problems button (only the
counts that exist, or a green check and "No problems" once the project is validated;
nothing while it opens or after it fails without diagnostics) and the live message. No
key hints: the shortcut reference is one keystroke or one titlebar button away.

**Banner.** A one-line strip under the titlebar only when the engine is absent, the
project is opening, or it failed. It is the one place the project's condition is stated
in full; panels show a short state of their own (or, while opening, keep it spoken-only).

## Viewport

The header says Live, Not attached, Waiting, Error or No engine with a tone dot. The
empty area shows only what the header does not: the host's reason or a render error's
message, in one quiet paragraph. No illustration, grid or vignette.

## Play

Native Play runs the game in a disposable native world, shown in the viewport (handoff
0044). It lives in the viewport header and the Inspector, without a new panel, colour or
frame.

**Header line.** The state replaces the viewport's own word while a session exists:
Preparing… and Stopping… (a small spinner instead of the dot), Playing (green), Suspended
(amber, only once a hold lasts 400 ms; tooltip: the window is in the background or the
viewport is hidden), Paused · tick 1,204 (blue, the tick in quiet numerals) and Play failed
or Play unavailable (red). With several scenes the playing scene follows Playing. At the
trailing edge are three plain 28 px buttons, always in the same place: Play (Resume while
paused, Pause while running), Step and Stop. A button that cannot act stays focusable, is
marked unavailable, and says why when pressed. Generation, sequence and frame counters are
not shown.

**Scene and camera.** Shown only when there is a choice: a quiet button before the transport
naming the target ("Harbor · Player Camera", or "Choose scene") that opens one radio list of
every scene and camera. A single scene and a scene's only camera are used without asking. Play
without a needed choice opens the list, and choosing starts Play. Choices are remembered per
scene for the editor session. The button hides while a session exists; if it had focus, focus
moves to Play.

**Notice.** A failed session's message is one strip under the header, the engine's words
after "Play could not start." or "Play stopped at tick 812.", with Dismiss; an unreadable
status says so in the same strip, without Dismiss. It is the one place the failure is stated;
it is an alert, and Play's earlier status-line message is cleared. Dismissed from the
keyboard, focus moves to Play rather than leaving with the strip.

**Game view.** While a session exists the viewport host is a focusable application region,
"Game view". A click or keyboard navigation focuses it (that press is not sent) and draws the
2 px focus ring inside its edge, whichever way it was focused. Keys and the pointer then go
to the game while it runs, including keys such as ? that are editor shortcuts elsewhere; F6
and Shift+F6 leave the view, and the Play shortcuts and ⌘ combinations stay the editor's.
Losing focus releases held input, and input that resumes releases whatever the game still
held first. The game has focus only while the view has: when Play starts or resumes, or the
window comes back, with focus in another panel, the game is told it has none. A session that
ends with focus in the view moves focus to Play.

**Runtime block.** While a session runs or is paused, an entity of the playing scene shows a
sunken block above its authored components: "Runtime", the tick it was read at, and a
refresh button; then Translation, Rotation, Scale and, when the entity has one, Velocity, in
the authored rows' own presentation. It is read on selection, after Pause and after each
Step, and never polled. An entity the game removed says "No runtime state." with the
engine's reason; an entity of another scene says "Not in the running scene." Authored
values are never shown in its place, and nothing in it is written to the project.

## Hierarchy

Flat ARIA tree, 28 px rows. The scene row is a heading weight; the selected row is the
indigo pill (no accent edge). Problem counts sit at the trailing edge.

**Context menu** (right-click, Shift+F10 or the Menu key): Rename (F2), Copy ID, Delete
(⌘⌫ on macOS, Delete elsewhere; danger). Shortcut labels are plain text in the
platform's notation. Items the bridge cannot do stay focusable, are marked unavailable
and explain why instead of acting; Delete runs through the command bus. Copy ID (here
and the Inspector's copy button) is marked unavailable, with the reason, where the
runtime has no clipboard, and a refused write is announced; neither fails silently.
Roving focus, arrows, Home/End, Enter/Space and Escape (focus returns to the row).

## Inspector

**Header line (36 px).** When something is inspected, the panel's header line names it:
the kind's glyph in its tint (no tile), the name (13 px semibold, ellipsis, full name in
the tooltip) and trailing marks: a lock glyph meaning read-only (tooltip "Read-only:
values can be inspected but not edited yet", spoken "Read-only") and the Copy ID button.
The kind and the stable ID are the name's accessible description. There is no meta row.
The visible "Inspector" title returns when nothing is inspected; the panel's accessible
name is always "Inspector". Assets and the import form use the same line.

**Assets by source type.** The list tags an ordinary image with its interpretation
(Color, Linear, Normal map) and a sprite atlas (`.atlas.json`) with "Atlas"; models and
audio are icon-only, their kind spoken. Details are Source, Interpretation, Reimport and
Identifiers: an image's interpretation is a choice, an atlas shows its saved value in a
plain value box ("Sprite atlases are always color"), and audio has no Interpretation.
Import rows name their type in quiet text under the path ("Model", "Color atlas", "WAV
audio", "OGG audio"); image rows show the interpretation choice instead.

**Problems on the entity.** A plain list under the header: each line is the severity
icon, the engine's message, then where it is (the component name and the exact path in
quiet monospace, as a link that focuses the field). Paths wrap after slashes.

**Components.** Hairline-separated sections (none above the first) with 34 px sticky
headers: chevron, the component's icon in quiet grey, the display name and problem
counts as coloured icon and number. Display names come from the schema title when it has
words, otherwise from the profile or by splitting the identifier. The type ID is in the
tooltip and the section's accessible description.

**Rhythm.** Values are 28 px tall and 6 px apart. Captioned runs (Collider: Material,
Collision) start 10 px below the previous row with a caption and a trailing hairline.
Component bodies end with 16 px of air.

**Rows.** A label column of `clamp(88px, 32cqi, 168px)` of the component's width, an
8 px gap, and the value column. Labels are muted 12 px, wrap to two lines and clamp at a
third. A vector that wraps (a quaternion's X Y / Z W) keeps its label on its first line.
Field descriptions and union variant notes are the row's tooltip and spoken description,
not a visible help line.

**Nesting.** Only real nested objects (NavigationMesh build settings, script props)
indent 12 px with a guide line, and narrow their label column by the same step, so value
columns align. A union's variant fields continue under the selector at its own depth.

**Value boxes.** Quiet raised boxes (28 px, radius 5) with no border; focus draws the
ring. Units follow the number. Status words ("Default", "Unused") sit at the trailing
edge as plain quiet text. Unset optional values are an outline with "Not set" or "Off".
A stable ID (a part's own ID) is metadata: an outline and muted text instead of a fill.
Union selectors can lead with a glyph (collider box, sphere, capsule, compound).

**Errors.** An invalid value tints its box and draws a red hairline; a whole invalid
vector (a non-unit quaternion) tints every cell and outlines the box once.

**References.** An ID that exactly matches a project asset or entity shows that thing's
icon and name, then the authored ID in quieter monospace; the name is also spoken.

**Notices.** A value, field or component that cannot be presented as its schema says
shows a warning icon, one sentence and the raw JSON in a well. No dashed box or tint.

**Lists of records** (compound parts, navigation sources and links) are one Tab stop:
a header with the name and count on the left and the breakdown on the right ("Parts 64
… 39 box, 13 sphere, 12 capsule"; the full sentence is spoken), a sunken well of 26 px
rows (index, glyph, tag, compact values, problem mark) and, below a hairline, the
selected item: "Part 42 of 64" (its document path is the tooltip and spoken), then its
fields at the list's own depth. No paragraph explains what a compound is.

**Long number lists.** A grid of costs with a matching width × height draws as a pixel
map with a legend and a "Show values" disclosure; values open in a bounded, focusable
table. String lists are a bounded list of read-only boxes.

## Output dock

Text tabs with plain counts (red for errors). Each view starts with a 40 px toolbar.

**Pages.** The bridge caps none of these lists and their rows wrap to their full text, so
each view mounts at most one page of 100 rows. When a list is longer, the toolbar's
trailing edge says which slice is shown ("101–200 of 20,000") with first, previous,
next and last page buttons (at an end they stay focusable and are marked disabled). A
chosen page is announced from the pager itself (the status line does not repeat the
range) and starts at its top. Nothing is hidden: every entry is one page away, filters
count every entry, and a list that fits one page shows no pager at all.

- **Problems.** Severity filters (Errors, Warnings, Info, with counts) are flat toggles:
  pressed is filled with a coloured icon, released is quiet text. Problems are grouped by
  entity, worst first, project-level problems last; each group has a heading with the
  entity's kind icon, name and counts, and each row the severity icon, the message, and
  "Component · /path" in monospace, wrapping under the message when space runs out. All
  filtered out, and validated with nothing to report, are distinct states. A group that
  spans pages repeats its heading followed by "continued", keeps its whole-group counts,
  and ends with "312 more problems on Crate 03 on the next pages". A filter change
  returns to the first page.
- **Console.** Time, level as a coloured word (info, warn, error, debug), source and
  message, with level filters; debug is hidden by default. No row tints. It opens on the
  newest page and follows it as messages arrive; another page stays put (only its total
  grows) until **Newest** returns to the end.
- **History.** Oldest first: the description, the origin as a coloured word (User,
  Agent, Script, Import), the model or script, and the time in a fixed last column. A
  hairline marked "Now" separates applied edits from those Redo would restore; undone
  edits are struck through and say "(undone)". The toolbar has Undo, Redo (tooltips give
  the platform's keys) and "4 of 5 applied". The list is a keyboard stop so it can
  scroll. It opens on the page that holds Now and follows it; **Now** returns there.

## Agent pane

The header is a disclosure: chevron, wisp mark, "Agent", and a status (Ready, Starting…,
Working…, Needs approval, Stopping…, Finishing…, Updating…, Signing in…, Sign-in problem,
Unavailable, Allowance used up, Off, Not signed in, No engine). Work in progress shows the
small spinner in place of the dot; "Needs approval" is a steady amber dot. Files:
`components/AgentPanel.tsx`, `components/agent/`, `styles/agent.css`.

- **Conversation.** The native session's turns (`snapshot.agentSession`), oldest first,
  divided by hairlines. A turn is the prompt (the one filled block, so a conversation
  scans by request; past 8 lines it clamps with Show all), the reply exactly as streamed
  (plain text, never markup), one quiet line per decided approval, and how it ended: a
  failure sentence after the error icon, then Stopped, the edits ("No edits" only when
  the report says so), the allowance charged ("(estimated)" when exact usage could not be
  confirmed or saved, "Token use unknown" when nothing was charged) and Show in History.
  A send adds nothing until the session lists its turn. The conversation is a log but not
  a live region: approval requests and outcomes are said once in the status line, streamed
  text never. It follows new output only while the reader is at its end; otherwise a
  floating Latest returns there.
- **Allowance line.** The open pane's last line, under the composer or the note, outside
  the conversation so it never scrolls or moves with it: "7,712 of 250,000 tokens used",
  then, where the project has an authored allowance (`projectAgentBudget`), a quiet
  Change at the trailing edge of the first line. The allowance is the project's authored
  cap; usage is the session's recorded total, "(some estimated)" where part of it is a
  conservative charge. While the session catches up with a changed cap its own copy is
  never shown. The numbers turn amber at 80 %, red when used up, and stay muted at 0 (off
  on purpose). Usage above a lowered cap stays visible; only what is left (spoken and in
  the tooltip) stops at zero. There is no reset and no money: tokens only. Older hosts
  without a project allowance show their session's allowance without Change.
- **Changing it.** Change edits in place, as a rename does: "Allowance", the Inspector's
  28 px value box (monospace digits, "tokens" following them, all selected), Cancel and
  Save. Enter saves, Escape cancels, and focus returns to Change. Digits with one
  consistent group separator are read exactly; fractions, signs, exponents and values past
  9,007,199,254,740,991 are refused inline. One quiet line under the box says the single
  most important thing: why it cannot be saved, the host's reason a save failed (what was
  typed stays), "Saving stops the current request." while one runs (native hosts end it),
  that another edit changed the allowance meanwhile, "0 turns the agent off.",
  "No new requests: 7,712 already used.", or else the usage it is weighed against. A save
  is one `agent.budget.set` through the command bus: journaled, undoable, offline, and
  sent once however often Enter is pressed. Save is the raised button; Send or Stop keeps
  the one accent. A narrow pane, or a very long value, puts the actions beside the note.
- **Updating.** After an allowance edit, Undo or Redo, or an account read, the host holds
  new requests until it re-reads the session. The pane keeps the composer (its action a
  quiet wait) and says Updating… instead of swapping in a note; decisions wait with a
  reason while the session's cap lags the project's or the host says it is not ready for
  them ("Waiting for the agent to update its session."), and Stop stays usable. A refresh
  the host never finishes says Unavailable after 4 s (counted from the end of the pane's
  own save) and shows the host's reason and Refresh in the notice line above the same
  composer and draft, as a transport failure does while a worker runs.
- **Approvals.** A bordered card at the end of its turn: the request as a question read
  from the exact arguments ("Apply 4 changes to the project?"), the agent's own
  description in quotes, one row per operation marked like a diff (+ add, − remove,
  ~ change, ! unreadable), the exact arguments on request (wrapped, 64 KB until Show
  all) and Deny / Allow in a footer that sticks to the bottom of a short pane, where the
  question is kept in view. Untrusted names are isolated; hidden direction characters
  are shown as escapes. Nothing is focused or decided by itself; after a decision its
  row takes focus.
- **Composer.** One line that grows with the text; then the model (monospace, editable,
  default gpt-6-astra; no model list is invented), the approval mode (Ask before edits,
  Ask every time, Never ask in amber) and one action: Send, Stop while a turn can be
  cancelled, or a spinner while sending, stopping or finishing. It is one element, so
  focus stays on it, and the draft stays editable while the agent works.
- **Cannot run:** one sentence (why, or the host's reason) and its single next step
  (Continue with ChatGPT, the account dialog, or Retry after a transport failure; Retry
  then says the same sentence if the agent is still unavailable). Turns the session still
  holds stay readable above it, and the
  allowance line stays below it, so the allowance can be changed without an account. A
  used-up allowance says "Raise the allowance to send new requests." ("…to turn the agent
  on." at 0), its next step being the line's Change; an older host says "This session’s
  token allowance is used up." A reason that names an internal milestone is shown in plain
  words ("The agent is not available in the editor yet.") without a sign-in invitation
  that would not help.
- **Collapsed:** the 36 px status line. A window shorter than 720 px starts collapsed.

## Dialogs

Floating surfaces: 10 px radius, a dim scrim without blur, no entrance animation, focus
trapped, Escape closes and focus returns to the opener.

- **Keyboard shortcuts.** One searchable reference table, 600 px wide. The header holds
  the title, the filter (focused on open; it matches actions, groups and key names,
  shows "N of M", and Escape clears it before closing) and Close. Sections (General,
  Play, Hierarchy, Panels and layout, Lists, assets and inspector, Agent) are caption rows;
  each row is the action (with a quiet "where" when needed) and, right-aligned, the
  current platform's keys as plain text: "⇧⌘Z" on macOS, "Ctrl+Shift+Z" elsewhere, "or"
  between alternatives and "/" between paired directions. Hairlines between rows; no
  cards, caps or icons. Every listed key is implemented, and each combination is also
  spoken in words.
- **ChatGPT account.** 420 px. One status line only where the state needs words
  (checking, signed out, browser step, error); the accounts on this computer as plain
  rows between hairlines (avatar, label, "Signed in" for the active account, or a small
  Use / Sign in button); one row of actions (Add another account; Sign out at the
  trailing end, in danger colour). Sign-out asks inline and focuses "Keep signed in".
  The signed-out state adds the "Continue with ChatGPT" button and one footnote: "Your
  sign-in stays on this computer and is never saved in your projects." No status card,
  badge, section label or list of facts.

## States

Every panel has loading, empty, error and "no engine" states, each one quiet title
(13 px, muted) with a detail line only where it carries information (an error message,
the host's reason). An icon appears inline before the title only for errors and
warnings; work in progress shows a small spinner. No badge disc or pictogram. Wrapped
titles stay centred with their mark on the first line. While a project opens, the banner
says so once; the Inspector, viewport and dock keep "Opening project…" spoken-only, and
the hierarchy shows skeleton rows. **Fatal screens** (an incompatible bridge, an unknown
fixture) are plain text on the canvas: an error icon and the problem as the page's `h1`,
what happened in one sentence, and what to do next; valid fixture names are listed in
monospace.

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
- Play by physical key, from anywhere outside a dialog (the game view included): ⌘P
  plays or stops, ⇧⌘P pauses or resumes, ⌥⌘P steps while paused (Ctrl elsewhere). F6
  lands on the game view while a session exists.

## Motion

90 ms / 160 ms on hover, disclosure, switch and divider highlight; none under
`prefers-reduced-motion`. Spinners, skeleton shimmer and the agent's signing-in dot are
the only continuous motion, and they stop under reduced motion.

## Open design questions

- No light theme and no forced-colors (Windows high contrast) theme yet.
- No application menu (File/Edit/View) on Windows/Linux. See TITLEBAR.md.
- Editing: value boxes are shaped for future editing, but every field is read-only
  until the bridge offers a component command.
- Runtime values are shown only for Transform and Velocity, the fields native inspection
  returns. Other runtime components would need the same read-only block.
- Engine messages number compound parts from 0 ("compound part 41") while the Inspector
  numbers them from 1 ("Part 42 of 64"); the message text comes from Rust validation.
