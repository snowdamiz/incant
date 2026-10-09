# Handoff 0001 result: editor foundation (revision 1, after director feedback)

## Status

**Review required.** Not self-approved. The phase gate and native rendering approval
remain with the director.

- The editor shell is implemented in `editor/ui` and redesigned per the director's
  priority revision. It builds, typechecks and passes all tests.
- Revision captures are real Chrome screenshots at 1280×800 and 1920×1080.
- **Native viewport and native titlebar evidence are pending.** No native window ran in
  this handoff, and `incant_headless screenshot` does not exist in this worktree.
- **Engine tests were not run.** `tools/cargo test --workspace` was declined twice at
  the permission prompt in this session. No files under `crates/`, `services/`,
  `tools/` or `sdk/` were changed.

## Model

- Model: Claude Opus 5.5, model ID `claude-opus-5-5`, running in Claude Code in this
  worktree on branch `handoff/0001-editor-foundation`.
- From inside the session I cannot tell whether it was launched through ACP or as the
  manual fallback. `status.json` was written by the handoff tool and was not edited.

## Director feedback acknowledged

The director said: "the design of the UI is not nearly good enough. It should look very
good, clean, easy to use, modern, custom titlebar, etc."

I agree with that assessment of the first design. It was correct and accessible, but it
looked like a form. Flat panels butted together, uppercase labels shouted, and the amber
focus and fixture styling were loud. The read-only and fixture notices dominated the
screen.

Revision 1 is a full redesign of composition, palette, typography, density and
components, not a set of cosmetic fixes. It adds a custom titlebar with a typed host
contract.

## What changed in revision 1

- **Composition.** Rounded panel "islands" sit on a darker canvas with 6 px gutters, and
  the gutters double as hidden resize handles. The titlebar and status line are part of
  the canvas.
- **Custom titlebar.**
  - It holds the logo, project name, a quiet "Sample data" chip, undo/redo, three layout
    toggles, provider status and a shortcuts button.
  - The whole bar is a drag area. Double-click maximizes. On macOS it reserves an inset
    for native traffic lights. On Windows and Linux it draws custom caption buttons.
  - Every window behavior goes through new typed `window.*` requests and is offered
    only when the host advertises the capability, so no control is faked.
  - `editor/ui/TITLEBAR.md` documents the contract.
- **Collapsible panels.** The hierarchy, output dock and inspector/agent column can be
  hidden from the titlebar or with ⌥⌘1/2/3, and F6 skips hidden panels.
- **Palette.** Cool graphite with one periwinkle accent. Focus is a soft ring and
  selection is a filled indigo pill, so they differ in shape. Every text pair still
  meets 4.5:1 and every focus and border pair meets 3:1, by test.
- **Typography.** Sentence-case semibold headings with count pills, tighter tracking,
  and a 15 px entity name.
- **Hierarchy.** Pill rows, indent guides, compact problem marks, collapse-all and an
  inline match count.
- **Inspector.**
  - The header shows a kind tile, the name, kind · ULID, and a copy-ID button.
  - "Read-only" is now a lock chip instead of a paragraph.
  - Values sit in field boxes with axis letters, units, link icons and swatches inside.
  - Booleans are read-only switches, and invalid fields get a red-tinted box.
- **Output dock.** Pill tabs, chip filters and dot-and-word provenance.
- **Agent.** Chat-style composer with an embedded send button. "Connect OpenAI…" sits
  in the composer bar, and no transcript is invented.
- **States.** A calm viewport placeholder with a dot field, tiled empty states, and one
  rounded notice strip for absent, connecting or lost engine.

## Before and after

The first design is kept untouched as the "before" evidence.

| View | Before (first design) | After (revision 1) |
| --- | --- | --- |
| Blank (pre-UI) | `screenshots/00-before-blank-1280x800.png` | — |
| Selection + error, 1280×800 | `screenshots/03-sample-selected-error-1280x800.png` | `screenshots/revision-1/03-sample-selected-error-1280x800.png` |
| Selection + error, 1920×1080 | `screenshots/03-sample-selected-error-1920x1080.png` | `screenshots/revision-1/03-sample-selected-error-1920x1080.png` |
| No engine | `screenshots/01-no-engine-1280x800.png` | `screenshots/revision-1/01-no-engine-1280x800.png` |
| Inspector mismatch/unsupported | `screenshots/04-inspector-unsupported-1280x800.png` | `screenshots/revision-1/04-inspector-unsupported-1280x800.png` |
| History | `screenshots/06-history-1280x800.png` | `screenshots/revision-1/06-history-1280x800.png` |

All paths are relative to `handoffs/0001-editor-foundation/`.

The improvements, comparing the 1280×800 captures:

- **Grouping and depth.** The islands give the panels clear grouping. The viewport is
  visibly the primary surface.
- **Quieter notices.** The read-only paragraph and the large fixture badge are gone.
  Both facts are still stated, now as a lock chip and a "Sample data" chip with full
  tooltip and screen-reader text, plus the status line.
- **Focus versus selection.** The first design ringed whole panels in amber. Now a soft
  ring sits on the focused row, and the selection pill is distinct from it.
- **Easier-to-scan values.** Inspector values now read as fields. Axis letters and
  units sit inside the box, booleans are switches, and the invalid field stands out
  without a banner.
- **Hierarchy structure.** Indent guides make depth readable, and the unboxed problem
  marks save width.
- **Usable agent at 800 px.** The first design hid the connect button below the fold.
  The revision keeps the action in the composer bar.

## Revision screenshots

All revision screenshots are in `handoffs/0001-editor-foundation/screenshots/revision-1/`:

- **Main views at 1280×800.** No engine, sample initial, selection with error,
  unsupported inspector fields, console, history, read-only explained, and the
  shortcuts dialog.
- **Main views at 1920×1080.** No engine, sample initial, and selection with error.
- **States.** Empty, loading, hierarchy error, connection error, large (2,041
  entities), and an unknown fixture name.
- **Titlebar variants.** Plain browser, macOS overlay double, Windows custom-controls
  double, Windows close hover, and the collapsed-panels view.
- **Native transparency check.** `12-native-viewport-transparent-1280x800.png`.

Measurements are in `screenshots/revision-1/evidence.json`. The first design's
measurements stay in `evidence.json` next to this file.

## Evidence

| Check | Result |
| --- | --- |
| `npm ci` at the root | passed, 0 vulnerabilities |
| `npm run typecheck --workspace editor/ui` | passed |
| `npm run build --workspace editor/ui` | passed |
| `npm test --workspace editor/ui` | 151 of 151 passed, in 5 files |
| axe-core in Chrome 155, contrast included | 10 runs, 0 violations |
| Network requests outside the local origin | none |
| Console errors or warnings, CSP included | 0 |
| Viewport hole alpha with a surface attached | 0 at the viewport center |
| Chrome alpha (hierarchy, titlebar, status line) | 255 |
| Requests recorded by the Windows double | minimize, drag, maximize |
| `tools/cargo test --workspace` | **not run**: declined at the permission prompt |

The tests cover:

- Token contrast for every pair.
- Tree keyboard model, filter and problem roll-up.
- Bridge resolution and unknown capabilities.
- Fixtures contain no credential-shaped values.
- Layout clamping.
- Shell integration with axe in jsdom. This includes commands going through
  `dispatch()` with no local mutation.
- Titlebar behavior:
  - Outside the host there are no controls or insets.
  - On macOS it reserves the inset and draws no buttons.
  - On Windows it draws only advertised buttons and sends typed requests.
  - Pressing a titlebar button does not start a drag.
  - Panel toggles work and F6 skips hidden panels.

Commands to reproduce:

```
npm ci
npm run build --workspace editor/ui
npm run typecheck --workspace editor/ui
npm test --workspace editor/ui
npm run evidence --workspace editor/ui    # needs local Chrome; writes screenshots/revision-1/
npm run icon --workspace editor/ui        # re-renders public/icon.png from public/icon.svg
```

What the evidence does not cover:

- jsdom tests are not layout evidence.
- The titlebar doubles and the attached-viewport double are labeled test doubles, not a
  host. A browser capture cannot show macOS traffic lights, only the inset reserved for
  them.

## Changed paths

- **Root workspace.** `package.json` adds `editor/ui` as a workspace, and
  `package-lock.json` pins all versions.
- **Packet and evidence.**
  - `handoffs/0001-editor-foundation/result.md`, `evidence.json` (first design) and
    `screenshots/` (first design).
  - `screenshots/revision-1/` holds the revision captures and their `evidence.json`.
- **Editor configuration.** `editor/ui/package.json`, `index.html`, `vite.config.ts`,
  `vitest.config.ts` and the four `tsconfig*.json` files.
- **Editor docs.** `DESIGN.md`, `TITLEBAR.md` and `NATIVE_VIEWPORT.md`.
- **Assets and scripts.** `public/icon.svg` and `public/icon.png` (512×512 application
  icon), plus `scripts/evidence.mjs` and `scripts/render-icon.mjs`.
- **Bridge.** `src/bridge/contract.ts`, `fixture.ts` and `resolve.ts`, with tests.
- **Shell.** `src/shell/ShellContext.tsx`, `layout.ts` and `regions.ts`, with tests.
- **Hierarchy logic.** `src/hierarchy/tree.ts`, with tests.
- **Components.** In `src/components/`: Titlebar, HierarchyPanel, ViewportPanel,
  BottomDock, AgentPanel, ProviderChip, StateView, Splitter and ShortcutsDialog. The
  inspector is in `src/components/inspector/`.
- **Icons and styles.** `src/icons/Icon.tsx`, `src/styles/tokens.css`, `app.css` and
  the token tests.
- **Entry and tests.** `src/App.tsx`, `src/main.tsx`, `src/App.test.tsx` and
  `src/test/setup.ts`.

`handoffs/0001-editor-foundation/brief.md` has the director's uncommitted packet
revisions. I left it out of my commit so it is not attributed to me.

## Requested integration from Astra

1. **Bridge contract.** `src/bridge/contract.ts` is a proposal. Replace it with the
   real `editor/bridge` types, including the new `WindowChrome` snapshot field,
   `window.*` capabilities and requests, and `viewport.bounds.cornerRadii`.
2. **Host window.** On macOS use an overlay titlebar with hidden title and centered
   traffic lights, reporting a 78 px inset. On Windows and Linux use
   `decorations: false`. Set the window background to `#0b0c0f`. See TITLEBAR.md.
3. **Native surface.** Clip it to the reported corner radii, and resolve the open
   questions on below/above layering, input and DPR in NATIVE_VIEWPORT.md.
4. **Missing commands.** A component edit command on `incant_cmd` (the inspector is
   read-only until it exists), an `entity.detail` paging request, and agent
   conversation events.

## Candid assessment of remaining visual issues

- **Native not seen.** Nothing has been checked on a native window. Traffic-light
  alignment, Windows 11 caption metrics, Snap Layouts, rounded surface clipping and
  window-blur dimming are untested by eye.
- **Inspector cramped at 1280×800.** It gets 448 px of height next to a 280 px agent,
  so long components scroll early. A future tabbed or stacked inspector/agent option
  would help.
- **Hierarchy width.** At the default 240 px, names with two problem marks still
  truncate, for example "Grapple C…".
- **Large tree not virtualized.** The 2,041-entity fixture is fine collapsed but would
  be slow fully expanded.
- **Empty agent space.** At 1920×1080 the agent island has generous empty space until
  real conversations exist.
- **Missing themes and menus.** There is no light theme, no forced-colors theme, and no
  application menu for Windows or Linux.
- **Minimum text size.** Status-line key hints render at 11 px. All content text is at
  least 12 px.
- **Placeholder viewport.** The empty viewport is a placeholder, so its look-dev has not
  been judged against real rendered scenes.

## Next steps

- Astra wires the host chrome and bridge, then runs the native window on macOS and
  Windows.
- Claude reviews native captures of the titlebar and viewport compositing in a
  follow-up packet.
- Design follow-ups: an inspector/agent layout option, hierarchy virtualization, the
  application menu, and light and high-contrast themes.
