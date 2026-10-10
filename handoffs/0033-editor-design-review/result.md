# 0033 full editor design pass: result

## Status

Revision 5 ("plain") is implemented and reviewed in the browser. It answers the
director's rejection of ornamental UI (packet of 2026-10-10, top of `brief.md`). Native
review is pending Astra's rebuild from this commit.

- **The five points: done in the browser.**
  1. *Useless text and rows.* The compound paragraph is gone. The Inspector's Entity /
     ID / Read-only row is gone. Comparable sentences and rows went across the app
     (matrix below).
  2. *Account dialog.* It is now a compact list: one status line only where the state
     needs words, the accounts as plain rows, and one row of actions.
  3. *Keybinds.* The card grid is now one searchable table, with keys as plain text in
     the platform's notation.
  4. *Account badge.* The long pill is now the avatar alone: the signed-in initial in its
     ring, or a profile glyph.
  5. *Ornament.* Removed across the app: icon tiles, badge discs behind empty-state
     icons, accent edges, illustrations, keycaps, pills, dashed notice boxes, row tints,
     timeline dots and entrance animations.
- **Measured.**
  - Visible words fall 13.6 % across the same 96 browser states: 19,478 → 16,820.
  - The first Inspector value sits 30–91 px higher (median 31 px) in 54 of 61 inspected
    states, and lower in none.
  - Before and after alike: 0 axe violations, page errors, remote requests, window
    scroll or step errors.
- **Behaviour kept.**
  - Astra's correctness patches, the bounded lists, paging and the diagnostic reveal
    are unchanged, as are every command-bus mutation, offline use and the auth logic.
  - Copy ID remains in the Inspector header and the hierarchy menu. "Read-only" is still
    spoken and still in the tooltip.
  - No Rust, schema, transport, credential storage or dependency changed.
- **Checks.** 29 test files and 499 tests pass, with the strict typecheck clean and the
  production build working. The count was 500 at 5cfa59d: one pager test was added, and
  two contrast tests went with the two tints they checked.
  - Tests changed only where the presentation changed on purpose; their behavioural
    assertions remain.
  - New test: the pager's own announcement. New assertions: the loading state is spoken
    once and not repeated on screen, and the fixture label has contrast on the titlebar
    canvas.
- **Native acceptance: not claimed.** All evidence is headless Chrome. The two native
  captures from 6db60b2 show revision 4, now superseded. The remaining native checks are
  listed exactly in "Remaining native checks".
- **Earlier work stands as recorded below:** main-based integration, revision 4, the
  correctness follow-up and the End-then-Tab fix. Revision 4's visuals are superseded
  wherever the revision 5 matrix says so.

No phase gate is approved or claimed.

## Model and effort

- Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Thinking effort: Max. The session reports reasoning effort "max" to the model. The
  runner selects and verifies the model and effort through ACP. I cannot inspect the
  ACP transport from inside the session.
- No model or effort substitution occurred.

## Packet revisions acknowledged

| Revision | What it asked | What I did |
| --- | --- | --- |
| Director rejects ornamental UI, 2026-10-10 | Remove useless text and rows app-wide; redesign the account dialog and keybinds; avatar-only account button; remove decorative edges, glows, tiles, nesting and prose; keep diagnostics, behaviour and a professional density | Revision 5 across every surface (next section). I read the five screenshots in the ignored `artifacts/0033-native-main/director-feedback/` first. |
| Native review update, 2026-10-10 17:45 | Review `compound-first.png` and `grid-current.png` (6db60b2); keep raw captures local; a focused delta review | Both show revision 4 as in the browser. The compound shows Part 1, so it is not an End check. The grid shows its values collapsed. No native-only defect was visible. Their ornament (meta row, paragraph, check disc, account pill) is removed in revision 5. The captures stayed ignored and none is committed. |
| Main-based integration, 2026-10-10 16:55 | Match main's Camera schema; keep the design | Done in e69f2f8; see the history below |
| Plan revisions 4–6 | Bounded large data; play kept out of authoring; hosted Device Farm phones | Unchanged: every list bounded; no Play control; no device claim |
| Director feedback, 2026-10-10 14:32 | Improve the whole app, not one sidebar | Revision 4, now simplified by revision 5 |
| Correctness follow-up (Astra) | Bounded dock rows; Copy ID never fails silently | Kept. The page announcement moved from the status line into the pager. |

## Plain pass (revision 5)

`editor/ui/DESIGN.md` is rewritten as revision 5. Its principles are no ornament, say
it once, identity costs no height, and spoken-only text where the screen no longer
repeats a fact.

### Coverage matrix

Paths are under `screenshots/plain/before/` and `after/`. "Before" is 5cfa59d: revision
4 with Astra's correctness fixes, built from an extracted copy. "After" is this commit.
Both come from the same 96 browser states, data and sizes. Account data comes from the
synthetic provider fixtures.

| Surface | Removed | Now | Evidence |
| --- | --- | --- | --- |
| Titlebar | Capsules around the toolbar groups; the amber "Sample data" capsule; the scene crumb that repeats the project name | Plain groups. Pressed panel toggles stay filled. "Sample data" is plain amber words. The scene appears only when it differs from the project | `no-selection-normal-1440x874.png`, `no-selection-minimum-1000x650.png` |
| Account button | The long pill: provider name, email or status words, status chip | One 28 px button: the signed-in initial in its ringed avatar, kept as the director asked, or a profile glyph. A corner dot marks signing in (amber) or a problem (red). The name is spoken, e.g. "ChatGPT account, signed in as ada@example.com"; the tooltip adds an error's message | `account-signed-in-normal-1440x874.png`, `agent-error-normal-1440x874.png` |
| Left column tabs | Count pills on Hierarchy and Assets | Plain counts beside the tab names | any full capture |
| Hierarchy | The 2 px accent edge on the selected row | The indigo selection pill alone; problem counts stay plain | any full capture |
| Context menu | "Del" on macOS; keycap styling | The platform's notation as plain text: ⌘⌫ on macOS, Delete elsewhere | `context-menu-normal-1440x874.png` |
| Viewport | The drawn grid-and-cube illustration, the dot grid and vignette, the pulsing pill, the second sentence of each state | The header pill (dot and word). The empty area shows only the host's reason or a render error's message | `no-engine-normal-1440x874.png`, `project-error-normal-1440x874.png` |
| Inspector header | The Entity / ID / Read-only row; the 22 px kind tile | The kind glyph in its tint, the name, a lock glyph (tooltip "Read-only: values can be inspected but not edited yet", spoken "Read-only") and Copy ID. Kind and full ID are the name's accessible description | `camera-rig-normal-1440x874-inspector.png` |
| Entity problems | The tinted card | A plain list of severity icon, message and location link | `native-malformed-normal-1440x874-inspector.png` |
| Component headers | Family-tinted icon tiles and the family colour tokens; count pills | A quiet icon, the name, and counts as a coloured icon and number. No hairline above the first section, so it no longer doubles the header's line | `native-character-normal-1440x874-inspector.png` |
| Field help | Help lines under values, including "One body and material shared by a bounded union of local primitive parts." | Descriptions and variant notes become the row's tooltip and spoken description | `native-rubble-parts-normal-1440x874-inspector.png` |
| Value tags | The outlined "Default" / "Unused" chip | Plain quiet words at the box's far end | `camera-rig-normal-1440x874-inspector.png` |
| Notices | Dashed warning box and tint | A warning icon, one sentence and the raw JSON in a well. Every diagnostic is still shown | `native-malformed-normal-1440x874-inspector.png` |
| Compound parts | The explanation paragraph; the visible document path "/shape/parts/41" beside the caption | "Parts 64" with the breakdown "39 box, 13 sphere, 12 capsule". The full sentence is spoken. "Part 42 of 64" carries its path as tooltip and spoken text | `native-rubble-parts-{normal-1440x874,minimum-1000x650}-inspector.png`, `railing-parts-normal-1440x874-inspector.png` |
| Asset details | The kind/usage metadata row; the "From the project folder" subtitle; the filled triangle | Header description (kind and usage, spoken). Identifiers uses the same chevron as every disclosure | `asset-texture-normal-1440x874.png`, `import-form-normal-1440x874-inspector.png` |
| Dock tabs and filters | Tab icons, count pills, outlined filter chips | Text tabs with plain counts (red for errors). Flat filter toggles, pressed ones filled | `dock-console-normal-1440x874.png` |
| Problems | The "continued" pill | "continued" as quiet words | `dock-paged-problems-minimum-1000x650.png` |
| Console | Level badges and error-row tints | The level as a coloured word | `dock-console-normal-1440x874.png` |
| History | Timeline dots and line; origin chips; hollow dots for undone edits | Description, origin as a coloured word, detail and time. A hairline marked "Now". Undone edits are struck through with "(undone)". Undo and Redo tooltips give this platform's keys only | `dock-history-normal-1440x874.png` |
| Pager and status line | The status line's persistent "Showing problems 101–200 of 450" echo | The pager announces the chosen page from its own hidden live region; its range stays visible | `dock-paged-problems-minimum-1000x650.png` |
| Status line | Key-hint caps (F6 Panels, ? Shortcuts) | Source, problem counts and the live message | any full capture |
| Banner | "You are looking at the editor shell." | "No engine connected. Open a project from the Incant editor to start." | `no-engine-normal-1440x874.png` |
| Agent pane | Note titles ("Not in the editor yet"), the offline sentence, the empty-transcript mark, title and body | One sentence and its single next step. Empty transcript: "Describe a change. Every edit lands in History and can be undone." | `agent-checking-normal-1440x874-agent.png` |
| Account dialog | Status card and badge; "Accounts on this computer" label; boxed account rows; the three icon facts; "In use" chips | A 420 px dialog. A status line only for checking, signed out, the browser step and errors. Account rows between hairlines: avatar, label, then "Signed in" or a Use / Sign in button. One action row: Add another account, then Sign out at the trailing end. Inline sign-out question, focus on "Keep signed in". Signed out adds Continue with ChatGPT and one footnote | `account-dialog-…`, `account-saved-…`, `account-signed-in-…`, `agent-error-normal-1440x874.png` |
| Keyboard shortcuts | The header mark and subtitle; five cards in two columns; tinted group icons; keycaps | One 600 px table. The header holds title, filter ("N of M") and Close. Sections are caption rows. Each row is the action (with a quiet "where") and the keys as plain text ("⇧⌘Z" / "Ctrl+Shift+Z"), with "or" and "/". Each combination is also spoken in words. About 29 px rows; 17 rows and 3 captions fit at 1440×874 | `shortcuts-dialog-normal-1440x874.png`, `shortcuts-filter-minimum-1000x650.png` |
| Dialog frame | Scrim blur, entrance animation, 14 px radius | Dim scrim, 10 px radius, no motion. Focus trap and focus return unchanged | same |
| Empty, loading, error states | 48 px tinted badge discs with pictograms (the "glow" behind empty-state icons); explanatory second lines | One quiet title. An inline icon only for errors and warnings, a small spinner for work. A wrapped title keeps its mark on the first line. While a project opens, the banner says so once and panels keep "Opening project…" spoken-only | `project-loading-…`, `hierarchy-error-…`, `empty-project-…`, `no-selection-normal-1440x874.png` |
| Fatal screens | The card, product mark, error tile, side-by-side version boxes, fixture chips, "The UI will not guess at an unknown protocol." | An error icon and the problem as `h1`, one sentence, the next step; fixture names as a plain monospace list | `incompatible-normal-1440x874.png`, `bad-fixture-normal-1440x874.png` |
| Tokens and CSS | Family colours, the unused warning and fixture tints, the unused accent button variant, `--text-xl`, the stale `.quiet-chip` selector | `tokens.css` is revision 5. Contrast tests cover the fixture label on the canvas | `editor/ui/src/styles/` |

### Kept on purpose

- **Information colour.** Hierarchy kind tints, History origin words and severity colours
  identify things; they never fill a decorative surface.
- **Structure lines.** Tree indent guides and the guide line of real nested objects show
  nesting.
- **The avatar's ring.** The director asked for "the profile icon like it is now". The
  ring is a 1.5 px solid presence mark, not a glow.
- **Truthful labels.** "Sample data" in the titlebar, "Read-only" in the header's tooltip
  and spoken name, and the unavailable reasons on disabled actions.
- **One privacy fact on the sign-in surface.** "Your sign-in stays on this computer and
  is never saved in your projects."
- **Behaviour promises.** For example "Reads the source file again. The name and ID stay
  the same." under Reimport.
- **Every diagnostic.** The entity problem list, inline field errors, raw values, error
  codes and paths are all still visible.
- **Activity motion.** Spinners, skeleton shimmer and the agent's signing-in dot stop
  under reduced motion.

### Behaviour and accessibility preserved

- **Identity.** Copy ID stays in the Inspector header and the hierarchy menu, with its
  clipboard handling.
  - The header's accessible description gives kind and full ID ("Camera, ID 01J…").
  - The lock glyph speaks "Read-only". No control suggests editing.
- **Account.** All of it is intact:
  - switching and adding accounts, signing in to a saved account, and sign-out with
    inline confirmation and safe focus;
  - Cancel sign-in, Open browser again, and the checking and validating spinners;
  - error notices with message and code;
  - focus trap, initial focus and focus recovery, and `aria-describedby` status.
- **Shortcuts.** All 29 implemented entries are listed. Bindings follow the platform
  the native host reports (else the browser's user agent), with the Windows/Linux
  alternatives. The filter matches action, group and key words, and Escape clears it
  before closing.
- **Bounded rendering.** Paging and the diagnostic reveal are untouched; the pager owns
  its live region now.
- **Tests adjusted for intentional presentation changes:**
  - `AccountDialog.test.tsx` asserts the button's accessible name instead of chip text,
    the single footnote, and "Signed in" instead of "In use".
  - `ShortcutsDialog.test.tsx` asserts table sections and combos as written.
  - `AgentPanel.test.tsx` asserts one-sentence notes.
  - `ProjectStates.test.tsx` asserts the loading title is spoken-only in panels.
  - `paging.test.tsx` asserts the pager's own announcement.
  - `tokens.test.ts` drops the removed tints and adds the fixture label on the canvas.

### Measurements

The same 96 states in headless Chrome 155 at device scale 2: sample fixture, synthetic
provider fixtures and the evidence test double. Per-state data is in
`evidence/plain-comparison.json`.

`visibleWords` counts words of visible text inside the window and every clipping
ancestor; spoken-only text is excluded. It also falls when less of a scrolling list fits
on screen. The shortcuts table shows 17 rows at once where the card grid showed about
26.

| All 96 states | Before (5cfa59d) | After |
| --- | --- | --- |
| Visible words, whole page | 19,478 | 16,820 (−13.6 %) |
| … titlebar | 886 | 459 |
| … Inspector | 3,410 | 2,962 |
| … agent pane | 1,600 | 974 |
| … status line | 949 | 553 |
| … dialogs | 712 | 379 |
| … viewport | 2,088 | 1,781 |
| First Inspector value, 61 inspected states | — | 30–91 px higher (median 31) in 54; unchanged in 7 (scrolled to a component); lower in 0 |
| Axe violations, step errors, page errors, remote requests | 0 | 0 |
| States where the window scrolls, horizontal page overflow | 0 | 0 |
| Clipped values | 36 | 36, the same long asset paths in reference boxes |
| Most dock rows mounted | 102 | 102 |

| State | Visible words before → after |
| --- | --- |
| Account dialog, signed in (dialog only) | 73 → 17 |
| Account dialog, signed out with saved accounts (dialog only) | 107 → 59 |
| Keyboard shortcuts (dialog only) | 211 → 102 |
| No engine (page) | 92 → 44 |
| Project opening (page) | 63 → 26 |
| Empty project (page) | 115 → 65 |
| Rubble Pile parts (Inspector) | 116 → 102 |

Malformed Props shows more Inspector words (87 → 98) because more of its diagnostics
now fit on screen.

### Remaining native checks

None of these is claimed. Please rebuild from this commit and check, with account-free
crops for anything committed:

1. **Titlebar, signed in.** The avatar alone, with its ring and no label. VoiceOver reads
   "ChatGPT account, signed in as …". Check the corner dot while signing in and after an
   error.
2. **Account dialog.**
   - Signed in with two or more accounts: rows, Use, Add another account, Sign out.
   - The sign-out question: focus starts on "Keep signed in".
   - The browser step with Cancel sign-in, and an error state.
   - All at 1440×874 and 1000×650.
3. **Keyboard shortcuts** at 1440×874 and 1000×650. Check the macOS glyph rendering in
   WebKit (⇧⌘Z, ⌥⌘1, ⌘⌫) and filtering by "hierarchy".
4. **Compound End then Tab** (carried over). In 64 Parts, select Part 1, press End, then
   Tab.
   - The trailing rows are visible with no blank region.
   - AX lists about Parts 52–64, not 1–13 plus 64.
   - Repeat with the window not frontmost.
5. **Grid values paging.** In Large Navigation Grid, open Show values and page or scroll
   through the 65,536 values; the table stays bounded and focusable.
6. **Inspector header.** VoiceOver reads the name, then "Entity, ID …", and the lock as
   "Read-only". Copy ID copies the full ID.
7. **Context menu and dock.** The context menu shows ⌘⌫ for Delete. A long Problems or
   Console list leaves the status line free of "Showing …" after paging.
8. **Project opening.** The banner shows "Opening project…" once; panels are blank apart
   from the hierarchy skeleton.
9. **Minimum window, 1000×650,** for the Inspector with 64 Parts, the account dialog
   and the shortcuts dialog.

Revision 5 changes no Rust, bridge or host code, so the native viewport, the titlebar
inset and window controls should be unchanged; a capture confirms it.

### Changed paths (this commit)

UI code (`editor/ui/src`):
- **Shell:**
  - `App.tsx`: status line without key hints; plain fatal screens; shorter banner and
    protocol text.
  - `components/Titlebar.tsx`: scene crumb only when it differs.
  - `components/ProviderChip.tsx`: avatar-only account button.
  - `components/ViewportPanel.tsx`: no illustration; loading spoken-only.
  - `components/StateView.tsx`: no badge; inline marks; `quiet`.
  - `bridge/provider.ts`: spoken account summaries.
  - `icons/Icon.tsx`: profile glyph.
- **Hierarchy and assets:**
  - `components/HierarchyPanel.tsx`: copy; platform Delete label.
  - `components/assets/AssetBrowser.tsx`: copy.
  - `components/assets/AssetInspector.tsx`: no meta row; description; chevron.
- **Inspector** (`components/inspector/`):
  - `InspectorPanel.tsx`: no meta row; read-only mark; plain headers, problems and
    loading.
  - `IdentityHeader.tsx`: plain glyph; accessible description.
  - `FieldView.tsx`: notes as tooltip and spoken text.
  - `ObjectList.tsx`: count and breakdown; spoken sentence and path.
  - `presentation.ts`: families removed.
- **Dock:** `components/BottomDock.tsx` (text tabs) and, in `components/dock/`,
  `ProblemsView.tsx`, `ConsoleView.tsx`, `HistoryView.tsx` and `paging.tsx`. The pager
  announces itself; History drops its timeline and gives this platform's keys in its
  tooltips.
- **Dialogs and agent:**
  - `components/AccountDialog.tsx`: compact body.
  - `components/ShortcutsDialog.tsx`: table; `comboText`; exported `usePlatform`.
  - `components/AgentPanel.tsx`: one-sentence notes.
- **Styles:** `tokens.css`, `base.css`, `shell.css`, `hierarchy.css`, `dock.css`,
  `inspector.css`, `agent.css`, `dialogs.css`, `assets.css`.
- **Tests:** `ProjectStates.test.tsx`, `components/AccountDialog.test.tsx`,
  `components/AgentPanel.test.tsx`, `components/ShortcutsDialog.test.tsx`,
  `components/dock/paging.test.tsx`, `styles/tokens.test.ts`.
- **Documents:** `editor/ui/DESIGN.md`, revision 5.

Handoff (`handoffs/0033-editor-design-review/`):
- `tools/capture.mjs`: a `visibleWords` measure per area.
- `tools/curate-plain.mjs`: new.
- `screenshots/plain/before/` and `after/`: 29 pairs, unedited and account-free.
- `evidence/plain-comparison.json` and `evidence/plain.sha256` (58 OK).
- This result.
- `brief.md`: the runner's packet updates.

Not committed (ignored `artifacts/`):
- `0033-native-main/`: the director's screenshots and native captures, which show the
  account.
- Raw browser sets `plain-before`, `plain-final`, `r5-before`, `r5-after`, `p1`–`p5`.
- The 5cfa59d extract `src-5cfa59d`.

No Rust, SDK, CI, schema, transport, auth storage or account setting changed. No Cargo
command ran, and no dependency was added.

### Commands and results

```sh
cd editor/ui
npx tsc -b --noEmit                         # strict: 0 errors
npx vitest run                              # 29 files, 499 tests passed
npm run build                               # built
cd ../..
# before: 5cfa59d, extracted (git archive, as for e8ee0d6) and built in the ignored artifacts directory;
# its 91 tracked UI and bridge source files were checked byte-identical to git
git archive 5cfa59d editor/ui editor/bridge | tar -x -C artifacts/0033-editor-design-review/src-5cfa59d
(cd artifacts/0033-editor-design-review/src-5cfa59d/editor/ui && npx vite build)
node handoffs/0033-editor-design-review/tools/capture.mjs plain-before --root=artifacts/0033-editor-design-review/src-5cfa59d/editor/ui
node handoffs/0033-editor-design-review/tools/capture.mjs plain-final   # 96 captures, 0 step errors, 0 axe
node handoffs/0033-editor-design-review/tools/curate-plain.mjs plain-before plain-final
(cd handoffs/0033-editor-design-review && shasum -a 256 -c evidence/plain.sha256)   # 58 OK
git diff --check                                                         # clean
```

Every capture server used the first free port from 4233 and was closed afterwards.
- **Not run:** Cargo, Clippy, GPU tests, native capture, native WebKit, VoiceOver, phones.
- **Native:** I drove no native UI. The two 6db60b2 captures and the director's
  screenshots were read from the ignored directory only.

## Requests for Astra

1. **Native checks for revision 5:** the list in "Remaining native checks". I will
   review the captures and commit only account-free crops.
2. **Part numbering in engine messages.** Rust validation numbers compound parts from 0
   ("compound part 41 rotation must be a unit quaternion"), while the Inspector numbers
   them from 1 ("Part 42 of 64"). Please consider 1-based numbering, or "index 41", in
   the message text.
3. **Output lists in snapshots: still open.** The UI pages Problems, Console and History,
   but every snapshot carries every console message and history entry.
4. **When pixel-perfect lands on main,** switch `PixelCameraFields.test.tsx` to the
   native schema, as its tripwire test will prompt.
5. **Carried over:**
   - entity kinds for lights and audio;
   - nullable `type` arrays;
   - nested-object order, units and human titles in the schema registry;
   - snapshot reuse at 100,000 entities.

## Limitations

- **Browser-only evidence.** Native WebKit text rendering, the titlebar, the viewport and
  VoiceOver were not checked for revision 5.
- **Word counts are a proxy.** They measure on-screen text, not quality. They also fall
  when fewer rows of a list fit.
- **Spoken-only text.** Some facts moved from the screen to tooltips and descriptions:
  field help, variant notes, part paths, kind and ID. Pointer users get the tooltip,
  keyboard and assistive users the description. A sighted keyboard user without a
  screen reader sees them only by hovering.
- **Not the performance gate.** The browser timings of earlier passes come from one fast
  desktop machine. This pass changed no data path.
- **Read-only Inspector.** Every field stays read-only until the bridge offers a
  component command.
- **Pixel-perfect only synthetic here.** The real schema, renderer and evidence remain on
  `handoff/0033-pixel-integration`.
- **Pages, not continuous scrolling,** in the dock, as before.

## Open questions

- Should field help (now tooltip and spoken) get an explicit "info" affordance for
  keyboard users who do not use a screen reader?
- Should the signed-out account dialog keep both "Continue with ChatGPT" and "Add
  another account"? Both start browser sign-in, but the bridge distinguishes adding
  (`add: true`), so both stay until the host defines the difference for signed-out
  users.
- Should the dock remember a chosen page per tab across tab switches?
- Should a short window keep the agent collapsed, or should the user's choice persist?

## Earlier in this handoff

### Main-based integration

Source files and main's native schema take precedence over older screenshots and
pixel-branch evidence. Main's `schemas/Camera.schema.json` declares two projections:
perspective, and orthographic with `vertical_size`. Commit e69f2f8 changed the following.

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

### Revision 4 coverage matrix (superseded where the plain pass changed it)

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

#### Hierarchy context menu

- The menu opens from a right-click, Shift+F10 or the Menu key.
- It offers Rename (F2), Copy ID and Delete (Del, through the command bus).
- An unavailable item stays focusable and explains why when chosen.

#### Compound Collider

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

#### Dock views

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

#### Shortcuts reference

The plain list became a searchable reference:
- **Header.** A tinted keyboard mark and a subtitle that names the platform.
- **Filter.** Focused on open. It matches actions, groups and key names, and shows
  "N of M". Escape clears it first, then closes the dialog.
- **Layout.** Five category cards in two balanced columns, one column when narrow.
- **Keys.** Platform keys as individual caps (⌘ ⇧ ⌥ on macOS, names elsewhere), with
  "or" between alternatives and "/" between paired directions.
- **Coverage.** All 29 entries are implemented keys. I verified each against the
  handlers.

#### Deliberately retained

- **Native titlebar inset and traffic-light alignment.** These are `TITLEBAR.md` host
  contracts. Only a native capture can verify them.
- **Asset list structure.** The windowed, grouped listbox from revision 3 already meets
  the bounded-rendering rules. It now uses the shared counts, search field and colours.
- **Navigation grid map and value table.** Revision 3 layout, inside the new rhythm.
- **Import form.** Structure unchanged; it inherits the new type, spacing and dialog
  vocabulary. With the read-only fixture it only shows its unavailable state.
- **Application menu (File/Edit/View on Windows/Linux).** Still none; this remains an
  open question in DESIGN.md, not something to fake.

### Correctness follow-up

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

### Native End-then-Tab investigation

**Observation (`compound-end-wide.jpg`, AX text).** After End, the detail correctly
showed Part 64, but the list was blank above the pinned last row. Accessibility listed
Parts 1–13 and Part 64.

**Cause.** `useViewport` re-measured the scroller only on an animation frame after a
scroll event. A programmatic reveal (End) scrolled the list, but the window state kept
the old top until a frame arrived. WebKit throttles or pauses frames for hidden,
occluded or non-frontmost views, which is likely during computer-use automation. The
UI therefore kept rows 1–13 mounted (the old window) plus the pinned selection, at the
bottom of a scrolled list.

**Fix (in the whole-app commits, with Astra's e8ee0d6 changes kept):**
- `useViewport` returns a synchronous `remeasure`. The tree, the asset list and record
  lists call it right after `revealSpan`, so the target's neighbours mount in the same
  render as the selection.
- Scroll and resize notifications are coalesced on a frame with a 100 ms timer
  fallback.
- Keyboard focus moves after React commits (a layout effect), not on a frame.
- `ThrottledFrames.test.tsx` reproduces the failure with frames disabled: it fails on
  the old code and passes with the fix.

`compound-min.jpg` shows Part 1 and is not a last-row test.

### Revision 3 → 4 measurements

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

#### 100,000 entities (browser, not the device gate)

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

### Changed paths in earlier commits

The integration commit (e69f2f8):
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

### Commands and results of earlier commits

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
- **Native:** I took no native captures and drove no native UI in that session. The
  f78d763 captures were read from the ignored directory only.
