# 0044 native editor Play: result

## Status

- **Browser stage: done.** Commit `1b01cdbf38f6ab9a6f8d6cc1568d9f3efc97f966`, integrated by the
  root at `cfc20c1`. The editor has:
  - native Play controls in the viewport header;
  - read-only runtime inspection in the Inspector;
  - bounded desktop game input from a focused game view;
  - the ⌘P, ⇧⌘P and ⌥⌘P shortcuts.
- **Native review: done** on the granted immutable binary (SHA-256 `718ea75b…bd1b6`). It
  covers wide, minimum and narrow layouts. Controls, status, Preparing, Pause, Step, Stop,
  Suspended, runtime rows, the picker, error placement, keyboard focus, F6 and the Play
  shortcuts all behave as designed in the real WKWebView window. Measured details:
  - Runtime rows match the engine exactly.
  - Game input reaches the game with the expected numbers.
  - The pointer arrives in viewport-local CSS px.
  - Traffic lights, logo and title share one centre line within 0.25 pt.
- **Three defects found natively:**
  1. **Game focus after Resume (fixed).** Resuming with another panel focused left the game
     believing it had focus. Fixed in `src/play/input.ts`; there is also a worker note for
     Astra.
  2. **Keyboard Dismiss (fixed).** Dismissing the error notice from the keyboard left no
     element focused. Fixed in `PlayControls.tsx`.
  3. **Splitter drag (reported only).** Dragging a panel splitter selects text across the
     Inspector. The Splitter belongs to the handoff 0001 shell, outside this packet's
     paths, so it is reported with a suggested patch and not changed.
- **The fixes are not natively verified.** Both have regression tests, and typecheck, the
  611 tests and the build pass. Native acceptance stops at the reviewed binary until the root
  builds one from this source.
- **Graphics slot released.** Every native process this review started is closed:
  `pgrep -fl incant_editor` and `pgrep -fl /tmp/incant-native-review-0044/` both find
  nothing. The slot is free.
- **No gate claims.** No phase gate is approved or claimed, and nothing here makes a phone,
  device or performance claim.

## Model and effort

- Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Thinking effort: Max. The session reports reasoning effort "max".
- How it was verified: both prompts are the handoff runner's post-verification prompts.
  The runner's `configure_model_and_effort` (`tools/handoff/main.py`) sets both settings
  through ACP and raises before prompting unless ACP confirms them. I cannot see the
  runner's own confirmation output from inside the session.
- No substitution, and no routing exception.

## Priority revision: acknowledged and applied

- **Native review is authorized.** I used only the immutable root binary
  `/tmp/incant-native-review-0044/incant_editor`, SHA-256
  `718ea75b9d792015fa2af4d939d4af2e2e412c01c706b046a910075cf70bd1b6`:
  - It was launched with one fixture path and no probe flags.
  - It was never rebuilt or replaced. `launch.sh` checks the hash before each launch.
  - The matching helper `incant_headless` (SHA-256 `4aae16ce…6c0f`) was used only for
    public `init`, `command.execute` and `project.save`.
- **Fixtures.** The granted fixture was not modified (its files keep their 15:32
  timestamps). Extra fixtures were authored through public RPC only, under
  `artifacts/native-review-0044/`.
- **Settled upstream:**
  - the pointer physical→logical mismatch is fixed by Sol in `2a4be3a`;
  - wheel signs are unchanged;
  - duplicate Play publication is being fixed separately.
- **Director confirmation not needed.** Routine shortcut and default choices stand under the
  original authorization, so the two director questions from the browser stage are closed:
  the ⌘P/⇧⌘P/⌥⌘P shortcuts are kept, and an entity already selected at Start is read once at
  tick 0.
- **Not done:** I opened no account dialog, changed no saved account and sent no model
  prompt. Nothing outside the review window was captured.

## Native review (actual window)

**Setup.**
- macOS 26.6 (25G72), arm64, built-in Retina display (2 device px per pt).
- Layouts (window size in pt):
  - wide: 1440×874;
  - minimum: 1000×650;
  - narrow: 1440×866 (the visible-frame clamp) with the Inspector dragged to its 280 pt
    minimum.
- Screenshots: 29 files in `screenshots/native/`, labelled as actual native captures in its
  README. They are separate from the browser-double sets. Hashes are in
  `evidence/native-screenshots.sha256`; steps, coordinates and measurements are in
  `evidence/native-review.json`.

**Method.**
- Window-only captures: `screencapture -o -x -l` on the review PID's own window id.
- Input: CGEvents posted only while that PID is frontmost, rechecked before every event.
  `tools/native/winctl.swift` refuses points outside the window.
- The Mac was in use during the review, so each step:
  - waited for 15 s without keyboard or mouse input;
  - ran for a few seconds;
  - handed the front and the pointer back afterwards.
- The guard worked: once the person switched apps mid-step, the step aborted before any
  further input was sent.

**Fixtures.**
- **Granted fixture** (`artifacts/native-review-0044/project.json`): its script moves X by 2
  per tick.
- **`input-variant/`, then `input-variant-2/`:** each has scene "Main" with Key Probe, Pointer
  Probe, Camera and Side Camera, and scene "Lit Without Camera" with one point light. They
  were authored by `tools/native/author-input-variant.py`: `init`, two `command.execute`
  transactions (the bus takes one script source write per transaction), then
  `project.save`; revision 2.
- **Why there are two variants.** In the first variant both probes iterated every entity.
  The SDK documents `chunks` as world-wide views, so the pointer probe overwrote the key
  probe's X and Y. That was my probe's mistake, not an engine defect. `input-variant-2`
  filters by `instances`, as `crates/incant_script/tests/project_bindings.rs` does.
- Neither fixture has a mesh. During Play the viewport shows the native surface's empty clear
  colour. The cube in stopped captures is the authoring viewport's diagnostic cube, not game
  geometry.

### Verified in the native window

| Item | Result | Screenshots (`screenshots/native/`) |
| --- | --- | --- |
| Header controls and status | Preparing… with spinner, only Stop usable; Playing; Paused · tick N; Stepped; Stopped back to Live with focus on Play and "Play stopped. Runtime changes were discarded." | `wide-01`–`wide-05`, `wide-08` |
| Suspended | Amber "Suspended" while the window is in the background (after the 400 ms delay) | `minimum-03` |
| Step while running | Says "Pause Play to step one tick." | `wide-07` |
| Runtime rows | Equal the engine: X = 2 × tick at tick 79, 81, 57 and 155 (X 158, 162, 114, 310). Authored Transform stays 0 | `wide-04`, `wide-05`, `minimum-02`, `narrow-01` |
| Layouts | Header, controls, runtime rows and notice fit at all three sizes. The only truncation is the picker label at 1000 pt ("Lit Without …"); its full name is in the tooltip and accessible name | `minimum-*`, `narrow-01`, `error-02` |
| Picker | Absent for one scene with one camera. Three targets with the "No camera" note. Play without a choice opens it. Arrow keys and Enter choose, and focus returns to the picker with a ring. The playing scene is named: "Playing · Main" | `picker-01`–`picker-03` |
| Error placement | The engine's refusal sits in one strip under the header, with Dismiss on the right; it wraps to three lines at 1000 pt. The native surface follows the smaller hole: the strip covers y 152–223 px, the hole starts at y 226 px, and the diagnostic cube re-centres | `error-01`, `error-02` |
| Viewport hole | The native surface fills the game view exactly, at 246, 76, 833×533 CSS px (x 492–2157 px, y 152–1217 px), between the panel borders | `wide-03` |
| Keyboard | Shift+F6 and F6 reach the game view, which shows its 2 px inset ring. Play shortcuts work from the game view and from panels. Shift+Tab from the game view reaches Stop, then Step, so this WKWebView puts buttons in the Tab order. Space on an unavailable Stop announces "Play is not running." | `wide-06`, `keyboard-01` |
| Titlebar | Traffic-light centre y 39.5 px, logo 39.0 px, title on the same line, in a 39 pt header (border at 78–79 px). Measured identically at all three sizes and unfocused. Lights at 23 pt spacing; the logo starts 15 pt after the zoom button; the titlebar dims when unfocused | `alignment-*`, `minimum-03` |

### Game input numbers (`input-variant-2`, 60 Hz ticks)

| Steps | Key Probe | Pointer Probe | Screenshot |
| --- | --- | --- | --- |
| F6 into the game view (pointer outside it); D held 1.0 s; ⇧⌘P | tick 326: X 64, Y 0, Z 0, scale X 2 | | `input-01` |
| ⇧⌘P from the game view; left button held 0.5 s at window (520, 320); CGEvent wheel +3 lines; pointer to (800, 450); ⇧⌘P | tick 441: X 64, Y 34, Z 120, scale X 2 | tick 441: X 554, Y 374 | `input-02`, `input-03` |
| Key Probe clicked (focus in the Hierarchy); ⇧⌘P resume; 0.6 s; ⇧⌘P | tick 485: scale X **2**, expected 1 (defect 1) | | `defect-input-…` |
| F6 into the view; ⇧⌘P; D down; 0.5 s; F6 out; 1.0 s; ⇧⌘P; D up | tick 637: X 102 (+38 in 152 ticks), scale X 1 | | `input-04` |

What the numbers show:
- **Keys, buttons, wheel and focus** reach the game while the view has focus, and only then.
- **Pointer units.** The position is viewport-local CSS px: (800, 450) − (246, 76) =
  (554, 374).
- **F6 releases a held key.** X grew only while D was held in the view.
- **Wheel.** +3 synthetic CGEvent lines arrived as Z +120. That is a recorded observation
  only, and the signs are unchanged.

### Defects found natively

1. **The game kept focus after Resume from another panel. Fixed in UI source.**
   - **Trigger:** pause with the game view focused, click a Hierarchy row, then resume (⇧⌘P).
     The game reports `input.focused` true while keys go to the editor (`defect-input-…`,
     tick 485).
   - **Cause:** the worker queues `focus:false, focus:true` itself whenever a held session
     continues, based on window focus (`editor/app/src/play/worker.rs:340-345` in this
     worktree). The UI sent nothing, because its view never had focus in that stream.
   - **Fix (`src/play/input.ts`):** whenever the worker starts taking input (Start, Resume,
     the window coming back) while the view lacks focus, the UI sends `focus:false`. Queued
     events are dropped once the worker stops taking input, because it releases input
     itself.
   - **Tests:** 2 unit tests and 1 integration test; one existing test is restated for the
     new contract. All fail on the reviewed source and pass now.
   - **Remaining gap:** the game still sees focus for up to one status read (about 250 ms)
     after such a resume. See Astra note 9.
2. **Keyboard Dismiss left no element focused. Fixed in UI source.**
   - **Trigger:** Tab to Dismiss in the error strip, then press Space. No element shows a
     ring (`defect-keyboard-…`), and the next Tab restarts at the picker (`keyboard-03`).
   - **Fix (`PlayControls.tsx`):** when the focused Dismiss removes the notice, focus moves
     to Play. A mouse press, which does not focus buttons in WebKit, leaves focus alone.
   - **Tests:** 2 tests. The focus test fails on the reviewed source.
3. **Dragging a panel splitter selects text across panels. Reported, not changed.**
   - **Trigger:** drag the Inspector's splitter in the native window. The stepped Runtime
     value and the text between are selected (`defect-splitter-drag-selects-text.png`).
   - **Cause:** `editor/ui/src/components/Splitter.tsx` (handoff 0001) only captures the
     pointer on press. Nothing stops the press's default text selection.
   - **Suggested patch (untested):** on a primary-button press, call `preventDefault()` on
     the press, focus the separator explicitly, then capture the pointer. Alternatively, set
     `user-select: none` on the document while dragging.
   - **Owner:** this path is outside 0044's scope. It needs a shell UI follow-up with a
     regression test.

**Inconclusive.** One ⌘P right after the fresh `input-variant-2` window was first activated
did not open the picker. In the same second, the person at the Mac became active and the
guard aborted the step. On two later attempts (after activation, and after a click) ⌘P
opened the picker both times (captures in the ignored `artifacts/`). Not counted as a
defect.

## What the user sees

The design from the browser stage is unchanged except for the two fixes; `editor/ui/DESIGN.md`
says both in its Play section.
- **Header.** The state word (Preparing…, Playing · scene, Suspended, Paused · tick N, Play
  failed, Play unavailable) is followed by three plain 28 px transport buttons.
- **Buttons that cannot act** stay focusable, are marked `aria-disabled`, and say why when
  pressed.
- **Picker.** It appears only when there is a choice. The only scene, and a scene's only
  camera, are used without asking.
- **Notice.** One strip under the header gives the engine's words. Dismiss stops the failed
  session; from the keyboard, focus then moves to Play.
- **Game view.** It is a focusable application region with an inset ring. It sends keys and
  the pointer while the session runs. F6 and Shift+F6 leave it. The game has focus only while
  the view has.
- **Runtime block.** A sunken Inspector well with Translation, Rotation, Scale and Velocity,
  read on selection, Pause, Step and Refresh. It is never polled and never written to the
  project.

Browser-double screenshots, from the first stage (headless Chrome, deviceScaleFactor 2):
- `screenshots/normal/` (1440×900);
- `screenshots/minimum/` (1000×650);
- `screenshots/narrow/` (280 px Inspector);
- `screenshots/regression/`.

Their traffic and accessibility results are in `evidence/review.json`, `capture-g1.json`,
`pixel-g1-g2.json`, `regression-pixels.json` and `screenshots.sha256` (43 entries).

## Behaviour and input rules

These are unchanged from the browser stage, apart from defect 1:
- **When input flows:** only from a deliberately focused game view, in a focused window,
  while the status says running and not suspended, and no control request is pending.
- **Packets:** pinned to their generation; one in flight; at most one per 16 ms and 128
  events; the backlog capped at 1024.
- **Releases:** every stream opens with `focus:false, focus:true`. Leaving the view, or a
  refused packet, sends `focus:false`.
- **New:** a worker that starts taking input without the view's focus is sent
  `focus:false`.
- **Re-renders:** Play status reads stay out of the authoring snapshot, so panels do not
  re-render at 4 Hz.

## Notes and gaps for Astra (Rust, transport and engine are yours; not changed)

1. **Identical statuses republished.** Being fixed separately, per the revision.
2. **CLI wording in the editor.** Seen natively: "native play: spatial content requires an
   explicit --camera and native render host". The editor shows it verbatim. Non-blocking.
3. **Pointer units.** Fixed upstream in `2a4be3a`. Natively, the editor delivers
   viewport-local CSS px, (554, 374) as expected.
4. **Wheel sign.** Kept. A synthetic +3-line CGEvent arrived as Z +120. This is an
   observation only.
5. **Start reply timing.** As before: the generation is first visible on the next status
   read.
6. **Suspended right after Start.** As before.
7. **Untyped inspect errors.** As before.
8. **No unsaved-project signal.** As before.
9. **New: the worker decides game focus from window focus.** On every hold → run transition
   it queues `focus:false, focus:true` (`frame()`, worker.rs:340-345 in this worktree).
   - **Problem:** in the editor, the game view's DOM focus is what matters. A resume or
     window return with focus in another panel gives the game focus until the editor's
     correction arrives, up to one status read later.
   - **Suggestion:** in the editor host, queue only `focus:false` on those transitions and
     let the editor's stream open with `focus:true`. The UI's correction is then a harmless
     no-op.
10. **New, for the root's fixture.** `artifacts/native-review-0044/move.ts` adds 2 to every
    row of every chunk, so it also moves the Camera. That is harmless without a mesh. With a
    mesh added for visual validation, the camera would move with it and the model would look
    still. Filtering by `instances` avoids this (SDK `NativeContext.instances`).

## Evidence

**Tests.**
- **Now:** 35 files, 611 tests (`npm test`, 0 failures).
- **Browser stage:** 606, from a 545 baseline.
- **New this session:**
  - `src/play/input.test.ts` (16 → 18): focus is denied when the worker starts taking
    input; an unsent focus notice is dropped.
  - `src/components/play/Play.test.tsx` (30 → 33):
    - Resume with another panel focused sends only `focus:false`;
    - focused Dismiss moves focus to Play;
    - an unfocused Dismiss leaves focus alone;
    - the test "sends nothing until focused" is restated as "sends only that the game has
      no focus";
    - the `ops` helper now lists control requests only.
- **Mutation check:** with `input.ts` and `PlayControls.tsx` restored from `HEAD` (the
  reviewed source), three new behaviour tests fail. The fix was then reapplied from a saved
  patch: 33 of 33 pass.

**Native evidence.**
- `evidence/native-review.json`;
- `evidence/native-screenshots.sha256` (29 entries);
- `screenshots/native/README.md`.

All 54 raw captures (`native-r1/`, `native-r2/`) and the editor logs are in the ignored
`artifacts/0044-native-editor-play/`.

## Not run and limitations

- **Fixes in a rebuilt binary.** Not checked: this review used the immutable binary built
  before the fixes. Further native acceptance needs the root to build one from this source.
- **Visible game geometry.** Not shown: neither fixture has a mesh.
- **Not run:**
  - Windows and Linux hosts;
  - VoiceOver or another screen reader;
  - IME composition;
  - touch or trackpad gestures;
  - real wheel hardware (only synthetic CGEvents);
  - other device pixel ratios or multiple displays;
  - phones;
  - performance measurement;
  - hosted checks;
  - Cargo (no Rust was built).
- **Short worker holds and the macOS ⌘ release.** Unchanged from the browser stage: a key
  held through a short hold, or released while ⌘ is down, stays released or held until the
  next stream.
- **Not edited:** `STATUS.md` and `docs/spikes/native-editor-play.md` (Astra's), and
  `Splitter.tsx` (outside this packet's paths).

## Changed paths

- **This session, edited:**
  - `editor/ui/src/play/input.ts` and `input.test.ts`;
  - `editor/ui/src/components/play/PlayControls.tsx` and `Play.test.tsx`;
  - `editor/ui/DESIGN.md` (Notice and Game view paragraphs);
  - `handoffs/0044-native-editor-play/result.md`.
- **This session, new:**
  - `handoffs/0044-native-editor-play/tools/native/`:
    - `winctl.swift`: per-PID window control with the frontmost and idle guards;
    - `pixels.swift`: capture measurement;
    - `author-input-variant.py`: public-RPC fixture authoring;
    - `burst.sh`, `launch.sh`, `cap.sh`.
  - `screenshots/native/`: 29 PNGs and a README.
  - `evidence/native-review.json` and `evidence/native-screenshots.sha256`.
- **Browser stage (commit `1b01cdb`):**
  - **Edited:**
    - `editor/ui/src/App.tsx`;
    - `shell/ShellContext.tsx`;
    - `components/ViewportPanel.tsx`;
    - `inspector/InspectorPanel.tsx`;
    - `Menu.tsx`;
    - `ShortcutsDialog.tsx`;
    - `icons/Icon.tsx`;
    - `main.tsx`;
    - `styles/base.css`;
    - `DESIGN.md`;
    - `NATIVE_VIEWPORT.md`.
  - **New:**
    - `editor/ui/src/play/*`;
    - `components/play/*`;
    - `shell/ShellContext.test.tsx`;
    - `styles/play.css`;
    - `test/nativePlayDouble.ts`;
    - the handoff tools, screenshots and evidence.
- **Unchanged:**
  - Rust and `editor/bridge/*` (`node tools/build_bridge.mjs --check` passes);
  - Cargo and npm manifests;
  - CI, schemas and the SDK;
  - the granted fixture.

## Commands and results

```sh
# Native review (granted binary, one fixture path, no probe flags)
swiftc -O handoffs/0044-native-editor-play/tools/native/winctl.swift -o /tmp/incant0044/winctl
swiftc -O handoffs/0044-native-editor-play/tools/native/pixels.swift -o /tmp/incant0044/pixels
shasum -a 256 /tmp/incant-native-review-0044/incant_editor      # 718ea75b…bd1b6, checked before each launch
python3 -I handoffs/0044-native-editor-play/tools/native/author-input-variant.py \
    --headless /tmp/incant-native-review-0044/incant_headless   # {"project": ".../input-variant-2/project.json", "revision": 2}
tools/native/launch.sh <fixture> <log>                          # input variants; the granted fixture ran the same path-only command first; one editor at a time, each closed with SIGTERM
tools/native/burst.sh '<winctl steps; cap.sh captures>'          # idle-gated steps; every step exit 0 except the one aborted by the guard
pgrep -fl incant_editor; pgrep -fl /tmp/incant-native-review-0044/   # none: all native processes closed

# UI checks after the fixes (editor/ui)
npm run typecheck          # exit 0
npm test                   # exit 0: 35 files, 611 tests
npm run build              # exit 0: index-B42XLpcc.js 430.03 kB (gzip 131.96), index-N6ovSePc.css 76.34 kB
node tools/build_bridge.mjs --check   # exit 0, bridge unchanged
(cd handoffs/0044-native-editor-play && shasum -a 256 -c evidence/native-screenshots.sha256)   # 29 OK
```

**Logs** are ignored, in `artifacts/0044-native-editor-play/`:

| Log | SHA-256 prefix |
| --- | --- |
| `typecheck-native-fix.log` | `5e05d027` |
| `vitest-native-fix.log` | `ec94e3f3` |
| `build-native-fix.log` | `99cdc757` |

The browser-stage logs are listed in that commit's `result.md`.

## Open questions

1. **For the root:**
   - Build a binary from this source, so the two fixes can be accepted natively.
   - Route the Splitter defect to a shell UI packet.
2. **For Astra:** notes 2 and 5–10 above.
3. **For the director:** none. The routine choices are settled by the revision.
