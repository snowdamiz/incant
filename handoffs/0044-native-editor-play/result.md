# 0044 native editor Play: result

## Status

- **Done for the browser stage.** The editor now has native Play controls, runtime
  inspection and desktop game input, wired to the real bridge contract:
  - **Header.** Play/Pause/Resume, Step and Stop, plus the session state, sit in the
    viewport header line. A scene and camera picker appears only when there is a choice
    to make.
  - **Notice.** A failed session or an unreadable status gets one strip under the header.
  - **Runtime block.** The Inspector shows a read-only block for the selected entity, read
    on selection and after Pause and Step. It is never polled and never written back.
  - **Game view.** Once deliberately focused, it sends bounded, generation-pinned input.
  - **Shortcuts.** ⌘P, ⇧⌘P and ⌥⌘P (Ctrl elsewhere), listed in the shortcut reference.
- **Pending: native actual-window review.** It waits for Astra's native binary or fixture
  and an explicit graphics-slot grant. There is no grant in `artifacts/tools/`. I built and
  launched no native renderer, window or GPU process, and claim no native visual
  acceptance.
- **No gate claims.** No phase gate is approved or claimed. Nothing here makes a phone,
  device or performance claim.

## Model and effort

- Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- Thinking effort: Max. The session reports reasoning effort "max".
- How it was verified:
  - The prompt I received is the handoff runner's post-verification prompt
    (`tools/handoff/main.py`, lines 225–240).
  - `configure_model_and_effort` sets both settings through ACP. It raises before
    prompting unless ACP confirms Opus 5.5 and `max`.
  - I cannot see the runner's own "Claude session verified" output from inside the
    session.
- No substitution, and no routing exception.

## Packet revisions and director feedback

- **Revisions.** The current brief has no separate priority-revision section and no new
  director note. Nothing from an earlier attempt existed: the worktree was clean, with only
  `brief.md`.
- **Standing feedback, as applied:**
  - **Restrained colour.** Colour is only on state dots: Playing green, Paused blue,
    Suspended amber, failures red. The one tinted surface is the error strip, which
    matches the existing project banner.
  - **Compact header.** Every Play control lives in the existing 36 px viewport header
    line. There is no new toolbar, panel or row.
  - **No ornament.** No stripes, glows, pills, badges or explanatory rows. Generation,
    sequence, frame and discarded-tick counters are not shown.
  - **Account dialog.** Pixel-identical before and after, in two states.
  - **Shortcut dialog.** Gains one "Play" section in its own style.

## What the user sees

Screenshots are under `screenshots/` (headless Chrome, deviceScaleFactor 2):
- `normal/` is 1440×900 with a 360 px Inspector.
- `minimum/` is 1000×650 with a 340 px Inspector.
- `narrow/` is 1440×900 with the Inspector at its 280 px minimum.
- `regression/` holds element crops.

### Viewport header

**State.** The header's state word changes with the session:
- **No session.** The viewport's own word shows (Live, Not attached, …).
- **Preparing… / Stopping….** Shown with a spinner.
- **Playing.** With several scenes, the playing scene follows: "Playing · Harbor".
- **Suspended.** Shown only once a worker hold lasts 400 ms. The tooltip explains it.
- **Paused · tick 1,204.**
- **Play failed / Play unavailable.**

**Transport.**
- Three plain 28 px buttons, always in the same place: Play/Resume/Pause, Step and Stop.
- A button that cannot act stays focusable, is marked `aria-disabled`, and says why when
  pressed.
- Each button's accessible name says its action. `aria-keyshortcuts` and the tooltip give
  the platform's keys.

**Picker.**
- **When it appears:** only with several scenes, or several cameras in the scene.
- **Labels:** "Choose scene", "Harbor · Choose camera" or "Harbor · Player Camera".
- **The list:** one radio list of every scene and camera. Its button is the toggle.
- **Defaults:**
  - The only scene, and a scene's only camera, are used without asking.
  - Play without a needed choice opens the list, and choosing starts Play.
  - Choices are remembered per scene for the editor session.
- **No camera:** a scene without a camera starts without one, and the engine decides.

See `normal/choose--stopped.png`, `choose--picker.png`, `choose--preparing-polled.png`,
`single--stopped.png` and `keyboard--choosing.png`.

### Notice

- **When:** a failed session, or a status that cannot be read.
- **What it says:** "Play could not start." or "Play stopped at tick 42.", then the
  engine's own words.
- **Dismiss:** a failed session's notice has Dismiss, which stops that session. An
  unreadable-status notice has no Dismiss: it clears itself when reads recover.
- **Announced once.** The notice is an alert, and Play's earlier status-line message is
  cleared so it cannot contradict it.
- **Viewport bounds.** The notice moves the viewport down, and the bounds are reported
  again (see Evidence).

See `normal/unsaved--refused.png`, `runtime-error--failed.png` and
`transport--unreadable.png`.

### Game view

- **Focusable region.** While a session exists, the viewport host is a focusable
  `role="application"` region named "Game view". It has a spoken note: keys and the pointer
  go to the game, and F6 leaves.
- **Focusing.** A click focuses it, and that press is not sent. F6 lands on it.
- **Focus ring.** A 2 px focus ring is drawn inside the view's edge however it was focused
  (`normal/game-input--focused.png`).
- **Keys.** While the session runs, keys and the pointer go to the game, including ?. F6,
  the Play shortcuts and ⌘ combinations stay the editor's.
- **Ending.** A session that ends with focus in the view moves focus to Play.

### Runtime block (Inspector)

- **Where:** a sunken well above the authored components.
- **Header:** "Runtime", the tick it was read at, and a refresh button.
- **Rows:**
  - Translation, Rotation, Scale and, when present, Velocity.
  - They use the authored rows' own renderer and schema. Rounded values show; exact ones
    are in the tooltip.
  - Field IDs are in a separate namespace, and no diagnostics are shown.
- **Explicit cases:**
  - An entity the game removed: "No runtime state." plus the engine's reason.
  - An entity of another scene: "Not in the running scene. Play runs Harbor." No request is
    sent.
  - A scene node: no block.
  - A host without `play.inspect`: says so.
- Authored values never stand in for runtime values.

See `normal/running--refreshed.png`, `pause-step--stepped.png`, `missing--absent.png`,
`missing--other-scene.png`, `older-host--labelled.png` and `narrow/`.

## Behaviour

| Situation | What happens |
| --- | --- |
| Play pressed | `play.start` with the scene, and the camera when there is one. The bridge adds the session generation and authoring revision. The header says "Preparing…" at once; Stop works at once |
| Stop before any status read shows the new session | Held, then sent with the new session's generation as soon as a read shows it. If the start fails first, the stop is dropped and the failure notice stays |
| Stop while preparing | Sent. The header shows "Stopping…" until the cancelled startup finishes; Play waits. The startup's late "Play startup was cancelled" is silent and never replaces the stopped state |
| Start refused or failed | If the session reports the failure, the notice shows it, once. If the session is unchanged (a bridge refusal), the status line says "Play could not start: …" |
| Pause / Step | Each re-reads the selected entity. Step stays unavailable until paused. A second press while a request is pending says to wait |
| Selection or session changes during a read | The older read is discarded. Only the newest read for the current selection and session is shown |
| Status unreadable | "Play unavailable" plus the notice. Stop (and Mod+P) still try: the bridge reads again first, then stops, or the status line says why it could not |
| Dismiss | Stops the failed session; Play waits until that is done |
| Unsupported host or fixture | Controls are marked unavailable: "Native Play is unavailable: the sample fixture is read-only." |

**Input rules** (`src/play/input.ts`, `src/play/useGameCapture.ts`):

- **When input flows:**
  - The view has deliberate focus in a focused window.
  - The status says running and not suspended.
  - No control request is pending.
- **Events sent:**
  - `focus`.
  - `key` with generated KeyCode names. Keys the engine lacks and auto-repeat are dropped
    explicitly.
  - Left, right and middle `mouse_button`.
  - `pointer_position` in viewport-local CSS px.
  - `wheel` in pixels or lines, as +x right and +y away from the user (DOM `deltaY`
    negated, matching what the native window player passes through from tao).
- **Packets and limits:**
  - Each packet carries the generation its events were collected in.
  - A new generation drops queued events and ignores late replies.
  - One packet in flight, at most one per 16 ms, at most 128 events each.
  - A pointer run keeps its first and latest position, and wheel deltas are summed.
  - The backlog is capped at 1024. Over that, everything is released and the user is told
    once.
- **Releases:**
  - Every stream opens with `focus:false`, `focus:true`, which releases whatever the game
    still holds.
  - Leaving the view, or a refused packet while the worker still takes input, sends
    `focus:false`.
- **Errors:**
  - A single refusal (usually a pause racing a packet) is not reported.
  - A repeated one is announced once per message, with no log flood.

**Re-renders.** Play status reads arrive at up to 4 Hz while a session runs. They are kept
out of the shell snapshot (`AuthoringSnapshot`), so panels do not re-render. Play views
select only the fields they show; running ticks are not among them.

## Independent review and fixes

A separate review agent read the diff and the backend semantics. It reported six bugs and
several minor issues. All were checked against the code, fixed and covered by tests:

1. **Dev builds sent no game input.** In StrictMode, React's extra effect cleanup disposed
   the input queue for good. `dispose()` is now reversible. Tested under StrictMode.
2. **Stop did nothing while the status was unreadable.** It was offered, but the handler
   gave up. It now really tries; the bridge reads again first, then stops or says why.
3. **A release could be lost.** If the packet after a blur was refused, `focus:false` was
   never sent. Releases now always go out while the worker takes input.
4. **Held keys could stay held in the game.** The editor forgot them without telling the
   game when input paused for editor-side reasons. Every stream now opens with a release,
   and editor-side stops send `focus:false`.
5. **Step or Resume could be dropped silently.** A press while a request was pending was
   ignored without a word. It now says to wait. The test that flaked now waits for the
   completed pause.
6. **The picker's button could not close its menu.** It now can; the menu ignores presses
   on its anchor.
7. **Minor issues, all fixed:**
   - ⌘ combinations stay the editor's (macOS sends no key release for them).
   - A P release with a modifier still reaches the game.
   - Dismiss counts as stopping.
   - A Play press during an unfinished startup says so.
   - No inspection is sent while a Step is pending.
   - A queued Stop checks the phase.
   - On macOS, Ctrl+P in a text field stays the text field's.
   - The runtime block shows only a read of the current selection.
   - A weak test was rewritten.

**Mutation checks.** Restoring the snapshot pass-through, the terminal `dispose()`, the
inert Stop or the anchor-less picker makes its new test fail. The files were restored and
compared byte for byte with `cmp`.

## Evidence

**Tests.** UI tests went from 30 files and 545 tests (baseline) to 35 files and 606
(final): 61 new tests in five new files. The final suite passed five consecutive full
runs. New files:

| File | Covers |
| --- | --- |
| `src/play/keys.test.ts` (2 tests) | The KeyCode list equals `schemas/InputEvent.schema.json`; unsupported browser codes are dropped |
| `src/play/input.test.ts` (16 tests) | <ul><li>packets of at most 128, one in flight, one per frame</li><li>pointer and wheel coalescing and sign</li><li>generation pinning and no relabelling</li><li>release on blur and on refusal</li><li>the 1024 backlog</li><li>error reporting without floods</li><li>reuse after dispose</li></ul> |
| `src/play/model.test.ts` (11 tests) | Scenes and cameras per scene, target choice, all states, runtime counterpart of the selection |
| `src/components/play/Play.test.tsx` (30 tests) | The real `NativeBridge` class against the double: request wiring, picker, preparing and pending Stop, stale startup, failures, Dismiss, unreadable status, inspection and stale reads, missing entities, older hosts, game input including StrictMode and a 300-event burst, shortcuts, axe |
| `src/shell/ShellContext.test.tsx` (2 tests) | Play status reads do not re-render authoring consumers; capability mapping |

**Browser review.**
- **Sets.** g1 and g2: 73 steps each, at the three sizes, on the final production build.
- **Results:**
  - 0 axe violations (axe-core 4.14.0, colour contrast included).
  - 0 page errors, and 0 requests to any other origin.
  - No header or page overflow. The only truncated text is the picker label at 1000 px,
    whose full name is in its tooltip and accessible name.
  - The macOS titlebar inset is 78 px wherever the double reports macOS chrome.
  - Inspector widths: 360, 340 and 280 px.
- **Native placement.** In all 71 steps that use the double, the viewport bounds the bridge
  sent equal the game view's rectangle × device pixel ratio. That includes the moments when
  the notice moves the view.
- **Recorded traffic.** 107 commands and 39 input packets were recorded. The largest packet
  had 5 events; none was refused, and every packet carried generation 1. No command
  carried a stale generation.
- **Noise.** 68 of 73 captures are identical between g1 and g2. The other five differ by
  4–193 pixels on anti-aliased corners and text (masks checked).

**Regression** (baseline `3910a85`, extracted with `git archive` into ignored artifacts
and built there). Five of seven element crops are pixel-identical:
- the account dialog, signed in with several accounts;
- the account dialog, signed out;
- the titlebar;
- the hierarchy;
- an authored Inspector (Crate 03).

Two change by design:
- the shortcut dialog, which gains the Play section;
- the viewport header, which gains the transport.

Before and after crops are in `screenshots/regression/`.

**Evidence files** (`evidence/`):
- `review.json`: what each curated capture shows, plus totals.
- `capture-g1.json`: every step, command and input packet.
- `pixel-g1-g2.json` and `regression-pixels.json`.
- `screenshots.sha256`: 43 entries; `shasum -a 256 -c` gives 43 OK.

### Coverage

| State | 1440×900 | 1000×650 | 280 px Inspector |
| --- | --- | --- | --- |
| Stopped with a choice; picker open | ✓ | ✓ | ✓ |
| Preparing (Stop available) | ✓ | ✓ | ✓ |
| Playing with runtime block; refreshed | ✓ | ✓ | ✓ |
| Paused; stepped | ✓ | ✓ | ✓ |
| Stopping a preparing start; after the cancelled startup | ✓ | | |
| Stop before the session is known | ✓ | | |
| Unsaved refusal; dismissed | ✓ | ✓ | ✓ |
| Runtime error | ✓ | ✓ | |
| Unreadable status | ✓ | | |
| Removed entity; other scene | ✓ | | ✓ |
| Suspended | ✓ | | |
| Game view focused; input sent; F6 left | ✓ | ✓ | |
| Keyboard only (rings, list, game view) | ✓ | ✓ | |
| Single scene and camera (no picker) | ✓ | ✓ | |
| Fixture (unsupported) | ✓ | | |
| Older host without `play.inspect`/`play.input` | ✓ | | |
| Shortcut reference | ✓ | | |

Every ticked cell is a step of set g1, recorded in `evidence/capture-g1.json` and kept in
the ignored `artifacts/0044-native-editor-play/g1/`. `screenshots/` holds a curated subset
of 43.

## What is real and what is simulated

- **Real:**
  - the production UI build;
  - all UI logic;
  - the compiled `editor/bridge/native.generated.js`, and in tests the `NativeBridge`
    class. They poll `play_read` and send `play_command` exactly as in the editor.
  - authored values converted by the real `snapshotFromEngine` from the repository's
    generated schemas.
- **Simulated** by the clearly labelled native-bridge double (`src/test/nativePlayDouble.ts`;
  the status line reads "Native-bridge double (simulated Play, not engine)"):
  - Every Play result is simulated. The double mirrors the worker's rules and messages:
    generations, the startup gate, Stop during preparation, deferred Step, the 128/1024
    limits and inspection.
  - It never cooks, simulates physics or renders.
  - Runtime transforms are simple functions of the tick.
  - Ticks advance only when the review calls `frame()`.
- **Not shown:** the viewport hole is transparent in a browser, so captures show the page
  behind it, not game pixels.
- **Not counted:** mock and browser tests are transport and visual evidence only; they are
  not a live gate.

## Gaps and questions for Astra (not changed: transport and Rust are yours)

1. **Identical statuses republished.** `NativeBridge.updatePlay` republishes an unchanged
   status on every 4 Hz read, including when idle, so every subscriber is notified. The UI
   now ignores these reads, but skipping the publish when generation and sequence are
   unchanged would help any other consumer.
2. **CLI wording in the editor.** Engine refusals use CLI wording: "choose --scene when the
   project does not have exactly one scene" and "spatial content requires an explicit
   --camera". The editor shows them verbatim. Editor-facing wording would read better.
3. **Pointer units.** The editor sends viewport-local CSS pixels, as the brief specifies.
   The native window player passes tao's `PhysicalPosition` (device pixels), while the
   InputEvent schema says logical pixels. Please confirm which is intended, for parity.
4. **Wheel sign.** The editor sends `[deltaX, -deltaY]`, matching tao's documented
   +y-away convention that the player passes through. Please confirm it numerically. tao's
   Linux path negates both axes, so horizontal may differ there.
5. **Start reply timing.** `play.start` replies only after cooking finishes. The new
   generation is first visible on the next status read, so the UI holds an early Stop
   until then. A start that published `preparing` with its generation at once would remove
   that gap.
6. **Suspended right after Start.** The start reply reports `suspended: true` until the
   first focused frame. The UI waits 400 ms before saying "Suspended".
7. **Untyped inspect errors.** Inspect errors are plain strings (`engine.request`). A
   typed code for an absent entity would let the UI word it without matching text.
8. **No unsaved-project signal.** No snapshot field says a project is unsaved, so the UI
   relies on the start refusal.

## Not run and limitations

- **Not run:**
  - any native app, window, renderer or GPU process (no grant);
  - WebKit/WKWebView;
  - VoiceOver or any screen reader;
  - phones;
  - Cargo;
  - hosted checks;
  - performance measurement.
- **Environment:** headless Chrome 155.0.8059.40, playwright-core 1.64.0, Node v22.23.3,
  vitest 5.0.3, TypeScript 5.9.3. Dependencies came only from the local npm cache.
- **Short worker holds.** A hold shorter than one status read (for example while a surface
  is outdated) releases keys inside the game while the UI still tracks them. A key held
  through it stays released until pressed again. The native window player also drops key
  repeats.
- **macOS ⌘ release.** A game key held before ⌘ and released while ⌘ is down gets no key
  release from the browser on macOS. It stays held until blur, pause or the next stream.
- **Stop message heuristic.** "Play start cancelled" versus "Runtime changes were
  discarded" depends on the last status read.
- **Not edited:** `STATUS.md` and `docs/spikes/native-editor-play.md` (Astra's). Its
  "Remaining UI work" paragraph is now out of date.
- **Not reviewed natively:** the native viewport, inset and focus behaviour in WKWebView,
  pending the native review.

## Changed paths

- **Edited:**
  - `editor/ui/src/App.tsx`: Play provider, shortcuts.
  - `editor/ui/src/shell/ShellContext.tsx`: snapshot without Play status, quiet requests,
    `capabilityOf`.
  - `editor/ui/src/components/ViewportPanel.tsx`: header state and controls, notice, game
    view.
  - `editor/ui/src/components/inspector/InspectorPanel.tsx`: runtime block.
  - `editor/ui/src/components/Menu.tsx`: radio items, notes, anchor.
  - `editor/ui/src/components/ShortcutsDialog.tsx` and its test: Play section.
  - `editor/ui/src/icons/Icon.tsx`: play, pause, step, stop, refresh.
  - `editor/ui/src/main.tsx`.
  - `editor/ui/src/styles/base.css`: menu check and note.
  - `editor/ui/DESIGN.md`: Play section, keyboard, styles table.
  - `editor/ui/NATIVE_VIEWPORT.md`: game view, input, focus.
- **New:**
  - `editor/ui/src/play/`: `PlayContext.tsx`, `model.ts`, `input.ts`, `keys.ts`,
    `useGameCapture.ts` and tests.
  - `editor/ui/src/components/play/`: `PlayControls.tsx`, `RuntimeSection.tsx` and
    `Play.test.tsx`.
  - `editor/ui/src/shell/ShellContext.test.tsx`.
  - `editor/ui/src/styles/play.css`.
  - `editor/ui/src/test/nativePlayDouble.ts`: test support, not in the app bundle.
  - `handoffs/0044-native-editor-play/`: `result.md`;
    `tools/{play-double,capture,regression,curate}.mjs`; 43 files under `screenshots/`;
    and `evidence/`.
- **Unchanged:** Rust, `editor/bridge/*` (`tools/build_bridge.mjs --check` passes), Cargo
  and npm manifests, CI, schemas and the SDK.

## Commands and results

```sh
npm ci --offline --ignore-scripts --no-audit --no-fund   # local npm cache only; 211 packages
npm run typecheck --workspace editor/ui                  # baseline and final: 0 errors
npm run test --workspace editor/ui                       # baseline 30 files / 545; final 35 files / 606 (five consecutive full runs)
npm run build --workspace editor/ui                      # final: index-Hq2TUUUA.js 429.89 kB (gzip 131.93), index-N6ovSePc.css 76.34 kB
node tools/build_bridge.mjs --check                      # generated bridge current and unchanged
git archive HEAD editor/ui editor/bridge sdk/ts schemas | tar -x -C artifacts/0044-native-editor-play/baseline-src
(cd artifacts/0044-native-editor-play/baseline-src/editor/ui && npx vite build)   # baseline index-BT33RD53.js, same as the untouched tree
node handoffs/0044-native-editor-play/tools/capture.mjs g1   # 73 steps, 0 axe, 0 errors, 0 remote
node handoffs/0044-native-editor-play/tools/capture.mjs g2   # same
node handoffs/0035-editor-union-review/tools/pixel-diff.mjs artifacts/0044-native-editor-play/g1 artifacts/0044-native-editor-play/g2 artifacts/0044-native-editor-play/pixel-g1-g2   # 68 of 73 identical (noise)
node handoffs/0044-native-editor-play/tools/regression.mjs artifacts/0044-native-editor-play/baseline-src/editor/ui reg-base1
node handoffs/0044-native-editor-play/tools/regression.mjs editor/ui reg-cand2
node handoffs/0035-editor-union-review/tools/pixel-diff.mjs artifacts/0044-native-editor-play/reg-base1 artifacts/0044-native-editor-play/reg-cand2 artifacts/0044-native-editor-play/reg-diff2   # 5 of 7 identical
node handoffs/0044-native-editor-play/tools/curate.mjs g1 reg-base1 reg-cand2   # 43 screenshots
(cd handoffs/0044-native-editor-play && shasum -a 256 -c evidence/screenshots.sha256)   # 43 OK
```

**Logs** are ignored, under `artifacts/0044-native-editor-play/`. SHA-256 prefixes:

| Log | SHA-256 prefix |
| --- | --- |
| `vitest-baseline.log` | `e64bbb95` |
| `vitest-final.log` | `d368cc8d` |
| `typecheck-final.log` | `5e05d027` |
| `build-final.log` | `6e965caf` |
| `build-baseline-src.log` | `70ad21f6` |
| `g1.log` | `fb5a9586` |
| `g2.log` | `fb5a9586` |
| `pixel-g1-g2.log` | `380c48f8` |

## Open questions

1. **For the director:**
   - **Shortcuts.** ⌘P / ⇧⌘P / ⌥⌘P follow the common game-editor convention, and Ctrl+P
     on Windows overrides the browser's Print there. Keep them?
   - **Runtime read at start.** An entity already selected when Play starts is read once
     at tick 0, then on Pause, Step and Refresh. Is that the right moment?
2. **For Astra:**
   - Gaps 1–8 above.
   - The final native-window review: the native viewport under the game view's ring, the
     WKWebView focus and keyboard routing, the macOS ⌘ release behaviour, and input
     numerics with a real game.
