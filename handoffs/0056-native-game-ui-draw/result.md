# 0056 native game UI drawing — result

## Status

Implemented and verified offscreen; native-window review is deferred and no
gate is claimed. The screen-space GPU consumer draws the actual native UI tree
from the accepted `UiPaint` through the ordinary route: `NativeRenderHost` →
`RenderInputConsumer` → `GpuRenderConsumer` → `incant_render::UiLayer`.
`GpuRenderConsumer` now advertises `SUPPORTS_UI` because it really draws the
UI. It draws labels, buttons and text inputs over the finished game frame. It
shows disabled, focused and pointer-pressed states. It draws the caret,
selection and composition (preedit) underline from the exact runtime edit
state.

- **Native window:** not reviewed. Following the coordination update, the
  production window host still passes `ui: None`, so a UI scene cannot run
  there yet. That is the exact missing platform connection. I wrote the
  required startup wiring as a proposal patch for Astra's input lane (below).
- **Input:** no native input or IME claim of any kind. Routed keyboard input
  (Tab, Enter, Shift+arrows) and explicit UI receipts (focus, pointer, commit,
  preedit) were exercised offscreen through the real composed game.
- **Not claimed:** phone, performance, complete Game UI or any phase gate.

## Model and effort

Claude Opus 5.5, model ID `claude-opus-5-5`, with thinking effort **max**. The
model ID comes from this session's system configuration, and the effort from
its reasoning-effort setting (max). I cannot inspect the ACP transport myself,
so Astra's ACP verification remains the authoritative record of both settings.

## Priority revision acknowledged

The brief changed mid-session. Root added "Coordination update: native input
lane reserved" (brief SHA-256 `3984d34b4aa6…488b139` as read). I applied it
before any further commits:

- **Reserved files.** `crates/incant_game_player/src/window/{input.rs,mod.rs}`
  are unchanged. I had already wired UI into `window/mod.rs` (plus a size
  helper in `window/surface.rs`) before the update arrived. I reverted both
  files, and none of my commits touches `crates/incant_game_player`.
- **Wiring proposal.** The required startup wiring is a format-clean proposal
  patch, `evidence/window-ui-wiring.patch`. It is not applied; `git apply
  --check` passes on this tree.
- **Scope.** GPU tests and offscreen pixel review went ahead. No native
  composition is claimed (pinned tao exposes no public preedit, cancel or IME
  enable).
- **`UiPending::visual`.** This is my owned source, as the update states, and
  Astra's adapter can consume it.

## Commits

On base `6d941d41`. Source and tests:

| Commit | Content |
| --- | --- |
| `5451cc01` | `incant_ui`: read-only `UiVisual`/`WidgetVisual` candidate projection and tests |
| `47ef6463` | `incant_render` UI layer, `incant_render_data::UiInput`, consumer boundary, GPU consumer, tests |
| `1b98ddbf` | Appearance revision after pixel review; `ui_review` example harness |
| `0afb2c18` | Refusing-consumer boundary test (candidate visibility, nothing published on refusal) |
| `0fe9dff9` | Field outline raised to 3:1 non-text contrast |

A final commit adds this result, the spike doc, tools, fixture and evidence.
`brief.md` is root's file and stays uncommitted as root left it.

## Changed paths

- **`crates/incant_ui/src/{visual.rs,lib.rs}`, `tests/visual.rs`.** Borrowed
  projection of one prepared candidate (`UiPending::visual`) or the accepted
  owner (`UiRuntime::visual`). For each node in canonical layout order it
  reports focus, captured pointer press and the TextInput buffer. Read-only:
  focus, text, layout and routing are unchanged.
- **`crates/incant_render_data/src/{ui.rs,lib.rs}`.** Typed draw input:
  `UiInput`, `UiWidget`, `UiRole`, `UiEdit` and `SceneError::UiGlyph`.
- **`crates/incant_render/src/ui/{mod.rs,geometry.rs,theme.rs,ui.wgsl}`, `lib.rs`.**
  - Validation, caret and selection geometry, and quads, built on the CPU.
  - One pipeline per output format (the display catalogue).
  - `UiLayer::stage/accept/submit`, `Renderer::ui_layer` and `ui_uploads`.
- **`crates/incant_render/src/{ui_tests.rs,ui_draw_tests.rs}`.** 7 CPU and
  5 GPU tests.
- **`crates/incant_render_game/src/{lib.rs,ui.rs}`.**
  - `RenderInputConsumer::prepare/accept` take the candidate
    `Option<&Arc<UiInput>>`.
  - New `admit_ui`; `resize_ui` admits before the owner publishes.
  - The binding builds or reuses `UiInput`; `NativeRenderHost::ui_input`.
- **`crates/incant_render_gpu_host/src/lib.rs`.** `SUPPORTS_UI`; stages UI
  before taking live state or a target; submits it after the world frame;
  `ui_stats`.
- **`crates/incant_render_gpu_host/tests/ui.rs`.** Hosted composed-game GPU
  tests.
- **`crates/incant_render_gpu_host/examples/ui_review.rs`.** Offscreen review
  harness.
- **`crates/incant_render_gpu_host/Cargo.toml`, `Cargo.lock`.** Dev
  dependencies `incant_ui` and `png 0.18.1`, both already in the graph. The
  lock change is two lines in that package's dependency list.
- **Mechanical trait updates.** Test consumers in
  `crates/incant_render_game/tests/{hosted.rs,ui.rs}` and
  `crates/incant_play/tests/control.rs`.
- **`docs/spikes/native-ui-draw.md`** (new). The contract, appearance,
  resource bounds and open work.
- **`docs/spikes/native-ui-layout.md`.** One sentence corrected: GPU consumers
  now accept UI, but the window host does not configure it yet.
- **`handoffs/0056-native-game-ui-draw/`.** `tools/`, `fixture/`,
  `evidence/`, `screenshots/` and this result.

## Drawing contract (summary)

Full text: `docs/spikes/native-ui-draw.md` and `crates/incant_render/src/ui/mod.rs`.

- **Space.** Logical pixels, top-left origin, Y down. The target region (whole
  target or editor viewport rect) shows the logical viewport at
  `region / viewport` physical pixels per logical pixel.
- **Snapping.** Rectangle and clip edges snap to physical pixels. A glyph pen
  (content origin plus shaped position) snaps to the nearest physical pixel,
  and its padded atlas rectangle follows at `scale` pixels per texel.
  - At 1x every texel lands on a pixel.
  - Glyphs sit within 0.5 physical px of the layout's own shaped positions,
    with no reshaping and no reversal.
- **Clipping.** Each instance is intersected with its own clip in the vertex
  stage.
  - Glyphs use the effective content clip (ancestors ∩ own content box) on
    their actual padded bounds. An overflowing Word token or an RTL glyph at
    negative x is cut at the box.
  - Decorations use the visible border box.
- **Order.** One instanced draw per frame in canonical preorder. Per widget:
  surface, selection, glyphs (label shadow pass first), composition
  underline, caret, focus ring.
- **Colour.** Display-referred sRGB, never tone mapped.
  - sRGB targets blend premultiplied in linear light; UNORM targets blend
    encoded values.
  - Glyph coverage uses the world-text filter rule.
- **Edit state.**
  - The caret uses downstream affinity at wraps and direction boundaries,
    per-glyph bidi edges, a proportional split inside ligatures, and
    cosmic-text's paragraph rules for empty lines.
  - Selection becomes merged spans per line; mixed direction can yield
    several spans.
  - An input-method cursor range draws as a double-weight underline; `None`
    hides the caret.

## CPU contract: added projection and requests

The visual state of the *candidate* frame was `pub(crate)` in `incant_ui`. A
consumer drawing the candidate during `accept` could not see its focus,
captured press, selection or composition. Drawing the accepted owner instead
would be one tick stale, and its byte offsets could mismatch the candidate's
glyphs.

The narrow, read-only projection `UiPending::visual` / `UiRuntime::visual` (my
owned source, per root's update) exposes exactly that state. Nothing is
inferred from raw text: selection and composition ranges come from
`TextBuffer::display_selection()` and `preedit().display_range`. The binding
also checks that the buffer's display text equals the paint's shaped source
before building.

Requested, not implemented:

1. **Hover.** The owner has no pointer-motion state, so no hover visual is
   drawn. A hover visual needs a hovered widget in the owner, with the same
   delivery and epoch rules as pointer receipts.
2. **HiDPI glyph rasterization.** The UI atlas is rasterized at the logical
   font size. At 2x the glyphs are bilinear-magnified and visibly softer than
   the pixel-crisp surfaces (see `desktop-2x-cjk-preedit.png`).
   - Narrow addition: raster UI glyphs at `font_size × scale`, with the scale
     in the atlas key, and expose the raster scale on `UiPaint`.
   - The consumer then maps texels at `1/scale`.
   - A scale change re-rasterizes, within the same 4 MiB atlas bound.
3. **Content alignment.** Button and field text sits at the content-box
   origin. Centring is only possible through padding and sizes.
4. **Window startup wiring (Astra's lane), per `evidence/window-ui-wiring.patch`:**
   - `UiSettings` at `inner size / scale factor` when `scene.has_ui()`.
   - Before every reserved presentation (tick or refresh), relayout to the
     reserved surface's logical size through `resize_ui` (which admits before
     publishing).
   - UI stats in the run report.
   - Platform pointer receipts and IME commit/preedit delivery are root's.

## Appearance and my visual judgment

The goal was a restrained, professional control set that reads over any game
scene: one dark control family, light text, and a single accent used only for
focus and selection.

| Element | Value | Note |
| --- | --- | --- |
| Text | `#ECEEF2` | 11.1:1 on buttons, 15.4:1 on fields |
| Button | `#2E323A` | |
| Button, pressed | `#1F2228` | darker while the press is captured |
| Button, disabled | `#23262C` | flatter fill, text at 41% alpha |
| Field | `#15171C` | 1 px `#646A77` outline marks an editable region |
| Accent | `#4C92FF` | 2 px inset focus ring, 4.2:1 against buttons |
| Selection | accent at 43% | selected text 5.0:1 |
| Corner radius | 4 px | |

There is no glow, gradient, decorative border or animation. The caret does not
blink, because no presentation clock is involved.

Changes made after reviewing the actual pixels myself:

1. **Opaque surfaces.** At 92% alpha, fields showed the scene's horizon line
   through the control.
2. **Disabled buttons are opaque.** They had been see-through over the pillar,
   which read as a glitch.
3. **Label shadow.** Labels sit directly on the scene by construction. The
   "الاسم"/"日本語" captions were illegible over the bright floor, so labels
   now get a hard 1 px shadow at 59% black: invisible on dark scenes, legible
   on bright ones.
4. **Lower composition underline.** It now sits at 0.2 em below the baseline
   so it clears CJK descent.
5. **Outline contrast.** The field outline went from 2.75:1 to 3.3:1 against
   the field and 3.36:1 against the backdrop.

Fixture authoring changes (not renderer changes): buttons hug their labels,
caption and field pairs are grouped, and the floor is darker.

I judge the result acceptable at 1x:

- Text is crisp, because pens snap to pixels.
- The hierarchy (title, body, actions, fields) is clear.
- Every state reads at a glance: keyboard focus, captured press, disabled,
  caret, selection, composition with IME caret, and the RTL caret at the
  visual left end.

The weak point is 2x text softness (request 2 above). Surfaces, rings and
carets are crisp at 2x.

## Resource reuse (measured, review fixture at 960×540)

From `evidence/review.json`, desktop-1x case. Counts are cumulative since
startup; the moving world never adds UI uploads.

| Step | Atlas uploads | Instance uploads | Frame-uniform writes | Quads | UI GPU bytes |
| --- | --- | --- | --- | --- | --- |
| Idle tick | 0 | 0 | 1 | 240 | 37,746 |
| Tab focus | 0 | 1 | 1 | 241 | 37,810 |
| Pointer press | 0 | 2 | 1 | 241 | 37,810 |
| Focus field, then Shift+←×8 selection | 0 | 4, then 5 | 1 | 241 | 37,810 |
| Commit "Byron" (text changes) | 1 | 6 | 1 | 238 | 37,670 |
| Preedit 語 (display text changes) | 2 | 9 | 1 | 240 | 37,798 |
| Commit 語 (display text unchanged) | 2 | 10 | 1 | 239 | 37,734 |

Unchanged frames upload nothing. A widget-state change re-uploads only the
instance buffer, and a text change adds one atlas prefix, because the CPU
owner re-rasterizes per paint. The frame uniform is written once per target
size.

Worst-case bounds (in `docs/spikes/native-ui-draw.md`):

- Instances: ≤ 169,984 quads, 10.4 MiB.
- Atlas prefix: ≤ 4 MiB.
- Accepted plus candidate overlap: ≤ 28.8 MiB.
- These are in addition to the CPU owner's two 4 MiB planes.

All numbers come from a ci or debug desktop build. They are resource counts,
not performance evidence.

## Fixture, package and review route

- **Fixture.** `tools/fixture.py` authors a fresh project with the
  final-source `incant_headless` (ci, SHA-256 `4c6db850…0bc704`), using only
  `init`, `import` and one atomic `command.execute`, then checks Undo/Redo
  and a journal reopen.
  - Imports: the three packaged test fonts and three generated cubes.
  - Scene: floor, pillar, a crate moving by native `Velocity`, key light and
    camera.
  - UI: title, a wrapped Latin, Arabic and CJK body label; four buttons
    (one disabled, one Arabic); three caption-and-field groups (Latin, Arabic
    RTL, CJK).
  - No Script: motion is native, so the review harness needs no project
    program.
  - Project `fixture/project.json`, SHA-256 `50048d2a…07f7aff5`.
- **Package.** `tools/package.py` builds a fresh `ci` CPU package player
  (`project,render-input`, SHA-256 `df31e695…a1fa9678`) in the granted
  target. The exporter's own `data_package` assembles the ordinary package,
  and its CPU startup check ran with `--ui-viewport 960x540`.
- **GPU review.** The `ui_review` example (ci, `a524793f…2f49181`) loads that
  package's cooked scene, models and fonts. It drives them through
  `HostedOfflineGameSession<NativeRenderHost<GpuRenderConsumer>>` with
  offscreen sRGB targets at three sizes:
  - desktop 960×540 at 1x (960×540 px);
  - desktop 960×540 at 2x (1920×1080 px);
  - narrow 360×640 at 2x (720×1280 px).
  Inputs are routed keys (Tab, Shift+ArrowLeft, Enter) and receipts (focus,
  pointer press/release, commit, preedit). A logging guest stands in for the
  package's embedded program. It recorded `activated` (Options),
  `value_changed` "Ada Byron", `value_changed` "日本語" and `submitted`
  "日本語".
- **CPU parity.** `tools/cpu_parity.py` replays the same sequence as an
  ordinary v2 input recording through the packaged CPU player:
  `--package data --ui-viewport 960x540 --input-recording`. The final
  accepted owner matches the GPU route exactly: focus, all three values,
  155 glyphs and UI atlas SHA-256 `58d4a5f3…7ff22527`
  (`evidence/parity/comparison.json`). The GPU consumer therefore drew the
  same canonical paint the CPU route publishes.
- **Hardware.** Apple M5 Pro (Mac17,9), Metal, macOS 26.6 (25G72).

## Commands and results

All runs are local, on the final source `0fe9dff9` (tree `abe9b80e`), with
`CARGO_TARGET_DIR=/tmp/incant-agent-native-target` and `./tools/cargo … --locked
--offline -j2`. Tests use the `ci` profile. The GPU suites ran with
`--include-ignored --test-threads=1` under the packet's lease, on Apple M5 Pro
Metal. Logs are in `evidence/checks/`: trailing blank lines are stripped, and
the raw hashes are in `raw-sha256.json`.

| Command (abbreviated) | Result | Log |
| --- | --- | --- |
| `cargo fmt --all -- --check` | clean | `fmt.txt` |
| `clippy -p incant_ui -p incant_render_data -p incant_render -p incant_render_game -p incant_render_gpu_host -p incant_render_native -p incant_play --all-targets -- -D warnings` | clean | `clippy-render.txt` |
| `clippy -p incant_game_player --all-targets -- -D warnings`, default and `--features native-window` | clean | `clippy-player*.txt` |
| `test -p incant_ui` | 30/30, including 3 new `visual.rs` | `test-ui.txt` |
| `test -p incant_render -- --include-ignored` | lib 88/88 (12 UI: 7 CPU, 5 GPU), `model_gpu` 50/50 | `test-render-gpu.txt` |
| same UI tests with `--nocapture` | every GPU case within 1 code value of the reference, 0 channels > 1 (1x, 2x, 1.5x sRGB; 1x UNORM; offset region; nested clips; over the world) | `ui-gpu-numeric.txt` |
| `test -p incant_render_game -p incant_render_native -p incant_render_data` | render_game: hosted 8/8, ui 7/7 (includes the new refusal test); render_native: 29/29 | `test-render-game.txt` |
| `test -p incant_render_gpu_host -- --include-ignored` | game 5/5, ui 2/2 (hosted composed UI) | `test-gpu-host.txt` |
| `test -p incant_play` | 9/9 | `test-play.txt` |
| `test -p incant_game_player --features native-window` | 32/32 unit tests, `routing.rs` 1/1 | `test-player-window.txt` |
| `check -p incant_headless -p incant_editor --all-targets` | clean | `check-downstream.txt` |
| `check -p incant_render --lib --target wasm32-unknown-unknown` | clean | `check-wasm.txt` |
| `tools/mutate.py`: one mutation at a time, byte-exact restore | 16/16 caught; unmutated suites pass | `evidence/mutations.txt` |
| `tools/fixture.py`, `tools/package.py` | atomic authoring with Undo/Redo and reopen; package startup check passes | `evidence/{fixture,package}.json` |
| `ui_review` example; `tools/cpu_parity.py` | 3 cases and 17 captures; CPU and GPU routes equal | `evidence/review.json`, `evidence/parity/` |

The 16 mutations, each caught by the selected tests:

- **Shader:** no clip intersection; an unsnapped glyph pen; no sRGB decode;
  straight-alpha output; the stroke hole ignored.
- **Geometry:** glyphs drawn before the selection; the RTL caret on the LTR
  edge; the label shadow drawn without its offset; glyphs clipped to the border
  box instead of the content clip.
- **Retention:** identical input not reused; a shared atlas never reused; a
  stage accepting another renderer.
- **Host:** the accepted UI offered instead of the candidate; a relayout
  published without admission; changed widget state ignored for reuse.
- **GPU consumer:** the staged UI never submitted.

Two earlier observations no longer hold:

- **Player `routing.rs`.** 0054 recorded it as a pre-existing failure; it now
  passes.
- **`incant_play` `animation.rs`.** This TypeScript cook test needs
  worktree-root `node_modules`. Without it the test failed, which is
  environmental. With the documented root symlink (identical
  `package-lock.json`) it passes.

## Screenshots

All are offscreen frames of the actual composed game and GPU consumer.
Nothing here is a native window capture.

- `screenshots/desktop-1x-idle.png`: no focus; disabled "Online play"; label
  shadows over the floor.
- `screenshots/desktop-1x-field-selection.png`: the name field focused (inset
  ring) with "Lovelace" selected under the text.
- `screenshots/desktop-1x-cjk-preedit.png`: composition 語 underlined with
  the IME caret.
- `screenshots/desktop-2x-cjk-preedit.png`: the same at 2x (crisp surfaces,
  soft magnified glyphs).
- `screenshots/narrow-2x-field-selection.png`: 360×640 logical at 2x. The
  body text wraps (Latin, Arabic, CJK) and the button row wraps.
- `screenshots/button-states-3x.png`: idle, keyboard focus and captured
  pointer press, zoomed 3x.
- `screenshots/field-states-3x.png`: the RTL caret at the logical end (visual
  left) and the CJK composition, zoomed 3x.

## Remaining limitations

- No native-window pixels, input or IME. The window host does not yet
  configure UI; that wiring belongs to Astra's reserved lane.
- 2x text is soft until glyphs are rasterized at device scale (request 2).
  There is no hover state (request 1) and no content alignment (request 3).
- The GPU UI admission refusal is non-terminal inside the consumer: accepted
  resources stay drawable. However, the game composition still treats any
  host sync failure as terminal, as before.
- The caret does not blink. No clipboard, accessibility tree, UI save state,
  editor Play UI (editor Play still passes `ui: None`) or phone/device
  measurement.
- The review fixture's CJK font is the small packaged subset (中文日本語一).
  The fixture is review content, not game art.

## Resources, lease and cleanup

- **Lease.** I created a fresh lease marker, `/tmp/incant0056/lease.json`, at
  04:37 UTC and released it at 05:33 UTC with `/tmp/incant0056/released` (copy
  in `evidence/release.json`). The guarded packet tools (`package.py`,
  `mutate.py`, `checks.sh`) now refuse, and I checked that they create
  nothing. `fixture.py`, `cpu_parity.py` and `pngtool.py` are CPU or
  file-only. The `ui_review` repo example has no lease check of its own, like
  the other renderer examples; run it only under a GPU lease. No 0054 or 0055
  tool, marker or evidence was reopened or changed.
- **GPU use.** Offscreen only: the headless renderer in tests and the
  `ui_review` example. No native window was launched and no screen capture
  was taken, so no foreground app or pointer was disturbed.
- **Processes.** No owned player, helper, renderer or build process was
  running at release. Each build, test and harness run ran in the foreground
  or was waited on to completion.
- **Other lanes, left alone.** At startup, CPU-only exports and tests of other
  lanes were running in their own target directories. During review, a
  `git merge` and `cargo test -p incant_agent` ran in the separate
  `ci-build-cache` checkout (`evidence/processes.txt`).
- **Target directory.** `/tmp/incant-agent-native-target` (28 GB) is kept as
  warm cache, and nothing in it was deleted. Disk had 356 GiB free at release.
  The `/tmp/incant0056` scratch keeps the fixtures, packages and full review
  frames; only curated copies are committed.
- **`node_modules`.** The worktree-root `node_modules` symlink to
  `../../node_modules` existed only for TypeScript cooking (the `incant_play`
  animation test and the project player builds) and is removed. The two `package-lock.json` files were identical (SHA-256
  `cbea068b…5337c60`).
- **Root's brief.** `handoffs/0056-native-game-ui-draw/brief.md` (root's
  coordination update) is left modified and uncommitted, as root wrote it.
- **Suggested root doc update.** `STATUS.md` still lists screen-space GPU
  drawing as open. Its accurate state is: the GPU consumer is implemented with
  offscreen evidence; native window wiring, platform input/IME, hover, HiDPI
  glyph raster and UI checkpoint state remain open. I corrected only the one
  now-false sentence in `docs/spikes/native-ui-layout.md`.
