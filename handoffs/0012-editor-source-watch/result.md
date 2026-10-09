# Handoff 0012 result: native editor source-watch review

## Status

- **Verdict: pass.** All eight supplied native captures were reviewed. The pixels show
  the original, external-edit, Undo, Redo, broken-source and recovered states the brief
  describes. The approved layout has not regressed.
- **No UI fix was made.** Nothing in `editor/ui` changed, so UI tests and the build
  were not rerun, as the packet allows for a review-only result.
- **No further native evidence is required.** `native-requests.md` was not created. The
  pending cache-scope fix changes filesystem validation only and does not touch the
  reviewed presentation. One optional capture is listed under open questions.
- One copy defect in the source diagnostic is reported to Astra below. Its text is
  composed in the native bridge, and the brief assigns bridge source diagnostics to
  Astra.
- No phase gate is approved or claimed by this result.

## Model and transport

- Model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Transport: Claude Code (Claude Agent SDK) session through ACP on the director's Claude
  subscription.
- No native computer use was available or used. No screencapture, CGWindow capture,
  AppleScript or synthetic native input was used. The app was not built or launched.
- The brief had no priority revisions or new director feedback beyond its body, and no
  files existed from an interrupted attempt. The director's earlier rejection of the
  bottom asset browser was respected: this review checks only that the merged layout
  held, and proposes no shell redesign.

## Inspected captures

All captures are from `artifacts/native-source-watch-review`, which is git-ignored.
None were committed or copied. Each is 1440×900 at scale 1, captured by Astra through
Codex CUA from a release build at ac2b162.

| Capture | Observed |
| --- | --- |
| w01-original.jpg | Two triangles, Problems empty, Console 1, History 2, Attached. |
| w02-external-edit.jpg | Top vertex of both triangles moved right. Console 2, History 3. Problems empty. |
| w03-undo.jpg | Geometry back to the original. Redo enabled. History still 3. |
| w03b-undo-stable.jpg | Pixel-identical viewport to w03-undo after further polling. |
| w04-redo.jpg | Geometry matches the external edit again. Redo now disabled. |
| w05-broken-source.jpg | Geometry unchanged from Redo. Problems shows 1 error, status bar shows 1 error, History stays 3, viewport still Attached. |
| w06-recovered-new-source.jpg | New third shape: top vertices moved left. Problems back to 0, Console 5, History 4. |
| w07-shared-history.jpg | History tab open: 4 of 4 applied, two "Reimport changed sources" entries tagged source-watch. |

The title bar shows a private account label in every capture. It is not quoted here.

## Viewport pixel comparison

The JPEGs were converted to BMP with the system `sips` tool and compared over the
viewport interior, pixels x 250 to 1105 and y 80 to 625. A pixel counts as changed when
its grey value differs by more than 64, which ignores JPEG noise. Scripts lived in
`/tmp` and are not committed.

```
sips -s format bmp <capture>.jpg --out /tmp/sw-review-bmp/<capture>.bmp
python3 -I /tmp/sw-review-scripts/diff.py /tmp/sw-review-bmp
```

| Comparison | Changed pixels | Meaning |
| --- | --- | --- |
| original vs external edit | 5367 | Reimport changed the rendered geometry. |
| original vs Undo | 0 | Undo restored the original exactly. |
| Undo vs Undo after polling | 0 | Polling did not reapply the external edit. |
| external edit vs Redo | 0 | Redo restored the changed geometry exactly. |
| Redo vs broken source | 0 | The last valid geometry was retained during the error. |
| broken source vs recovered | 8011 | The repaired source produced new geometry. |
| original vs recovered | 5042 | The recovered geometry is a third, distinct version. |
| recovered vs history view | 0 | Opening History did not disturb the viewport. |

Lit pixel counts were 8615 for the original and Undo states, 8946 for the edit, Redo
and broken states, and 8307 for the recovered states. This matches three distinct
geometry versions.

## Review findings

- **Layout separation held.** The left Assets/Hierarchy library, the main Inspector and
  the output-only dock with Problems, Console and History are unchanged in all eight
  captures. No asset browser returned to the dock and no watch controls were added.
- **The error state reads clearly.** The Problems badge turns red at 1, the row has a red
  error icon, and the status bar count agrees. The message fits on one line at 1440 wide,
  with the "Project" scope label right-aligned and legible.
- **The recovered state is clean.** The red badge disappears and the "No problems" empty
  state returns, so the user is not left with a stale error.
- **History is readable and attributes automation.** Automatic imports use the same amber
  Import tag as manual imports. They are distinguished by the source-watch actor in the
  right-hand mono column. Row spacing and timestamp alignment are consistent.
- **Undo and Redo toolbar state is correct.** Redo is enabled after Undo and disabled
  after Redo, and after the broken edit, because no redo is pending.

## Defect for Astra: doubled file name in the source diagnostic

The w05 Problems row reads, in shape:

```
<file>: could not cook <file>: invalid glTF: expected value at line 1 column 1
```

The file name appears twice. The import crate already formats the error as
`could not cook {path}: {source}` in `crates/incant_import/src/lib.rs`. The native
bridge mapping in `editor/bridge/native.ts` then prefixes the watch diagnostic's
`source`, and sets `path` to null. So the right column falls back to "Project" instead
of naming the asset. The Console line built in `editor/app/src/source_watch.rs` has the
same doubling, though no capture shows it.

Recommended bridge change, with no UI change needed: map the watch diagnostic's `source`
into the diagnostic's `path` field and drop the message prefix. The bridge test in
`editor/bridge/native.test.ts` asserts the current prefixed shape and would change too. The existing Problems
row then shows the asset path in the mono meta column, as it already does for entity
diagnostics. This is correctness and data-binding work owned by Astra. It is a copy
improvement, not a blocker for this verdict.

## Changed paths

- `handoffs/0012-editor-source-watch/result.md`: this file.

No Rust, contract, dependency, camera, lighting, shader, CSS or component change.

## Tests and builds

- UI tests and the UI build were not run. No UI file changed and the packet waives them
  for a review-only result.
- The native app was not built, per the packet.

## Screenshots

- No screenshots were produced by Claude.
- Reviewed native captures stay in the ignored `artifacts/native-source-watch-review`.

## Limitations

- Only 1440×900 at scale 1 was observed. Minimum-size and Retina presentation of the
  diagnostic and History states were not reviewed and are not claimed.
- Long diagnostic messages and wrap or truncation in a narrow dock were not observed.
- Only the Problems and History tabs were seen. The Console entries behind the count
  changes were not shown in any capture.
- It was not observed whether a new source error is noticeable while the dock shows
  History or Console. The red badge on the Problems tab and the status bar count are
  the only cues, and both are present in w05.
- The final reopened-app capture after the cache-scope fix was not reviewed.
- The monochrome triangles are diagnostic fixtures. Production PBR, antialiasing and
  camera work remain open.

## Open questions

- Should a project-level source diagnostic reveal the asset in the Assets view when
  clicked, the way entity diagnostics reveal the entity? That is new behavior and needs
  a bridge binding first, so nothing was added.
- Optional capture if Astra wants it: a source error raised while the History tab is
  active, at 1440×900, to confirm the red Problems badge is noticeable from another tab.
