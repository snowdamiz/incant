# 0057 native window game UI — result

## Status

Both parts of the packet are done.

**Priority Linux failure: diagnosed and fixed at the source.** Fixing the
reference showed nothing wrong in the renderer's geometry, clipping or
opacity. The 0056 reference test assumed exact arithmetic, but the Linux CI
device (Mesa llvmpipe) filters and blends at its documented 8-bit precision.

- The source patch corrects the reference. The shader now discards fragments
  that add nothing.
- Local Metal checks and an emulation of llvmpipe's arithmetic support the
  fix. Wrong geometry, clipping and opacity still fail.
- Not verified on Linux.

**Native window review: done on the actual composed player.** Real drawing and
input were exercised together on two displays (2x built-in, 1x external).
Drawing is correct: positions, clipping, selection, caret and composition,
pointer and keyboard focus, resize and DPI change.

The review found a P0 crash in the vendored Tao IME bridge, plus four
CPU-semantics problems. All are reported below for root's lanes, and none of
them was edited.

Nothing here claims complete Game UI, a live CJK/Arabic IME, phone or
performance results, or any phase gate.

## Model and effort

Claude Opus 5.5, model ID `claude-opus-5-5`, with thinking effort **max**.
Root's ACP record is `~/.codex/incant-evidence/0057/acp-resume-20261012.log`
("Claude session verified: Opus 5.5; thinking: max"). I cannot inspect the ACP
transport myself.

## Priority revision acknowledged

The brief's "Priority recovery and cross-platform failure — 2026-10-12"
section is applied (brief SHA-256 `97c004ad…7de9ec`).

- **Evidence and cache.** After the host restart, new evidence and logs are in
  `~/.codex/incant-evidence/0057`, and the cache is
  `~/.codex/incant-native-target`.
- **Nothing old counts.** No lost `/tmp` result or process from before the
  restart counts as a check.
- **Order.** The Linux diagnosis came first and the native review followed.
- **Untouched.** `tools/probes/render-check.py` and the Windows
  geometry-receipt failure are untouched.
- **Root's file.** `brief.md` is root's and stays uncommitted.

The two source edits made before the restart were preserved and finished:
the caret accessor and the tools.

## Part 1: Linux UI draw failures (checkpoint11)

Sources:

- Root's failure intake: `~/.codex/incant-integration-evidence/checkpoint11/`
  (log SHA-256 `b344c86d…f4ac61`, candidate `82aa2bc3`).
- Full record: `evidence/linux-ui-precision.json`.

**What failed, and what didn't.** On `ubuntu-latest` with
`mesa-vulkan-drivers` (lavapipe on llvmpipe):

| Case | Worst (codes) | Channels over 1 | Result |
| --- | --- | --- | --- |
| Panel at 2x | 7 | 275 | Failed |
| Nested clips (1x) | 2 | 1 | Failed |
| Panel at 1x, editor region, over world | 1 | 0 | Passed |

On the same runner, the world-text tests measured bilinear coverage errors up
to 0.00551 (about 1.4/255), within their alpha-space bound of 0.01.

**Cause.** llvmpipe's arithmetic, read from Mesa source (through the Android
mirror; freedesktop GitLab refused automated access):

- **Sampler** (`lp_bld_sample_aos.c`). It filters the R8 atlas in 8-bit fixed
  point: coordinates ×256 rounded, −128, then 8-bit weights.
- **Lerp** (`lp_bld_arit.c`, SSSE3/AVX2). Each lerp is
  `v0 + floor((w·Δ + 128)/256)` on 8-bit values.
- **Blending** (`lp_bld_format_srgb.c`). It decodes and encodes sRGB with
  polynomials between blends. Mesa documents ±0.35 for decode and ±0.17 before
  rounding for encode.

Near black, the sRGB encoding turns a 1/255 coverage step into 4–8 codes. The
0056 reference assumed exact filtering and exact, unquantized blending under a
fixed code tolerance, so it could not express this precision. That was the
defect. There was a secondary rendering issue: zero-coverage fragments (glyph
padding, outside rounded corners, the ring hole) still blended. On a
non-identity sRGB round trip this perturbs pixels that nothing covers.

**Emulation check.** A CPU emulation of exactly that arithmetic, run in CI's
configuration (old shader), compared with exact arithmetic by the old metric:

| Case | Emulation (worst / channels over 1) | CI log |
| --- | --- | --- |
| 2x | 7 / 303 | 7 / 275 |
| Nested | 2 / 2 | 2 / 1 |
| 1x | 2 / 1 | 1 / 0 |

The emulation omits llvmpipe's hardware rsqrt estimates in the encoder.

**Fix (commit `e2551dd2`).**

- **`ui.wgsl`.** A fragment whose alpha is zero is discarded. Pixels a quad
  does not cover keep their stored value on every backend. On Metal this
  changes no pixel; the changed-pixel counts are identical before and after.
- **`ui_draw_tests.rs`.** A `Model::Portable` reference gives every channel the
  stored codes the contract admits:
  - geometry, clips, layering, colours and rounded coverage stay exact;
  - glyph taps between texel centres may be off by `TAP` = 1/255, or 2/255
    when the position is off the 1/256 grid. This bound comes from 8-bit
    weights and two rounded lerp stages. Taps on texel centres, or between
    equal texels, are exact;
  - every blend of an sRGB target reads the destination within 0.5 code and
    stores within 0.67 code (±0.17 before rounding, as Mesa documents);
  - UNORM targets read back exactly;
  - the four GPU tests now require zero channels outside.
- **Two candidate models in the test file.** `Model::Exact` and
  `Model::Llvmpipe` (the emulation, from Mesa's source).
- **Docs.** The contract comment in `ui/mod.rs` and
  `docs/spikes/native-ui-draw.md` record the discard and the precision.

**Why the tolerance keeps the contract.** The admitted spans are wide only on
anti-aliased glyph edges between texel centres near black, where the sRGB
curve is steep:

| Case | Widest span (codes) | Channels wider than 2 |
| --- | --- | --- |
| 2x sRGB | 21 | 3.8% |
| 1.5x sRGB | 29 | 6.3% |
| 1x sRGB | 4 | 44 |
| UNORM | ≤ 2 | 0 |

Everything else is checked tightly. The new CPU test
`reference_admits_backend_precision_but_not_wrong_geometry_clip_or_opacity`
shows the bounds still catch real errors:

- **Admitted.** Exact arithmetic and the llvmpipe emulation, at every scale.
- **Rejected:**
  - text 1 px right: at least 3,053 channels, by up to 229 codes;
  - text half a pixel right at 2x: 3,460 channels;
  - 90% text opacity: at least 2,233 channels;
  - a label missing its ancestor clip: 146 channels, by up to 230 codes.

**Local Metal review.** Apple M5 Pro, macOS 27.0.1.

- **UI draw tests.** All pass with zero channels outside. The worst distance
  from exact is still 1 code.
- **Order check.** The text-under-selection reference rejects the real frame
  (279 channels, up to 47 codes).
- **Mutation harness** (`tools/mutate.py`, `evidence/mutations.json`). All
  eight mistakes were caught: pen +1 px, quarter-texel sampling, no clip
  intersection, 90% opacity, straight alpha, no sRGB decode, shadow without
  offset, caret +1 px. The unmutated suite passed before and after.

**Linux limitation.** Nothing ran on Linux or lavapipe here. Docker Desktop is
installed but not running, and I did not start it. No CI run was made. Root's
next frozen checkpoint is the real test. The reference assumes at least 8
sub-texel bits; Vulkan's minimum is 4.

## Part 2: native window review

### Build, fixture and package

| Item | Identity |
| --- | --- |
| Source | HEAD `ca111ccf` (packet base `5399dcd4` plus my two commits), tree `d276eaef` |
| Lock and toolchain | `Cargo.lock` `6d574acc…132016`; rustc 1.99.0 |
| `incant_headless` (ci) | `5a05216b…651d27` |
| `incant_game_player` (ci, `project,native-window`) | `4b6cc6a1…9462fa`, 31,119,248 bytes |
| Fixture `project.json` | `0821bf54…b643d` (`tools/fixture.py`) |
| Review script | `c9292969…49aa` |
| Fonts | NotoSans `2ec33f84…`, NotoSansArabic `ee489b99…`, IncantCJKFixture `3299d6ab…` |
| Package | assembled by the exporter's `data_package`; CPU startup check at 320×240: 18 widgets, 99 glyphs |
| Host | Apple M5 Pro (Mac17,9), Metal, macOS 27.0.1 (26A434) |
| Displays | built-in 1512×982 pt at 2x; external 3440×1440 at 1x |

The full list is in `evidence/identities.json`.

**Fixture.** `tools/fixture.py` authored a fresh project through public
commands only: `init`, `import` of the three packaged test fonts and three
generated cubes, then one atomic `command.execute` with a write and upsert of
the TypeScript script. It checks Undo/Redo and a journal reopen. New
identities use the `…57xx` range.

**Scene.**

- **UI:** a title; a wrapped Latin, Arabic and CJK body; Resume, Options,
  disabled Online and متابعة buttons; and Latin, Arabic and CJK caption-field
  rows.
- **World:** a crate spinning over a floor, beside a pillar.
- **Script:** makes UI outcomes visible in the world. Resume and متابعة step
  the crate between slots, Options pauses its spin, and `submitted` lifts it.

### Native event mechanisms used

- **Pointer:** CGEvent move/down/up at `kCGHIDEventTap`.
- **Keys:** CGEvent key down/up with US key codes. Shift and Option are real
  modifier events. Typed text goes through the managed `NSTextInputContext`.
- **Arabic and CJK text:** CGEvent key events carrying a Unicode string,
  committed by the managed context's `insertText:`. This is not an Arabic
  layout or a CJK input method.
- **Live composition:** the U.S. layout's Option-E dead key, then E. macOS
  produced real marked text (`setMarkedText:`), then a commit. This is not a
  synthetic preedit receipt.
- **Window operations:** resize, move and minimize through AX attributes;
  close through the AX close button.
- **Focus:** System Events app activation, and hand-back of the front.
- **Captures:** `screencapture -l` on the player window only.

**Not available.** Only the U.S. layout and Character Palette input sources are
enabled, so a live CJK or Arabic IME, its candidate window and multi-clause
conversion are untested. A synthetic press-and-hold (autorepeat) did not open
the accent picker; it inserted repeated "e".

**Burst rules.** Bursts ran only when the session was unlocked, the display
awake and the Mac idle for 20 s. Every event rechecked the front app, the
pointer and the lock. Holds yielded on real input; this happened twice
and was respected. The front app and pointer were always handed back.

### Runs

Six player processes ran; details are in `evidence/native-runs.json`, the
ledger in `evidence/processes.txt`, and crash excerpts in `evidence/crashes/`.

| Run | PID | Display | Exercised | Outcome |
| --- | --- | --- | --- | --- |
| 1 | 85772 | 2x built-in | Idle; Tab, Shift-Tab, Space, Return; pointer press/release, disabled click, activation; Tab into a field | Crashed on blur |
| 2 | 17610 | 1x external | Click focus; typing; selection; dead-key composition; Return; overflow; Home | Aborted on blur |
| 3 | 28481 | 1x | Arabic typing; RTL selection; mixed RTL + CJK | Crashed on blur |
| 4 | 35850 | 1x | CJK typing; select all; press-and-hold attempt | Crashed on blur |
| 5 | 41607 | 1x, then 2x | Resize 400×300 and 200×300; click after relayout; move to the 2x display; blur, refocus, minimize, restore | AX close, report printed |
| 6 | 69999 | 1x | The player's own `--lifecycle-probe` | Exit 0 |

**Run 5 report.**

- 662 ticks, 665 accepted frames, 3 presentation refreshes, 0 dropped ticks.
- 4 surface reconfigurations, 3 UI relayouts.
- Over the run, uploads were 4 atlas textures, 12 quad buffers and 4 frame
  writes. Unchanged frames reused their resources.

**Run 6 (probe).** Actual resize, minimize and restore, and an explicit surface
recreation. 41 frames; 1 relayout to 512×320.

### What the actual window shows (my visual judgment)

**Positioning and clipping are correct.**

- The startup frame (320×240 pt at 2x) matches the offscreen consumer within 2
  codes. The only differences are the moving crate, the window server's
  conversion of the saturated teal pillar into the Display P3 profile, the
  OS-rounded corners, and a 1-pixel macOS 27 title-bar hairline over content
  row 0. That hairline is OS chrome.
- After a live 1x→2x move, the frame is bit-identical to a fresh 2x start in
  the same UI state, apart from the crate. Reserved-frame relayout across a DPI
  change is exact.
- At 200 pt wide, the body and buttons wrap. A click at the wrapped
  coordinates of Options focuses it, so hit-testing follows the displayed
  layout.
- Field text overflowing its box is cut at the content box, and RTL text sits
  right-aligned.

**States read clearly.**

- Keyboard focus: a 2 px accent ring, which reads (93,144,247) in the
  colour-managed capture of #4C92FF.
- Captured press: a darker button, (32,34,39) against (47,50,57).
- Disabled: flatter and muted.
- Selection: an accent highlight under the text.
- Caret: 1 logical px, crisp at both scales.
- Composition: underlined. The dead-key "´" is underlined across exactly its
  width, with the input method's caret at its end.

**Text.** Text is crisp at 1x and soft at 2x, the known 0056 limitation. Arabic
joins correctly. The caret sits at the logical end, which is visual left in RTL
text, and at the trailing edge of an LTR run inside an RTL paragraph, as the
documented rule says.

**Legibility.** Captions placed directly over the bright floor at 2x are
legible but weak. The fill contrast is about 2.5:1 against (150,152,157); the
1 px shadow defines the lower edges. I left the theme unchanged. A stronger
shadow drifts toward a halo, and the real remedy is a backing surface, which
UI containers cannot draw today (proposal 5).

I made no other appearance changes. The one rendering change is the
zero-coverage discard, which is invisible on Metal.

### Findings for root (not edited; outside my lane)

1. **P0 crash in the vendored Tao managed IME client.**
   - **Mechanism.** `third_party/tao/src/platform_impl/macos/text_input.rs:26–44`
     subclasses `NSView`. It overrides `-window` (`owning_window`) to read the
     ivar `window: Weak<NSWindow>`. objc2 0.6.5's generated `dealloc` drops the
     ivars before `[super dealloc]`. objc2 itself warns about this case in
     `__macro_helpers/defined_ivars.rs`. `-[NSView dealloc]` → `_finalize` →
     `_removeFromKeyViewLoop` → `previousKeyView` → `_useFocusEngine` then
     calls the override, which loads the freed boxed `Weak`.
   - **Trigger.** `view.rs set_text_input_session` retires the client, called
     from the player's text-session sync in `window/mod.rs`.
   - **Reproduction.** Run any UI package with `--window`. Tab or click into a
     TextInput, then activate another app.
   - **Results.** Four of four runs died within 6 s on macOS 27.0.1. Three
     were `EXC_BAD_ACCESS` in `objc_retain`/`objc_msgSend`, with "Weak
     reference loaded … not in the weak references table". One was SIGABRT,
     "Rust cannot catch foreign exceptions". One run on macOS 26.6 before the
     restart crashed the same way.
   - **Impact.** Every text-editing session ends the game, and the run report
     is lost.
2. **A pointer click submits a text field.** `incant_ui/src/focus.rs`: a
   pointer release on the captured, focused widget calls `activate`, which for
   a TextInput emits `Submitted`. Natively, the crate lifted on the click
   itself. Proposed: a release focuses the field (and later places the caret)
   without submitting; Enter stays the submit gesture.
3. **Focus across blur and refocus.**
   - While the window is inactive, the frame still shows the focus ring (and
     would show a caret), because the blur is applied only at the next tick.
   - On refocus, the owner has cleared focus (`WindowFocus(false)`, and the
     host's `reset_focus` sends false then true), so nothing is restored.
   - Proposed:
     - keep the logical focus and its token across blur, but report it as not
       visible while inactive;
     - refresh the presentation once on blur, which the host can already do
       without ticking;
     - restore focus on refocus.
   - The IME session may still retire on blur.
4. **The caret is not kept visible in an overflowing field.** Single-line
   fields have no horizontal scroll, so after typing past the width the caret
   is clipped away. Proposed: the owner keeps a per-field horizontal scroll
   offset that keeps the caret inside the content box (same caret rule, small
   margin). It publishes this as `UiWidget.origin.x − scroll` with the clip
   unchanged; future pointer caret placement would use the same offset. The
   renderer needs no change.
5. **No backing surface for labels over bright scenes.** Proposed: a typed,
   optional panel surface for containers, drawn by the theme. This is a design
   decision for root and the director.

### Proposed CPU contracts (root coordinates)

**Canonical caret geometry for the IME.** Implemented in the renderer (commit
`ca111ccf`); not wired in.

- **API.** `incant_render::ui_caret(&UiWidget) -> Option<UiCaret { rect, visible }>`
  returns the exact rectangle the layer draws, in logical viewport pixels.
  That is the caret line box, 1 px wide, centred on the insertion edge. It
  covers a collapsed selection or the input method's cursor inside a
  composition, and returns None for every non-drawing state.
- **Host use, after each accepted presentation:**
  1. Find the widget whose id is the owner's text-focus token widget in the
     accepted `UiInput`. The accessor itself decides nothing about ownership.
  2. If the caret is visible, set the IME anchor to
     `LogicalPosition(rect[0], rect[3])` (bottom-left). Viewport coordinates
     are window-content coordinates, because the viewport is the reserved
     frame divided by the scale.
  3. If the caret is not visible, clamp the anchor to the widget's `visible`
     box. If there is no caret (a converted clause or a hidden cursor), keep
     the last anchor.
  4. Call `set_ime_position` only when the anchor changes.
- **Tao follow-up.** For a precise `firstRectForCharacterRange`, Tao's managed
  client should store a rectangle rather than a point.
- **Tests.** Numeric tests cover equality with the drawn quad, the RTL
  visual-left edge, the CJK composition caret, every no-caret state, and
  overflow or clipped carets.

**High-density text.**

- `UiSettings` carries the window scale. The owner rasterizes UI glyphs at
  `font_size × scale`, with the scale in the atlas key. `UiPaint` exposes the
  raster scale, and atlas `rect`/`origin` are in raster texels.
- The renderer then places texels at `frame.scale / raster_scale` physical
  pixels. That is exactly 1 at matching integer scales, with no resampling.
- A scale change already relayouts and re-rasterizes, as run 5 showed. The
  atlas byte cap stays fixed, with an explicit refusal on overflow.

## Changed paths

- **Commit `e2551dd2`:**
  - `crates/incant_render/src/ui/ui.wgsl`: zero-coverage discard.
  - `crates/incant_render/src/ui_draw_tests.rs`: portable reference, llvmpipe
    model, new CPU test.
  - `crates/incant_render/src/ui/mod.rs`: contract comment.
  - `docs/spikes/native-ui-draw.md`.
- **Commit `ca111ccf`:**
  - `crates/incant_render/src/ui/geometry.rs`: `UiCaret`, `ui_caret`; the draw
    path uses it.
  - `crates/incant_render/src/ui/mod.rs` and `crates/incant_render/src/lib.rs`:
    exports.
  - `crates/incant_render/src/ui_tests.rs`: accessor tests.
- **Handoff commit:** `handoffs/0057-native-window-game-ui/`, including
  `tools/`, `evidence/`, `screenshots/` and this result. Notable tools:
  - `review_offscreen.rs`: copied into the GPU host's examples only while
    `offscreen.sh` runs, then removed;
  - `winctl.swift`, `launch.sh`, `burst.sh`, `cap.sh`, `probe.sh`;
  - `mutate.py`, `pngtool.py`, `fixture.py`, `package.py`.

No file in `incant_runtime`, `incant_script`, `incant_game_runtime`,
`incant_ui`, `incant_render_game`, the player or window loop, the input queues
or Tao was edited.

## Commands and results

All runs are local on Apple M5 Pro Metal, macOS 27.0.1, using
`./tools/cargo … --locked --offline -j2 --profile ci` with
`CARGO_TARGET_DIR=~/.codex/incant-native-target`. GPU suites ran with
`--include-ignored --test-threads=1`. Logs are in `evidence/checks/` (trailing
blank lines stripped; raw hashes in `raw-sha256.json`).

| Kind | Command (abbreviated) | Result |
| --- | --- | --- |
| Offscreen GPU | `test -p incant_render --lib ui::` before the fix | 12/12; worst 1 everywhere (`baseline-ui-tests.txt`) |
| Offscreen GPU | the same after the fix | 13/13; 0 outside (`fixed-ui-tests-1.txt`) |
| Offscreen GPU | `test -p incant_render` (all, GPU included) | lib 90/90, `model_gpu` 50/50 (`test-render-full.txt`) |
| Offscreen GPU, hosted game | `test -p incant_render_gpu_host` | game 5/5, ui 2/2 (`test-gpu-host.txt`) |
| CPU | `ui::tests`, including the caret accessor | 8/8 (`ui-cpu-tests.txt`) |
| CPU | reference test, final and in CI configuration | pass; numbers above (`oracle-cpu-3.txt`, `oracle-cpu-old-shader-emulation.txt`) |
| GPU mutations | `tools/mutate.py` | 8/8 caught; unmutated before and after pass |
| Lint | `clippy -p incant_render -p incant_render_gpu_host --all-targets -D warnings` | clean |
| Lint | `fmt --all --check` | clean |
| Build check | `check -p incant_render --lib --target wasm32-unknown-unknown` | clean |
| Offscreen GPU | `offscreen.sh` (composed consumer, logging guest) at 320×240@2x, 480×300, 200×240, 640×480@1x | geometry and reference frames (`offscreen-geometry.json`) |
| Native window | six player processes | as tabled above |

Not run:

- Linux, Windows or lavapipe execution.
- Any CI.
- Player crate tests and Clippy: nothing in the player changed, but the native
  player built fine from this source.

## Screenshots

Native captures are colour-managed into the display's P3 profile, which is
kept in the files.

- `screenshots/native-2x-idle.png`: the startup window, 320×240 pt at 2x, the
  UI over the world.
- `screenshots/native-2x-keyboard-tab-shifttab-return.png`: Tab to Resume;
  Shift-Tab to Options; Return on Resume (the crate stepped).
- `screenshots/native-2x-pointer-pressed-disabled-activate.png`: a captured
  press on Options; a disabled click (focus unchanged); a click activating
  متابعة.
- `screenshots/native-2x-button-pressed-released-3x.png`: pressed against
  released fill.
- `screenshots/native-2x-field-tab-focus-caret-2x.png`: the field ring and
  caret at 2x.
- `screenshots/native-1x-click-submits-field.png`: before the click, after the
  click (crate lifted: the click submitted the field), and after Return.
- `screenshots/native-1x-latin-editing.png`: click focus, typing, selection,
  dead-key composition, commit, overflow (caret hidden), Home.
- `screenshots/native-1x-deadkey-composition-6x.png`: live marked "´" with its
  underline and caret, then "é".
- `screenshots/native-1x-arabic-rtl-editing-3x.png`: the RTL caret, typing,
  selection, and mixed RTL with CJK.
- `screenshots/native-1x-cjk-editing-3x.png`: CJK caret, typing, select all,
  and the press-and-hold attempt (repeated "e").
- `screenshots/native-1x-resize-400-200.png`: 400×300; 200×300 wrapped; a
  click on the relaid-out Options.
- `screenshots/native-2x-after-dpi-move.png`: after moving 1x→2x, focus
  retained.
- `screenshots/native-2x-blurred-refocused-restored.png`: the inactive window
  still showing the ring; refocused with focus cleared; restored after
  minimize.
- `screenshots/native-2x-captions-over-floor-4x.png`: caption legibility over
  the bright floor.
- `screenshots/offscreen-2x-startup-reference.png`: the offscreen consumer
  frame used for comparison (untagged sRGB).

## Limitations

- **Not complete (from 0056):** soft 2x text from logical-size rasterization
  (proposal above); no hover state; no content alignment (the Arabic caption
  is right-aligned within its column); candidate-window caret placement is not
  wired.
- **Untested:** a live CJK or Arabic input method, its candidate window and
  clause conversion; the press-and-hold accent picker; clipboard;
  accessibility.
- **Text editing was limited by the crash.** Each text-field scenario needed
  its own process, and only run 5 (no text focus) produced a run report.
- **Linux is unverified locally** (see Part 1).
- **No performance or phone claim.** Counts are resource evidence from a `ci`
  desktop build.
- **The review script is review content, not game art.** The CJK font is the
  small packaged subset.

## Resources, lease and cleanup

- **Lease.** `~/.codex/incant-evidence/0057/lease.json` was written at
  14:46 UTC and released at 16:01 UTC with the `released` marker (copy in
  `evidence/release.json`). Every guarded tool now refuses and creates
  nothing.
- **Processes.** None owned at release: all six players exited (four crashed,
  one closed, one probe), and no build was running.
- **Other lanes.** Builds in their own targets, an Android emulator and Brave
  were left alone.
- **macOS crash reports.** macOS wrote five `incant_game_player-*.ips` reports
  under `~/Library/Logs/DiagnosticReports`; they were left in place. Only
  excerpts are committed.
- **Cache.** `~/.codex/incant-native-target` is kept as a warm cache; nothing
  was deleted.
- **`node_modules`.** The worktree-root `node_modules` link (identical
  `package-lock.json`, needed for the TypeScript cook) is removed.
- **Root's brief.** `handoffs/0057-native-window-game-ui/brief.md` is left
  modified and uncommitted, as root wrote it.
