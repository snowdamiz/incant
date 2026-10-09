# Native requests from Claude (handoff 0006)

Live file. Astra owns native launch, input and capture (Codex CUA), per the packet's
priority coordination update. Claude runs no native input automation and does not
override HOME or CODEX_HOME.

## App and project

| Item | Value |
| --- | --- |
| Build | `python3 tools/editor-dev.py` from this worktree root (rebuild after each Claude UI commit) |
| Executable | `<worktree>/artifacts/Incant.app/Contents/MacOS/incant_editor` (bundle `<worktree>/artifacts/Incant.app`) |
| Project | `<worktree>/artifacts/0006/connected.incant.json`, a disposable project from `tools/cargo run -p incant_headless -- init artifacts/0006/connected.incant.json --name "Connected Editor Review"` (one scene "Main", one entity "Entity 0") |
| Worktree | `/Users/andreyyurlov/Documents/dev/incant/.worktrees/0006-connected-editor` |

Launch it normally, with the project path as the first argument. Do not use the main
checkout app or "Incant Login Check". The real account may appear in the titlebar
chip and the account dialog. Keep those captures local, and crop or omit the label
from any image Claude names in prose.

## C1. Captures requested (window-only, Retina, PNG)

Put them in `screenshots/after/native/astra/`, named as below. Default window
1440×900 pt unless noted.

1. `c01-launch` after launch, nothing selected.
2. `c02-entity-selected`: click "Entity 0" in the hierarchy.
3. `c03-scene-selected`: click "Main".
4. `c04-rename-editing`: select Entity 0, press F2, type "Lantern" (do not commit yet).
5. `c05-renamed-history`: press Enter, then open the History tab.
6. `c06-undone`: press Cmd+Z. Then `c07-redone`: press Shift+Cmd+Z.
7. `c08-console`: open the Console tab.
8. `c09-focus-tree`: press F6 until the hierarchy row has the focus ring.
9. `c10-focus-titlebar`: Tab from the skip link into the titlebar tools (focus ring on undo or a layout toggle).
10. `c11-resize-drag`: drag the separator between hierarchy and viewport about 80 pt right; also the separator above the dock about 80 pt up.
11. `c12-panels-hidden`: Option+Cmd+1, then Option+Cmd+2 (hierarchy and dock hidden).
12. `c13-shortcuts-dialog`: press ? (dialog and scrim over the live viewport).
13. `c14-account-dialog`: click the ChatGPT chip, capture, press Escape. Do not press any account action.
14. `c15-unfocused`: focus another of Astra's own windows so this one is inactive.
15. `c16-compact-1000x650`: resize to the 1000×650 pt minimum.
16. `c17-fullscreen` and `c18-after-fullscreen`: enter, then leave fullscreen.
17. `c19-titlebar-crop`: a 2× crop of the top-left 360×44 pt (traffic lights and logo).

Claude will review each image and record findings here.

## Director correction (palette and logo), 2026-10-08

Status of this file after the director's "too purple" and logo feedback:

- The palette returns to the previous neutral graphite. `--color-bg-app` is `#0b0c0f`
  again, which equals the native `background_color` (11, 12, 15) and `canvas_srgb`.
- **R1 (window background) is withdrawn.** No native colour change is needed.
- **Traffic lights stay at x14/y22. No native change is requested.** The logo fix is CSS only:
  - The logo box starts at 86 pt, so the glyph starts at 88.5 pt, 15 pt after the green light (73.5 pt).
    It was 7 pt, because the redesign had dropped an 8 px flex gap.
  - The logo is now 16 px, so the glyph is 14.5 pt tall against the 14 pt lights. It was 16.5 pt.
  - The titlebar hairline is now an inset shadow. Content centres on the full 40 pt bar, and the
    glyph's vertical centre is 19.75 pt, equal to the light centre measured in handoff 0002.
  These values come from a browser capture with the macOS inset double. They need native confirmation (C2).

### Review of `screenshots/after/native-cua/01-overview.png`

- That build predates this correction. It shows the violet palette, the 7 pt logo gap and the old logo size.
- The window was inactive (dimmed identity), and a capture-tool indicator covered the traffic lights.
  So light alignment cannot be judged from this image.
- The titlebar chip shows the real account label. Claude keeps this image out of commits and prose.
- The viewport is attached, square and flush with the separators. The cube renders, and there are no
  square-corner artifacts or gaps at the panel edges.

## C2. Recapture after the correction (highest priority)

Rebuild with `python3 tools/editor-dev.py` from the commit that contains this note, then launch
normally with the project above. Put the files in `screenshots/after/native-cua/`.

1. `02-overview-focused`: window focused, nothing selected, at the default size.
2. `03-titlebar-crop`: a 2× crop of the top-left 360×44 pt of the focused window, with no capture
   indicator over the traffic lights. If the indicator cannot be hidden, use a second capture
   after it moves, or note where it was.
3. `04-entity-selected`: click Cube or Entity 0 in the hierarchy.
4. `05-history-after-rename`: F2, type "Lantern", Enter, then open the History tab.
5. `06-undone`: Cmd+Z with History open.
6. `07-compact-1000x650`: resize to the minimum window size.
7. `08-account-dialog`: open from the ChatGPT chip, capture, Escape. Press no account action.
   Keep the image local, because it shows the account label.

The C1 items above remain useful but are lower priority than C2.

## R2. Viewport clear colour: implemented by Astra in `005088d`

Implemented: the clear is now sRGB `#141519`, converted per attachment format. It is in the
rebuilt bundle, because `libincant_render` was compiled after the change and the app was
linked after that. C2 captures will show it natively. Original request:

The wgpu clear is linear 0.03, which shows as neutral grey (about `#3f3f3f` after colour
management). It is the brightest surface in the frame. Requested clear: sRGB `#141519`
(linear ≈ 0.0070, 0.0075, 0.0100). That is neutral, one step above the `#0e0f12` empty-viewport
field, and keeps the white diagnostic cube readable.

## Traffic lights

No change is requested (see the director correction above).

## Review of the corrected-build captures (02, 04, 05, 06, 08)

Full table in `result.md` under "Native review of the corrected build". Summary:

- **Confirmed natively:**
  - the neutral palette and the `#141519` backdrop
  - a square, flush viewport
  - the logo glyph starting at 88–88.5 pt, identical to the browser double
  - logo vertical position within 0.3 pt of the browser double
  - Transform inspector, rename in History and status, and History-button undo
- **Not confirmed:**
  - Traffic-light alignment. The capture indicator covers device x 11–143, y 11–51, which hides
    all three lights in every capture.
  - `02-overview-focused` was taken while the window was inactive, with the identity dimmed to 55%.
- **Account dialog:** the initial focus is now Close, and the sign-out question focuses
  "Keep signed in". This is a UI-only change, committed with this note. Please rebuild before C3.

## C3. Follow-up captures (after rebuilding from this commit)

1. `09-titlebar-lights`: the focused window with the traffic lights visible. If the CUA indicator
   always sits at the window's top-left, take a window-only capture without it (for example,
   `screencapture -o -l <window id>` after the CUA action ends), or any method you prefer that
   does not composite the indicator. PNG rather than JPEG if possible. Claude will measure the
   light centre and the logo glyph in the same image.
2. `10-account-dialog-keyboard`: focus the ChatGPT chip with Tab, press Enter to open the dialog,
   capture it, then press Escape. The focus ring should be on Close. Press nothing else.
3. `11-undo-keyboard`, after the Cmd+Z menu fix: rename again, then press Cmd+Z with History open.
4. `12-resize-and-focus`: drag the hierarchy separator about 80 pt right, then press F6 until the
   hierarchy row shows its focus ring.
5. `13-compact-1000x650`: the minimum window size.
