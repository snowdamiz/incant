# Palette notes: landing page revision 3 (neutral)

These notes are for Astra to share with the editor packet. They record the cross-platform
colour decisions made on the landing page. The landing page follows
`references/previous-editor-tokens.css`; it does not define new editor tokens.

## Decisions

1. **The editor is neutral graphite.** The landing-page editor illustration uses the reference
   surfaces exactly, with no violet tint:

   | Role | Value | Reference token |
   |---|---|---|
   | Title bar, status bar, code wells | `#0b0c0f` | `--color-bg-app` |
   | Outline and inspector panels | `#131418` | `--color-bg-panel` |
   | Chat bubble, input, Play button | `#1a1c21` | `--color-bg-raised` |
   | Viewport behind the scene | `#0e0f12` | `--color-bg-viewport` |
   | Selected outline row | `#262b52` | `--color-bg-selected` |
   | Hairlines | `#22252b` | `--color-border` |
   | Control borders, 3:1 | `#676e7b` | `--color-border-control` |
   | Text and muted text | `#ececf1`, `#a9aeb9` | `--color-text`, `--color-text-muted` |
   | Accent: selection box, predicted path, agent mark | `#8c95ff` | `--color-accent` |
   | Agent history badge | `#d892f5` | `--color-origin-agent` |
   | Success, warning, danger | `#5dd39a`, `#f0bd55`, `#ff7a70` | status tokens |
   | Move gizmo axes | `#ff8a7f`, `#7fe0a3` | `--color-axis-x`, `--color-axis-y` |

2. **One accent hue across both products.** The editor accent is periwinkle `#8c95ff`, at a hue
   of 235°. The light marketing page uses the same hue darkened to `#4650c8`. That gives 5.8:1
   on paper `#f4f3ef`, so it also serves as the focus ring and text-selection colour. On dark
   page surfaces the page uses `#a9b0ff`, which is the editor's `--color-accent-strong`.

3. **Brand violet `#4b2bc7` is reserved for the wisp logo mark.** It appears in the header,
   footer and favicon only. It is no longer a background, button hover, focus ring, rule or
   emphasis colour. If the editor packet places the wisp in the titlebar, the landing page shows
   it in periwinkle `#8c95ff` inside the editor chrome. The editor packet may choose
   differently. If it does, the landing page should follow, and this is the one value to sync.

4. **Page neutrals lost their purple cast.** Ink moved from `#17151d` to `#15161a`, and muted text
   from `#5d5966` to `#5b5e66`. Paper moved from `#f4f1ea` to `#f4f3ef`. The statement band
   changed from solid violet to ink `#15161a`, with the wisp drawn in paper.

## Open question for the editor packet

The reference keeps `--color-bg-selected: #262b52`, an indigo-tinted selection pill. The landing
page matches it. If the editor packet neutralises that pill further, the landing page's selected
outline row should change with it.
