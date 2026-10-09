# Native viewport integration: constraints and open questions

Scope: how the web shell (`editor/ui`) cooperates with the native wgpu surface
(Phase 0 Spike 1). Written by handoff 0001 for Astra, before any native run.

**Native results (handoff 0002, macOS only).** The real Tauri/wgpu app was reviewed on
macOS 26.6, an Apple M5 Pro, and a Retina display at a device pixel ratio of 2. The
surface sits below a transparent webview. The geometry shows through the viewport
hole, and the chrome is opaque at exact token values. The GPU corner mask matches
the island with no seam. The surface follows panel hiding, window resizing,
fullscreen and zoom, and the modal dialog covers it. No Windows or Linux evidence
exists yet. See `handoffs/0002-native-viewport/result.md`.

## What the UI does today

1. **Reserves a hole.** `[data-viewport-host]` in `ViewportPanel` is the only area the
   native surface should occupy. The UI never draws engine pixels there.
2. **Goes transparent when attached.** When `snapshot.viewport.status === 'attached'`,
   the shell adds `html.native-viewport`, which makes `html`, `body`, the app frame, the
   workspace, the viewport panel and the viewport host transparent. All other chrome
   (other panels, the viewport strip, the 1 px separators, titlebar, status line) stays
   opaque. Revision 2 (handoff 0006) has no gutters, so only the hole is transparent. The
   host window background stays the canvas colour `#0b0c0f` (see TITLEBAR.md).
   Measured in Chrome with an evidence test double (`evidence.json → transparency`):
   PNG alpha is 0 at the viewport center and 255 on the hierarchy, top bar and status bar.
3. **Reports placement.** If the bridge advertises `viewport.bounds`, the UI sends
   `{ type: 'viewport.bounds', rect, devicePixelRatio, cornerRadii }` on mount, on element resize
   (`ResizeObserver`) and on window resize, coalesced to one per animation frame.
   `rect` is in CSS pixels relative to the webview's top-left. `cornerRadii` is
   `[topLeft, topRight, bottomRight, bottomLeft]` in CSS px, read from the host
   element. Since revision 2 the host is square and flush with its neighbours, so it
   reports `[0, 0, 0, 0]`, measured in Chrome at 1000×650, 1280×800 and 1440×900. The host's
   corner clip remains supported for any future rounded layout.
4. **CSP allows the IPC scheme.** `connect-src 'self' ipc: http://ipc.localhost`.

## Constraints the host must satisfy

- **Window transparency.** The Tauri window and its webview must be created
  transparent (`transparent: true`, plus macOS private API where required), or the
  native surface must be layered *above* the webview and clipped to the reported rect.
  Which of the two is used is Spike 1's decision; the UI supports both.
- **Coordinate conversion.** Physical pixels = `rect × devicePixelRatio`. The host must
  re-place the surface when the window moves between displays with different scale
  factors (the UI re-sends bounds on `resize`, but a DPR-only change may not fire one;
  see open question 3).
- **Overlap.** Nothing in the web UI overlaps the viewport except the modal shortcuts
  dialog and its scrim. If the native surface is layered *above* the webview, the
  dialog will be hidden behind it. Options: hide the native surface while
  `[role=dialog]` is open (the UI can report this), or keep native below the webview.
- **Input.** With the surface below a transparent webview, pointer events over the
  hole reach the webview first. The host must forward them or the UI must mark the hole
  `pointer-events: none` (not done yet; needs a decision with gizmo work).
- **Focus.** The viewport region is an F6 stop. Keyboard input while it is focused
  should go to the engine; this needs a `viewport.focus` request or an equivalent.

## Open questions for Astra

1. Is the native surface a child window *below* the webview (transparent webview) or
   *above* it (clipped)? This decides dialog overlap and input routing.
2. Should `viewport.bounds` also carry a visibility flag (hidden when a modal is open,
   when the window is minimized, or when the panel is collapsed)?
3. Will the host listen for DPR changes itself, or should the UI add a
   `matchMedia('(resolution: …)')` listener and re-send?
4. Inspector data is in the snapshot for every entity (spike-sized). For large scenes
   the bridge should page `EntityDetail` by selection; the contract can grow a
   `entity.detail` request.
5. Field editing needs a component command on `incant_cmd` (e.g.
   `component.set { entity, component, path, value }`). The inspector is read-only until
   that exists. `editor/ui/src/bridge/contract.ts` is a proposal to be replaced by the
   real `editor/bridge` types.
6. Conversation events for the agent transcript are not in the contract yet; the agent
   panel shows only availability state and never a fabricated transcript.

## Evidence still pending

- Native screenshot via `tools/cargo run -p incant_headless -- screenshot ...` (command
  not available in this worktree).
- macOS and Windows runs of the Tauri window with the surface attached.
- Resize and DPR-change behavior with a real surface.
