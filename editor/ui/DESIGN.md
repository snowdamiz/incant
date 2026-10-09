# Incant editor design system, Phase 0 proposal (revision 1, "islands")

Status: proposal from handoff 0001 (Claude). Not approved. Revision 1 replaces the first
direction after director feedback that the first shell was not good enough visually.
Tokens: `src/styles/tokens.css`. Styles: `src/styles/app.css`. Titlebar host contract:
`TITLEBAR.md`. Native viewport: `NATIVE_VIEWPORT.md`.

## Principles

1. **The viewport is the content.** Chrome is quiet graphite; color is spent on status,
   provenance and one accent. The viewport island is the largest shape on screen.
2. **Calm, not empty.** Every panel has a designed state for loading, empty, error and
   "no engine", and each state includes a one-line explanation.
3. **Honest, but quiet.** Fixture data and read-only limits are always labeled, as small
   chips with full explanations in tooltips and screen-reader text, never as banners
   that dominate the work area. Unavailable actions stay focusable and explain why.
4. **Keyboard first.** Every action is reachable without a pointer. Focus is always
   visible and is a ring; selection is a fill.

## Composition

```
┌ titlebar (40, canvas) ─ logo / Project  [Sample data] ··· drag ··· ↶ ↷ │ ◧ ⬓ ◨ │ ● OpenAI │ ⌨ │ — ▢ ✕ ┐
│ ╭ Hierarchy ╮ ╭ Viewport ─────────────────────╮ ╭ Inspector ─────────╮ │
│ │ filter    │ │                               │ │ entity · id · copy │ │
│ │ tree      │ │   native surface (hole)       │ │ components         │ │
│ │           │ ╰───────────────────────────────╯ ╰────────────────────╯ │
│ │           │ ╭ Problems · Console · History ─╮ ╭ Agent ─────────────╮ │
│ │           │ │                               │ │ composer           │ │
│ ╰───────────╯ ╰───────────────────────────────╯ ╰────────────────────╯ │
└ status line (26, canvas) ─ source · problems · live messages ··· F6 · ? ┘
```

- **Islands:** each panel is a rounded (10 px) surface on a darker canvas, separated by
  6 px gutters. The gutters are also the resize handles, invisible until hovered or
  focused.
- **Titlebar** is part of the canvas, not a separate toolbar strip. It holds identity,
  history, layout toggles, provider status and shortcuts. Window controls follow the
  platform (see TITLEBAR.md).
- **Collapsible panels:** the three titlebar layout toggles (and ⌥⌘1/2/3) hide the
  hierarchy, output dock or inspector/agent column; the viewport takes the space.
- Defaults (`src/shell/layout.ts`): hierarchy 18 % of width (240–320 px), inspector
  22 % (320–420 px), dock 28 % of height (160–360 px), agent 34 % (280–360 px).
  The viewport never drops below 360 × 200.

Measured viewport island (Chrome, sample fixture):

| Window | Viewport island | Inspector island |
| --- | --- | --- |
| 1280×800 | 696×504 | 320×448 |
| 1920×1080 | 1156×706 | 420×648 |

## Color

Dark only for Phase 0. Every pair below is enforced by `src/styles/tokens.test.ts` and
re-checked in Chrome by axe-core `color-contrast`.

| Role | Token | Value | Rule |
| --- | --- | --- | --- |
| Canvas | `--color-bg-app` | `#0b0c0f` | titlebar, gutters, status line |
| Island | `--color-bg-panel` | `#131418` | panel surface |
| Raised | `--color-bg-raised` | `#1a1c21` | fields, chips, selected tab |
| Hover | `--color-bg-hover` | `#202329` | |
| Selection | `--color-bg-selected` | `#262b52` | indigo-tinted pill |
| Text | `--color-text` | `#ececf1` | ≥ 4.5:1 on every surface |
| Muted | `--color-text-muted` | `#a9aeb9` | ≥ 4.5:1 on every surface |
| Subtle | `--color-text-subtle` | `#959cab` | ≥ 4.5:1 on every surface, incl. selection |
| Accent | `--color-accent` | `#8c95ff` | periwinkle: primary buttons, switches, active tab icon |
| Focus | `--color-focus` | `#c3c8ff` | ring, ≥ 3:1 on every surface |
| Control border | `--color-border-control` | `#676e7b` | ≥ 3:1 |
| Status | danger / warning / success / info | `#ff7a70` / `#f0bd55` / `#5dd39a` / `#7ab8ff` | ≥ 4.5:1 on surfaces and tinted backgrounds |
| Provenance | user / agent / script / import | blue / orchid / green / amber | always with a word |

Color is never the only cue: problems carry icons and counts, provenance carries a
word, switches carry "On/Off", axes carry letters.

## Typography

Inter Variable for the UI and JetBrains Mono Variable for ids, paths, numbers and code.
Both are bundled locally, so nothing is fetched from a network.

- 13 px default and 12 px minimum for content. Status-line key hints use 11 px; that is
  the measured minimum.
- 14 px for inputs, chat and state titles, 15 px for the inspected entity name, 16 px
  for dialog titles, 20 px for full-screen errors.
- Headings use sentence case with −0.011 em tracking. The first design's uppercase
  tracked labels were removed.
- Weights are 420, 530 and 620. Tabular numerals are used for counts and values.

## Space and size

- Spacing follows a 4 px grid.
- Rows are 26 px, fields 28 px, panel headers 40 px, the titlebar 40 px and the status
  line 26 px.
- Gutters are 6 px and tree indent is 14 px.
- Radii are 4, 6 and 10 px for small, control and island. The dialog uses 14 px.

## Components

- **Hierarchy:** pill rows inset 6 px from the island edge, with one hairline indent
  guide per ancestor level. Selection is an indigo pill with a medium-weight name.
  Problem marks are unboxed icon and count to save width, and collapsed parents show a
  dot for problems inside. Collapse-all sits next to the filter, and the filter shows
  its match count inline.
- **Inspector:**
  - The header shows the kind tile, entity name, kind · ULID and a copy-ID button.
  - "Read-only" is a quiet lock chip in the panel header.
  - Components are disclosure rows, and fields sit in a 34/66 label/value grid.
  - Values sit in 28 px field boxes with prefixes and suffixes inside the box: axis
    letters, link icons, swatches and units.
  - Booleans are read-only switches. Invalid fields get a red-tinted box plus a message
    under the value.
- **Output dock:** pill tabs with counts. Console level filters and history undo/redo
  are chips. History provenance uses colored dots with words.
- **Viewport:** a calm dotted field with a vignette, and a centered tile explaining the
  state. With a surface attached, the hole is fully transparent and only the island
  header is painted.
- **Agent:** a centered mark and one sentence. The composer is a rounded field with the
  send button inside. When signed out, a white "Continue with ChatGPT" button sits in
  the composer's action bar; while signing in or after an error, a status button
  reopens the account dialog. No transcript is ever invented.
- **ChatGPT account (handoff 0003):** the titlebar chip shows a status dot, "ChatGPT"
  and the account or state, and opens a 460 px modal. The modal has one status row
  (36 px badge, headline, one sentence), host errors verbatim with their code on a
  second mono line, a bordered list of saved accounts where only a signed-in account
  is filled and marked "In use" (signed-out registrations offer "Sign in", never an
  active mark), then actions and three facts: you stay signed in across restarts and
  new builds, sign-in is never in projects and the computer may ask once for access
  after Incant changes, and everything but the agent works offline. While adding an
  account, a note says the current one stays in use and cancelling keeps it. "Continue with ChatGPT" uses OpenAI's
  white button format. Sign-out asks inline first. Escape closes without cancelling a
  sign-in; "Cancel sign-in" is explicit. No field ever accepts a token, code or key.
- **Notices:** absent, connecting and lost engine states show as one rounded strip
  under the titlebar. Lost connection uses an alert role.

## Focus and keyboard

- Focus ring: 2 px `--color-focus` (`:focus-visible`), inset inside lists and islands so
  it is never clipped. Field boxes ring on `:focus-within`.
- F6 / Shift+F6 cycle visible panels (hidden panels are skipped). Hierarchy is an ARIA
  tree with roving tabindex, type-ahead, F2 rename, Delete. Tabs use arrow keys.
  Dividers are `separator`s resizable with arrows (Shift = 64 px). `?` opens the
  shortcut list (modal, focus trapped, Escape returns focus). A skip link is first.

## Motion

90 ms / 160 ms on hover, disclosure, switch and divider highlight; none under
`prefers-reduced-motion`.

## Revision 1 vs. first design

| Area | First design (`screenshots/`) | Revision 1 (`screenshots/revision-1/`) |
| --- | --- | --- |
| Frame | Flat panels butted together with 1 px lines; looked like a form | Rounded islands on a canvas; clear grouping and depth |
| Titlebar | Generic toolbar strip with a large amber fixture badge | Integrated custom titlebar: drag area, platform insets/controls, layout toggles |
| Hierarchy | 24 px rows, full-bleed selection with left bar, boxed badges | 26 px pill rows, indent guides, compact problem marks, collapse-all |
| Inspector | Big blue "read-only" paragraph, grey component bands, bare inputs | Quiet lock chip, identity header with copy, field boxes with inline axis/unit, switches |
| Focus | Amber outline that boxed whole panels loudly | Soft periwinkle ring; selection is a fill |
| Agent | State view plus a disabled textarea with a separate Send button | Chat-style composer; connect action where the user acts |
| Fixture label | Loud badge in the top bar | "Sample data" chip with full tooltip, plus the status line |
| Headers | Uppercase tracked labels | Sentence case, semibold, with count pills |

## Open design questions

- No light theme and no forced-colors (Windows high contrast) theme yet.
- There is no application menu (File/Edit/View) on Windows/Linux. See TITLEBAR.md.
- Long entity names with two problem marks still truncate at 1280 px in the default
  240 px hierarchy.
- No measured platform performance budgets were supplied for the webview. The bundle
  is 283 kB JS (88 kB gzip) and 39 kB CSS (8 kB gzip).
