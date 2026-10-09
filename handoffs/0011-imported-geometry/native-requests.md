# Native capture requests for handoff 0011 (Claude to Astra)

Requested by Claude Opus 5.5 (`claude-opus-5-5`), which has no native computer use in
this ACP session. Please capture through Codex CUA and save the files as ignored
`artifacts/geometry-review/native-*.jpg` in this worktree. Keep them private and do
not commit them. Crop out or avoid the account chip in the titlebar.

The five captures supplied from b2721a2 were reviewed; see result.md. They already
cover the wide attached view, Undo to the original version, the pre-copy error, and
automatic recovery. The requests below cover only what is still missing.

Build the release app from the integrated branch that contains this commit's UI
change. Use a `CARGO_TARGET_DIR` that no other checkout shares. Reuse the
"Imported geometry review" project.

## Requests

1. **`native-render-error-after-copy.jpg`. Required.** Repeat the earlier asset IO
   error: Redo with the replacement cooked file temporarily moved. Capture the whole
   window and a crop of the viewport. Expected result: an "Error" pill, the heading
   "Viewport render error", the verbatim backend detail beneath it, and no line saying
   "failed to attach" or "Nothing in this area is rendered by the engine". Then restore
   the file and confirm the pill returns to "Attached" with the scene visible. Please
   record the exact detail string in your notes.
2. **`native-minimum-1000x650.jpg`. Requested.** The supplied minimum capture is
   1720×669 pixels, so it shows a short window but not the 1000-point minimum width.
   Please size the window to exactly 1000×650 logical points with the default panel
   layout, then capture the whole window in the attached state.
3. **`native-viewport-retina-crop.jpg`. Optional.** The supplied captures are 1440×900,
   which looks like 1× scale. If the display allows, crop the viewport island at native
   Retina resolution, including its edges and the neighbouring splitters. This is to
   judge triangle edge aliasing and any seam at the island border.

For each capture, please note the window size, the display scale factor, and whether
any panel was resized from the default.
