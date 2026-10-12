# Native Play controls and runtime inspection

Use Claude Opus 5.5 through ACP, explicitly Max. Astra owns the real native worker,
transport and numerical acceptance. You own this editor UI, interaction design,
styles and rendered-pixel review. Read AGENTS.md and the current bridge contract.
Implement in this handoff worktree; no PR, publishing or main/candidate writes.

The real backend now cooks once into a disposable native world, with retained GPU
presentation and device audio. It supports Start, Pause, Resume, one-tick Step,
Stop, selected-entity read-only inspection and bounded desktop input. Stop discards
runtime changes. Authoring edits continue through incant_cmd and affect the next
session. Do not invent missing engine features or move simulation into JavaScript.

## Scope and design

Integrate these real capabilities into the editor as a coherent professional tool.
Choose control placement and appearance yourself. Keep the user's repeated feedback
central: clean restrained colors, compact useful headers, generous but efficient
spacing, no decorative stripes/glows, no oversized badges, no explanatory filler
rows. Account and shortcut dialogs should retain their current improvements. This
is a working editing environment, not a dashboard mockup. Native viewport bounds,
macOS traffic-light insets, splitters, keyboard navigation and existing panels must
remain correct at 1000x650, 1440x900 and narrow inspector widths.

Make the running/paused/preparing/error/stop transitions understandable. Keyboard
and accessible names should work. Handle stale operations, loading, terminal runtime
errors, unsaved-project refusal, multiple scenes and camera choice if needed. Show
only useful status; raw generation/sequence IDs and discarded-tick counters need
not be prominent. Stop must remain usable while startup is preparing. A failed
old startup must not visually replace a newer session.

Display live native inspection deliberately as read-only runtime state, retaining
access to authoring. Inspect the selected runtime entity on selection and after a
successful pause/step; avoid polling whole worlds or per-frame inspection. Show
Transform and optional Velocity from the native result accurately, with the tick
it represents. Missing runtime entities/scenes are explicit, not fake authoring
fallback values. Never write a native inspection back into the project.

Wire desktop game input only while the viewport is deliberately focused and the
session is running and not suspended. Blur/focus loss must clear held input. Keep
editor shortcuts and text fields usable. Bound/coalesce pointer events and input
batches (max 128 per request, max 1024 backend pending); do not launch a promise per
mousemove indefinitely. Expose input errors without log floods. Supported events:
focus, generated KeyCode key presses/releases, left/right/middle buttons,
viewport-local CSS-pixel pointer_position, and wheel with pixels/lines units.
No gamepad/touch/raw motion or pointer-lock claim. Use the existing generated
InputEvent types/KeyCode list; unsupported browser keys are ignored explicitly.

## Interfaces and paths

- editor/bridge/contract.ts: PlayStatus, PlayRequest, PlayEntity, PlayInput.
- snapshot.play is the small native status; snapshot.playError is a distinct
  transport failure. Missing status is loading/unavailable, not pretend running.
- ShellContext request returns BridgeResult; success may carry playEntity for
  play.inspect. play.start optionally takes scene/camera ULIDs. The native bridge
  Spatial scenes require an explicit camera ULID (auto-select the unique authored
  camera where unambiguous, otherwise let the user choose). The native bridge
  adds current authoring revision and session generation for controls; no raw IPC
  required. play.input MUST carry generation captured when the UI event batch was
  created. Clear queued input on generation change; never relabel it for a new game.
- Capabilities: play.control, play.inspect, play.input. Fixtures/older hosts must
  refuse or label unsupported features. NativeBridge polls metadata with one read
  in flight at up to 4 Hz; never add engine_read or document polling for Play.
- Rust owner files editor/app/src/play/*, crates/incant_play/* and main.rs are
  Astra-owned. Do not edit them. Report transport gaps instead of fabricating work.
- UI components/interaction helpers/tests and UI bridge fixture support are your
  scope. Do not change engine/compiler/runtime/CI/dependency manifests.

## Validation and evidence

Run npm run typecheck --workspace editor/ui, npm run test --workspace editor/ui,
and npm run build --workspace editor/ui. Test actual request wiring, keyboard and
focus transitions, pending stop, failed startup, stale inspection, max-size input
and disabled controls. Mock bridge tests are transport/visual evidence only.

Use a clearly labelled deterministic native-bridge double for initial browser
pixel review of all important states and responsive sizes; preserve screenshot
paths and explain fixture use. Astra will supply a native binary/fixture and an
explicit graphics-slot grant for final actual-window review; until then do not
build/launch a native renderer or claim native visual acceptance. You may perform
ordinary browser UI review. There are no phone/performance/phase-gate claims.

Return result.md with exact ACP model+Max verification, changed paths, evidence,
what was tested, screenshots and remaining limitations. Commit Built-by: claude.
Do not keep expanding the UI after the scoped work and review are complete.
