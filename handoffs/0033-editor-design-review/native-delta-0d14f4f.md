# 0033 native delta review: binary from 0d14f4f

Packet: "final native read-only delta, 2026-10-10 19:08" (top of `brief.md`). Linked from
`result.md`.

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`), Max. The session reports
  reasoning effort "max" to the model. The runner selects and verifies both through ACP,
  which I cannot inspect from inside the session.
- **Scope:** I reviewed one native capture, read-only. I did not drive, close or capture
  the native app, and I did not ask the director for screenshots. I made one targeted
  browser capture to verify the one fix below.

## Source and binary

| Item | Value |
| --- | --- |
| Binary source | `0d14f4fa7b629aee43607f63131e2cf60f846618` (ignored `artifacts/0033-native-clean/manifest.json`) |
| Binary | custom-protocol release bundle; sha256 `8d914d67ec86d38a5dc5cd8228ae325e08e3ee12042a65d8c3df78f4eb9dfaf8`, 36,366,576 bytes; 504 UI tests at build |
| Capture | ignored `artifacts/0033-native-clean/current.png`: 2880 × 1748 JPEG, i.e. 1440 × 874 logical at 2×. The window is inactive (grey traffic lights), and the director chose the selection, collapsed sections and sidebar size |
| This branch before this commit | `bab8883`, Astra's same-titled commit applied here |
| UI identity | `editor/ui` tree `a55d0427…` and `editor/bridge` tree `4dca2112…` are identical at `0d14f4f` and `bab8883`. The capture therefore shows exactly this branch's UI as it stood before this commit. The rest of the repository differs, because `0d14f4f` sits on the runtime and Tauri integration line |

## Observed state

These observations cover only what the capture shows:
- Dumbbell is selected. Its Inspector sections Collider, Rigid Body and Transform are all
  collapsed, and the Inspector is about 329 px wide.
- The hierarchy has 7 entities.
- The viewport is Live, with the native render.
- Problems shows "No problems"; Console and History each have 1.
- The agent is collapsed and Unavailable.
- The status line reads "Native engine" and "No problems".
- An account is signed in, shown by its initial only.

| Surface | Plain design (revision 5) | Native | Verdict |
| --- | --- | --- | --- |
| Titlebar | Breadcrumb with the scene only when it differs from the project; plain groups with pressed toggles filled; the avatar alone in its ring; tools dimmed while the window is inactive | "Incant Studio Review / Main", three filled toggles, avatar initial, dimmed | Matches |
| Left column | Plain counts, filter, tree; the selection pill with no accent edge | "Hierarchy 7", "Assets 0"; Dumbbell in the pill | Matches |
| Viewport chrome | Header word and dot only | "Live", green dot, native surface | Matches |
| Inspector header | Kind glyph, name, lock, Copy ID; no metadata row | As designed (`screenshots/native-delta/native-0d14f4f-inspector-top.png`) | Matches |
| Collapsed sections | Chevron, quiet icon, name; hairlines between sections and none above the first | As designed | Matches. The order Collider, Rigid Body, Transform is the engine's key order: the bridge passes `Object.entries` through unchanged. It is not a UI change |
| Dock | Text tabs, plain counts, one quiet title | "Console 1", "History 1", "No problems" | Matches |
| Agent | Collapsed line with a hollow dot | "Unavailable" | Matches |
| Status line | Tone dot, source, counts | The connected dot carries a tinted halo | **Defect** (below) |

## Defect found and fixed

- **What.** `editor/ui/src/styles/shell.css`, `.statusbar__conn--ok .statusbar__dot`:
  `box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-success) 18%, transparent)`. It
  drew a soft green ring around the 7 px dot in the connected tone only; the sample,
  pending and error dots had none. It belongs to the class of glow the director rejected.
- **Why it was missed.** The browser evidence shows the same halo, because the evidence
  test double uses the connected tone. The plain audit missed it, and the native capture
  is where it stood out.
- **Fix in this commit.** The box-shadow is removed. This is style only: the dot keeps
  its size and the line its layout. No other file changed in the UI.
- **Evidence** (`screenshots/native-delta/`; account-free crops, unedited apart from
  cropping):
  - `native-0d14f4f-statusbar.png`: native, before the fix. It is converted to PNG from
    the JPEG capture.
  - `browser-statusbar-before.png`: browser, `plain-final` set, before the fix.
  - `browser-statusbar-after.png`: browser, `delta-dot` set, after the fix.
- **Verification:**
  - 504 UI tests in 29 files pass; the strict typecheck and the production build pass.
  - The browser capture `delta-dot` (Player Character, 1440 × 874) has 0 axe violations,
    0 page errors, 0 remote requests and 0 window overflow.
  - The fix still needs a native rebuild to confirm.

## Astra's accessibility fixes: preserved

`0d14f4f` (here `bab8883`) made three changes, and this commit does not touch any of them:
- **Shortcut table.** Sections use `scope="rowgroup"`, and each keys cell is labelled by
  its group, action and spoken keys.
- **Account dialog.** The signed-in status keeps the same live node through OAuth
  completion. Separate React keys stop Cancel sign-in from being reused as Sign out, so
  focus returns to Close.
- **Pager.** A repeated page range is cleared and announced again; newer navigation and
  unmounting cancel the pending repeat.

## Not observed on this binary

User input took priority, so none of these has been seen on the 0d14f4f binary or with
this commit's fix:
1. The 1000 × 650 minimum window: Inspector with 64 Parts, account dialog, shortcuts
   dialog.
2. Compound End then Tab in 64 Parts: the trailing rows are visible with no blank
   region, and AX lists about Parts 52–64. Repeat with the window not frontmost.
3. Large Navigation Grid, Show values: paging or scrolling the 65,536 values stays
   bounded and focusable.
4. Account dialog:
   - signed in with two or more accounts;
   - the sign-out question, with focus on "Keep signed in";
   - the browser step, Cancel sign-in and focus moving to Close;
   - an error state.
5. Keyboard shortcuts: WebKit glyphs (⇧⌘Z, ⌥⌘1, ⌘⌫), filtering, and VoiceOver reading
   group, action and keys per cell.
6. Hierarchy context menu: ⌘⌫ for Delete.
7. Expanded Inspector sections, including the compound parts list; dock paging with no
   "Showing …" echo in the status line; the project-opening state.
8. The status dot after this commit: plain, with no halo.
9. VoiceOver:
   - the avatar's name ("ChatGPT account, signed in as …");
   - the Inspector header's description and "Read-only";
   - pager announcements.

## Changed paths (this commit)

- `editor/ui/src/styles/shell.css`: the status dot's halo is removed.
- `handoffs/0033-editor-design-review/native-delta-0d14f4f.md`: this appendix.
- `handoffs/0033-editor-design-review/screenshots/native-delta/`: four account-free crops.
- `handoffs/0033-editor-design-review/result.md`: the link, status and packet row.
- `handoffs/0033-editor-design-review/brief.md`: the 19:08 packet section, recorded
  verbatim as received.

Not committed: the raw native capture and its full-resolution review crops (ignored
`artifacts/0033-native-clean/`), and the browser set `delta-dot`. No Rust, Cargo,
engine, schema, bridge, storage or account change.
