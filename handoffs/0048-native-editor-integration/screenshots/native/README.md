# 0048 native captures

Actual native window captures of the granted immutable editor
(`incant_editor`, SHA-256 `3d059445…6603`, source `bb1b517`) in its WKWebView, on macOS 26.6,
built-in display at 2×. Window-only (`screencapture -o -x -l` on the owned window), so 2880 px
wide at 1440 pt and 2000 px at 1000 pt. No browser double is in this folder. Hashes:
`../../evidence/native-screenshots.sha256`; measurements: `../../evidence/native-review.json`.

The UI fixes in this handoff are not in these images: they need a newly granted binary.

| File | Shows |
| --- | --- |
| `unsaved-01-start-explains-saved-project.png` | No project path: Play unavailable, "Open a saved project to run Play" in the status line, no notice |
| `play-01-playing-imported-geometry-stationary-camera.png` | Playing · Main: both imported cubes from the Stationary Camera |
| `play-02-stepped-runtime-tick-264-x-3.1.png` | Paused · tick 264 after Step; runtime X 3.1 = 2 + 0.25 × 264 / 60 |
| `play-03-stopped-authoring-x-2-restored.png` | Stopped: authoring viewport, authored X 2, "Runtime changes were discarded" |
| `play-04-key-probe-focused-view-65-ticks-scale-2.png` | Key Probe after 1 s of D in the focused game view: X 65, scale X 2 |
| `play-05-resume-composer-focus-scale-1-counter-65.png` | Resumed with the composer focused and D held: X still 65, scale X 1; the D went to the composer (macOS autocorrect bubble visible) |
| `play-06-no-camera-notice-after-cmd-p-from-picker.png` | Defect D1 state: ⌘P from the picker; notice shown, no element focused |
| `play-07-no-camera-notice-dismiss-keyboard-focus.png` | Dismiss reached by Tab, with its ring |
| `splitter-01-inspector-drag-to-280-no-selection.png` | Inspector separator dragged across text to its 280 px minimum: nothing selected |
| `splitter-02-dock-drag-to-120-no-selection.png` | Dock separator at its 120 px minimum: nothing selected |
| `splitter-03-separator-press-closed-menu.png` | Hierarchy menu closed by a press on a separator |
| `splitter-04-ordinary-text-drag-selects.png` | A drag that starts on Inspector text still selects |
| `layout-01-1440x875-inspector-380-playing.png` | 1440 × 875 (largest available), 380 px column, Playing, runtime read at tick 82 |
| `layout-02-1440x875-inspector-280-paused.png` | 1440 × 875, 280 px column, paused at tick 278 |
| `layout-03-1000x650-inspector-280-paused.png` | 1000 × 650, 280 px column, paused |
| `layout-04-1000x650-inspector-380-notice-wraps.png` | 1000 × 650, 380 px column: notice wraps to 3 lines, picker label truncated |
| `allowance-01-invalid-fraction-refused.png` | "12.5": "Enter a whole number of tokens." |
| `allowance-02-zero-off-no-send.png` | Allowance 0: header Off, no Send, "Token allowance set to 0." |
| `allowance-03-below-usage-22044-of-10000.png` | Reopened process, cap lowered below usage: "22,044 of 10,000 tokens used", Allowance used up |
| `turn-01-working-stop-available.png` | The one real turn working: header spinner, Stop available |
| `turn-02-patch-approval-exact-arguments.png` | The patch approval with its exact arguments opened by keyboard (the sticky row covers most of them in the 288 px pane) |
| `turn-03-finished-1-edit-22044-tokens.png` | Finished: reply, two allowed decisions, "1 edit · 22,044 tokens" |
| `turn-04-history-agent-entry-undone.png` | After Undo: name restored, History 8 of 9 applied (the agent entry is below the fold) |
| `reopen-01-account-and-usage-retained.png` | New process, same fixture: Ready, "22,044 of 64,000 tokens used" (window in the background) |
| `defect-model-name-clipped-crop.png` | Defect D2, 2× crop of the composer at 380 px: "gpt-6-astr…" |

The titlebar avatar shows only the account's initial, as in 0044's published captures. No
account dialog, tooltip, e-mail address or other account identity appears in any image.
