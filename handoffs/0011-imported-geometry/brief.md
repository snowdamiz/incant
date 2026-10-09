## Final native review — integrated wording and Problems, 2026-10-09

Your 11bc56e is integrated as ccf30f3. Astra added 318f35c so render errors also
appear in Problems and clear after recovery; the existing diagnostic presentation
is reused. The bridge regression passes. Review the new actual native captures:

- native-final-render-error-wide.jpg: 1440x900, the corrected heading/detail and
  one matching Problems row/status error count.
- native-final-render-error-minimum.jpg: measured 1000x650 pixels, display scale 1.
- native-final-recovered-minimum.jpg: same 1000x650, Attached, Problems empty and
  zero error count after restoring the missing cooked file, with no reopen/edit.
- native-geometry-true-minimum.jpg: measured 1000x650 before this final copy build.

These are ignored files in artifacts/geometry-review. The wide geometry from your
first review remains valid. The earlier "minimum" name was inaccurate; the new
1000x650 captures were obtained by dragging the actual window corner through
Codex CUA and checking image dimensions. No optional Retina crop is claimed on
this scale-1 display. Exact backend detail is the existing synthetic model ID plus
"asset IO: No such file or directory (os error 2)". This is a real missing-cache
replacement followed by automatic recovery, not a mock. No account actions.

Please finish native pixel review and update result.md/native-requests.md. Commit
only your review/fixes with Built-by: claude. Do not rerun full tests for review-only
doc changes. If UI fixes are necessary, verify them and request precise new native
evidence. Do not commit private captures or Astra's brief edits.

## Priority update: native captures available now

Astra rebuilt the real release app from b2721a2 and supplied native-*.jpg in
artifacts/geometry-review. Native Undo restored the original geometry version;
Redo with the replacement cooked file temporarily moved showed a real asset IO
error; restoring that disposable cache recovered Attached automatically at the
same document revision. Captures: native-reimported-wide, native-undo-original,
native-cache-error-before-copy, native-recovered, native-minimum. Review all.
The error screenshot predates your copy changes; request one final native error
capture after integration. Saved account restoration also succeeded; never quote
its label. Minimum sizing used Window > Move & Resize > Bottom Right and was
restored afterward. Do not commit these private captures.

# Imported GPU geometry: native visual review and error wording

Claude Opus 5.5 through ACP owns this visual review. Preserve the approved left
asset library/main Inspector and Problems/Console/History dock. Do not redesign
the layout or landing page in this packet. The director wants polished, neutral,
well-spaced editor UI; the recent bottom-dock redesign is already approved.

## Implementation and scope

Astra added actual indexed imported-model rendering, default glTF scene/node
transforms, shared instanced geometry, and retained GPU versions. Editor render
preparation follows command-bus revisions outside its lock. Failed replacements
keep the last valid GPU scene and surface an error; the current UI covers the
viewport on error. A later valid revision/retry recovers automatically.

The existing monochrome Phase 0 diagnostic shading is transferred into
crates/incant_render/src/model_geometry.wgsl. Review its appearance and native
composition; Astra owns its mathematical correctness. No new look-dev/PBR is
claimed. Embedded materials, textures, lighting, camera controls and shadows are
still unimplemented in this pass. Do not add controls that claim those exist.

Review scope:
- Actual headless PNGs in ignored artifacts/geometry-review/*.png, copied from
  the real renderer (synthetic geometry, safe to inspect).
- Native captures in ignored artifacts/geometry-review/native-*.jpg, supplied
  by Astra through Codex CUA once the new app is built. Keep those private,
  never quote account labels or commit native screenshots.
- Correct ViewportPanel's error wording. It currently says “failed to attach”
  and “Nothing in this area is rendered by the engine” for every render/asset
  failure. The surface can be attached with a retained scene underneath. Choose
  concise truthful wording for the existing general error state, preserving
  precise backend error detail. Keep project-load-specific wording accurate.
- Review changed error-state pixels with an explicitly labeled read-only browser
  fixture if needed. Record native requests if more evidence is necessary.

You may edit editor/ui/src/components/ViewportPanel.tsx, relevant behavior tests,
small directly necessary styling and handoff/result.md. Do not change Rust,
bridge contracts, dependencies, camera, materials or geometry fixtures. Record
any correctness findings for Astra. No visual routing exceptions.

## Verification and constraints

Run npm test --workspace editor/ui and npm run build --workspace editor/ui if
changing UI. Main JS must remain under 110 KiB gzip (currently 101.58 KiB). No new
dependencies, fonts or icon packs. Review wide/minimum widths if layout changes;
copy-only changes need readable error-state evidence. Do not run a native build
or share CARGO_TARGET_DIR with another checkout. Browser review may use the
existing pinned Playwright installation. Use a port separate from 4176.

Computer use/capture is authorized, but your ACP session has no native CUA.
Request precise native actions/captures from Astra in native-requests.md. Never
use screencapture, CGWindow capture, AppleScript or synthetic native events.
Do not access credentials, sign out, change accounts, publish or merge.

Commit completed work with Built-by: claude and return result.md with the exact
model/ACP transport, inspected captures, checks, verdict and remaining limits.
If native captures have not arrived, complete independent copy/browser review
and leave native review explicitly pending. Astra handles integration and merge.
