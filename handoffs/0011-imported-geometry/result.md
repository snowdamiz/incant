# Handoff 0011 result: imported geometry visual review and viewport error wording

## Status

- **Error wording.** Done and verified in the browser. The general viewport error state
  no longer claims the surface failed to attach or that nothing is rendered.
- **Headless geometry review.** Done.
- **Native review of the supplied captures.** Done for all five captures from b2721a2.
- **Native review of the new error copy.** Done on 2026-10-09 from Astra's final release
  captures, at 1440×900 and at the true 1000×650 minimum. Verdict: pass. No UI fix was
  needed, and no further native evidence is requested.
- No phase gate is approved or claimed by this result.

## Model and transport

- Model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Transport: Claude Code (Claude Agent SDK) session through ACP on the director's Claude
  subscription. No native computer use was available. No screencapture, CGWindow capture,
  AppleScript or synthetic native events were used.
- Director feedback acknowledged: the priority update supplying native captures was read
  before finishing. All five captures were reviewed, and the final post-integration error
  capture was requested as instructed.
- Final packet section acknowledged. Astra integrated the wording and made render errors
  appear in Problems. In this worktree those commits are 11bc56e and c135b96. The brief
  cites ccf30f3 and 318f35c from the integration branch. The bridge change in 318f35c
  matches c135b96. This final pass changed documentation only, so the full UI tests were
  not rerun, as the packet allows.

## Changed paths

- `editor/ui/src/components/ViewportPanel.tsx`: the copy change below.
- `editor/ui/src/ProjectStates.test.tsx`: two behavior tests.
- `handoffs/0011-imported-geometry/native-requests.md`: remaining native requests.
- `handoffs/0011-imported-geometry/result.md`: this file.

No Rust, bridge contract, dependency, font, icon, camera, material, fixture or CSS change.
The brief's priority update was edited by Astra in this worktree. It is left uncommitted
because it is not Claude's work.

## The wording change

The native app reports three different failures under the single code `viewport.failed`.
They are a surface or renderer setup failure, a failed scene replacement that keeps the
last valid GPU scene, and a draw failure that stops the render loop. The UI cannot tell
them apart, so the copy has to be true for all three.

| State | Before | After |
| --- | --- | --- |
| General viewport error, heading | The viewport failed to attach | Viewport render error |
| General viewport error, detail | Backend message | Backend message, unchanged and verbatim |
| General viewport error, footnote | Nothing in this area is rendered by the engine. | Removed |
| Project failed to open, heading and detail | No project loaded. There is nothing to render until the project opens. | Unchanged |
| Project failed to open, footnote | Nothing in this area is rendered by the engine. | Removed |
| Not attached, no engine, loading | Unchanged, footnote kept | Unchanged, footnote kept |

Rationale:

- "Viewport render error" names what happened without claiming a cause. It does not say
  the surface is detached, and it does not promise recovery, because a draw failure stops
  the loop for good.
- The footnote is removed in both error states. In native, the render thread can still
  hold an attached surface under the overlay, so "nothing is rendered" is not reliably true.
- No recovery hint was added. A truthful hint needs a distinguishable error code; see the
  findings for Astra.
- The status pill stays "Error". The layout, spacing, tile and type are unchanged.

## Commands and results

```
npm test --workspace editor/ui
```

The first run with the default 5-second test timeout had 18 failing tests across 4 files.
All of them were timeouts. The machine's load average was between 180 and 330 at the time,
from other processes. Running the new test alone also timed out in its axe accessibility
pass, not on an assertion.

```
npm test --workspace editor/ui -- --testTimeout=120000
```

Result: 10 files and 278 tests passed. The longer timeout was a command-line flag only.
The test configuration is unchanged.

```
npm run build --workspace editor/ui
```

Result: the build succeeded.

| Measure | Value |
| --- | --- |
| Main JS, Vite gzip report | 101.59 kB |
| Main JS, gzip -9 | 100,228 bytes, 97.88 KiB |
| Budget | 110 KiB |

```
node artifacts/geometry-review/error-review/capture.mjs
```

This is a read-only browser fixture that is not committed, because `artifacts/` is ignored.
It serves the production build with `vite preview` on port 4187 and uses local Chrome
155.0.8059.40 through the pinned playwright-core. It injects the UI's own sample snapshot
with a viewport error. The bridge is labeled "Read-only review fixture: viewport error copy
(not engine state)", shows as a fixture in the status bar, and rejects every command. It
captured the render-error and project-error states at 1000×650 and 1920×1080, at 2× scale.

New tests:

- A render error on a ready project shows "Viewport render error" and the verbatim detail.
  The detail stays the host's accessible description. None of the old phrases appear, no
  project banner appears, and axe reports no violations. A later attached snapshot clears
  the overlay.
- A failed project open keeps "No project loaded" and its detail, and never says "render
  error".

## Screenshots

All of these are local and ignored. Native captures are private and were not committed.

- Browser fixture, in `artifacts/geometry-review/error-review/shots/`:
  - `render-error-minimum-full.png` and `render-error-minimum-viewport.png`
  - `render-error-wide-full.png` and `render-error-wide-viewport.png`
  - `project-error-minimum-*.png` and `project-error-wide-*.png`
  - `report.json` holds the measured text, boxes, colors and overflow checks.
- Headless renderer frames in `artifacts/geometry-review/`: `original.png`, `reimported.png`
  and `without-sources.png`.
- Native captures from Astra in `artifacts/geometry-review/`: `native-reimported-wide.jpg`,
  `native-undo-original.jpg`, `native-cache-error-before-copy.jpg`, `native-recovered.jpg`
  and `native-minimum.jpg`. The last one is 1720×669, not the true minimum.
- Final native captures in the same folder: `native-final-render-error-wide.jpg`,
  `native-final-render-error-minimum.jpg`, `native-final-recovered-minimum.jpg` and
  `native-geometry-true-minimum.jpg`.

## Error-state review in the browser

The new state is readable at both widths, with no horizontal overflow.

| Measure | Value |
| --- | --- |
| Viewport host at 1000×650 | 446×320 |
| Heading contrast on the viewport field | 16.3:1 |
| Detail contrast on the viewport field | 8.6:1 |
| Detail contrast on a grid dot | 7.6:1 |

The long engine detail wraps to three centered lines inside the existing 44ch measure, at
both sizes. Without the footnote, the block is shorter and stays centered. The pill, tile
and heading hierarchy match the approved empty states.

## Native review of the supplied captures

- **Composition is sound.** The native surface fills the viewport island exactly. The left
  library, Inspector and the Problems, Console and History dock are intact. No seam or gap
  is visible at the island edges at this capture scale.
- **Clear color matches the panels.** The renderer's clear color differs from the panel
  background by about one step per channel, so the attached viewport reads as part of the
  panel. The empty-state field is slightly darker, which is acceptable.
- **The diagnostic shading reads as intended.** It is flat, neutral and high-contrast
  against the background, at 14.2:1 in the headless frames. Nothing suggests materials,
  textures, lighting or shadows, and no controls claim them.
- **Versions are visually consistent.** Undo shows the original placement, and the
  replacement shows the larger placement to the right. Both match the headless frames.
  The frame without sources is byte-identical to the original frame.
- **Recovery works as described.** After the cache file was restored, the viewport returned
  to Attached with the replacement scene and no leftover overlay.
- **The old copy is wrong as reported.** The pre-change error capture shows "The viewport
  failed to attach" and "Nothing in this area is rendered by the engine" above a real asset
  IO detail. This handoff fixes that copy.
- **Capture artifacts, not UI defects.** A purple pill sits over the traffic-light area, and
  the Undo or Redo glyph is replaced by a cursor arrow in three captures. Both look like
  computer-use overlays.

Appearance findings, not changed here because they need Rust or are outside this scope:

- **Edges are aliased.** The headless frames contain exactly two colors, so there is no
  multisampling or edge smoothing. Edges will stair-step on 1× displays. This is a
  look-dev decision for a later handoff.
- **Framing is high and small.** The geometry sits in the upper third and covers about
  2% of the headless frame. This belongs to the future camera work and was not changed.
- **Both triangles share one tone.** Faces near the fixed light direction look identical,
  so the shading gives little shape cue on this fixture. That is expected for a
  diagnostic shader, and the synthetic geometry is flat.

## Final native review of the integrated wording

All four captures come from the real release app with a real missing cooked file. They
were reviewed at full size. No account label is quoted here.

- **Wide error state passes.** `native-final-render-error-wide.jpg` shows the "Error"
  pill, the heading "Viewport render error" and the verbatim backend detail. Neither old
  phrase appears. Problems has exactly one matching row, and the status bar counts one
  error. The heading, detail and Problems row agree.
- **Minimum error state passes.** `native-final-render-error-minimum.jpg` is 1000×650 at
  scale 1. The viewport island is about 420×300, and the tile, heading and three-line
  detail fit with comfortable margins. The Problems row truncates with an ellipsis, which
  is acceptable because the full text is in the viewport. The titlebar account chip
  truncates without overlapping anything.
- **Recovery passes.** `native-final-recovered-minimum.jpg` shows "Attached", the scene
  again, an empty Problems list and a zero error count. This followed restoring the file,
  with no reopen or edit. The Console keeps its two entries, which is correct for a log.
- **Geometry at the true minimum passes.** `native-geometry-true-minimum.jpg` shows both
  triangles fully inside the island, with no clipping and no seam at the edges. The
  dimmed titlebar is the normal unfocused-window state.
- **Wording reads well in native.** The long asset ID cannot break, so the first detail
  line is short. That is readable, and changing it would mean altering the backend text.

Minor presentation notes, not fixed here because they are outside the files this packet
allows:

- **The Problems row is labeled "Project".** Rows without an entity use that scope label.
  A render error is better described as "Viewport". That would mean a diagnostic source
  field in the contract, or a change to the dock row.
- **The earlier 1720×669 capture was misnamed.** Astra has already corrected this. The
  new captures are the true minimum.

## Findings for Astra

1. **One code covers three failures.** Setup, scene replacement and draw failures all
   report `viewport.failed`. A distinct code for a failed scene replacement would let the
   UI truthfully say that the last valid scene is still shown, and that a valid revision
   will recover. That would be a contract change, so it was not made here.
2. **The overlay hides the retained scene.** The brief notes this. Once failures are
   distinguishable, a non-blocking banner over the attached surface would show both the
   error and the scene. That is a layout change for a future visual handoff.
3. **The draw-failure path never recovers.** After a draw error the render loop exits, so
   the error stays until restart. The UI copy now avoids promising recovery for that case.
4. **Render errors in Problems: resolved.** The pre-copy capture showed "No problems" next
   to a viewport error. Astra's c135b96 publishes the error as a diagnostic, and the final
   captures confirm it appears and clears on recovery.
5. **Test timeouts under load.** The UI suite exceeds the default 5-second timeout when the
   machine is heavily loaded. The CI gate is unaffected by this run, but it is worth
   watching.

## Not done or unavailable

- No native build was run, per the brief.
- No Retina crop exists, because the capture display runs at scale 1. Edge aliasing was
  judged only from the 1× captures and the headless frames.
- No visual-regression baseline exists or was claimed.
- Mathematical correctness of the shader and transforms was not reviewed; Astra owns it.

## Open questions

- Should the scene-replacement error get its own code, so the viewport can keep showing
  the retained scene with a banner? This needs Astra for the contract and a later visual
  handoff for the layout.
