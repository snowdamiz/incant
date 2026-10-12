# 0054 canonical native text drawing — result

## Status

Implemented and locally verified, with one reported foundation defect and no
gate claimed. The renderer draws the composed native text input through the
canonical `SceneInput` → `NativeRenderHost` → `RetainedScene` →
`GpuRenderConsumer` path used by the native game window and editor Play. The
explicit text refusals in `scene::resolve`, `Renderer::prepare_retained` and
`RetainedScene::stage` are gone because those three now resolve, upload and
draw text. Numeric GPU tests pass on Metal. The ordinary package/window entry
point was exercised for rendered review, and I reviewed the pixels myself.

- Rendered review found a nonvisual **foundation layout defect**: the
  cosmic-text wrap drops a space and overflows the line width after an RTL run.
  It is reported below with a repro and a proposed narrow repair, not fixed here.
- Not done: widget layout, editable text/IME, focus/gamepad navigation and the
  Phase 1 Game UI gate. Nothing in this packet claims them, any phone, shipping,
  memory or frame-pacing gate, or approval of a phase gate.

## Model and effort

Claude Opus 5.5, model ID `claude-opus-5-5`, with thinking effort **max**. The
model ID comes from this session's system configuration, and the effort level
from the session's reasoning-effort setting (max). I cannot inspect the ACP
transport myself, so Astra's ACP verification of model and effort remains the
authoritative record.

## Commits and changed paths

- `264fd08867c6d0bdf5fe236f37500b5e90b9f00f`: source and tests (tree
  `f0a72cd612523abd3702eab5014d79cd7addfa5c`), on base `122aefb` and the
  frozen foundation `6144e1e`.
- A follow-up commit adds the docs, packet tools, fixture copy, screenshots,
  evidence logs and this result. `brief.md` and `work.json` are root files,
  unchanged.

Source:
- `crates/incant_render/src/text/{mod.rs,geometry.rs,drawing.rs,text.wgsl}` (new):
  plan/preflight, identity caches, retained transforms, pipeline, shader.
- `crates/incant_render/src/{scene.rs,retained.rs,models.rs,graph.rs,lib.rs}`:
  resolution, retained staging/submission, upload, `Pass::Text` after sprites,
  exports (`TEXT_LOGICAL_PIXELS_PER_UNIT`, `TEXT_COLOR`, `text_layout_to_local`,
  `Renderer::text_uploads`).
- `crates/incant_render/Cargo.toml`: `incant_text` moves from dev to normal
  dependency (it was already in the lock entry; no resolution change).
- `crates/incant_render_data/src/{lib.rs,text.rs}`: `SceneError::TextGlyph`;
  `SceneStats.text_{entities,glyph_quads,missing_glyphs,gpu_bytes}`; doc text.
- `crates/incant_render_native/src/lib.rs`: the `TextUnsupported` message now
  names the missing font binding (that refusal remains for spatial-only adapters).
- `crates/incant_game_player/src/{window/mod.rs,package.rs}`: window
  `--font-cache FONT_CACHE`; `--package … --window` passes `data/fonts`.

Tests:
- `crates/incant_render/src/{text_tests.rs,text_draw_tests.rs,text_retained_tests.rs,text_native_tests.rs}`
- `crates/incant_render/src/typed_scene_tests.rs` and
  `crates/incant_render/tests/graph_ownership/mod.rs` (text pass)
- `crates/incant_render_gpu_host/tests/game.rs`: hosted mixed text game.

Docs, tools and evidence:
- `docs/spikes/native-text-draw.md` (new) and `docs/spikes/native-window-player.md`
- Packet tools: `handoffs/0054-native-text-draw/tools/` (`fixture.py`, `mutate.py`,
  `winctl.swift`, `launch.sh`, `burst.sh`, `cap.sh`, `review.sh`, `pngcrop.py`)
- `handoffs/0054-native-text-draw/{fixture,screenshots,evidence}/`

## Coordinate and presentation contract

Full text: `docs/spikes/native-text-draw.md` and `crates/incant_render/src/text/mod.rs`.

- **World-space route.** `TextNode.world` is used unchanged. Layout logical
  pixels map into the entity's local XY plane at 100 px per local unit: layout
  `(x, y)` with Y down becomes local `(x/100, −y/100, 0)`. The layout box's
  top-left corner is the entity origin, and line `i`'s baseline sits at local
  `y = −baseline_i/100`. Entity translation, rotation, scale and shear apply as
  they do for sprites. There is no screen anchoring or billboarding, and nothing
  is inferred from the font size. With a pixel-perfect camera at 100 px per
  unit, layout pixels land 1:1 on frame pixels; the tests use that alignment.
- **Glyphs.** The coverage rectangle starts at `position + origin` (the Y-down
  bearing from the baseline origin), with the atlas entry's size plus its
  one-texel zero padding; one layout pixel equals one atlas texel. Spaces draw
  nothing. `.notdef` draws its own outline. Shaped order, bidi placement and
  font fallback are taken from the layout as given.
- **Depth and clipping.** Quads are two-sided and depth-tested `LessEqual`
  against opaque geometry, with no depth writes. They draw after sprites, in
  node stable-ID order and then glyph order. There is no widget clip rectangle.
- **Coverage and color.** Coverage is linear area coverage and blends
  premultiplied over the HDR scene. Text radiance is unlit linear white
  `(1,1,1)`, the same as an untinted white unlit sprite texel; the shared tone
  map displays it at linear 0.9 (sRGB 243). Authored text has no color field yet.
- **Filtering.** Each pixel takes one bilinear tap up to 1.25 texels of
  footprint. Above that it averages an N×N box (N ≤ 4) of taps, clamped to the
  glyph's own padded rectangle. There are no mipmaps.

My reasons for this policy:
- It keeps the hierarchy transform's existing meaning, so text behaves as a
  sprite-like plane.
- 100 px per unit makes 16–48 px labels 0.16–0.48 units tall in a metre-scale
  world, and gives exact 1:1 placement under a 100 px-per-unit pixel-perfect
  camera.
- Using the top-left origin matches layout and future widget boxes.
- White linear radiance matches unlit sprites and reads on the default
  backdrop. Styling belongs to the Game UI layer.

## Retention, publication and resource accounting

- **Planning.** `scene::resolve` counts every node and handle before any
  allocation: at most 1,024 nodes, sorted with unique IDs, and 65,536 handles.
  Each handle must resolve in this exact atlas, so a foreign or reset handle
  fails with `SceneError::TextGlyph`. Device limits are checked before the scene
  uploads anything else.
- **Identity.** The atlas prefix texture and the glyph quad buffer are
  immutable. They are keyed by the exact atlas `Arc` plus each node's ID,
  layout `Arc` and handle `Arc`, never by a revision number. Holding the atlas
  `Arc` keeps it immutable and its pointer unique.
- **Motion.** Motion rewrites only a preallocated 256-byte node transform
  slot. Topology rebuilds that keep layouts share geometry and upload nothing.
- **Refusal.** A rejected stage (changed identity, missing binding, bad
  transform or bad target) happens before any GPU write and leaves the accepted
  frame unchanged and retryable. Failures after submission keep the existing
  terminal host contract.

| Resource | Bytes | Bound |
| --- | --- | --- |
| Atlas prefix texture (shared by identity) | prefix width × height (R8) | ≤ 4 MiB |
| Glyph quads (shared by identity) | quads × 32 | ≤ 2 MiB |
| Node transforms, accepted + staged | 2 × nodes × 256 | ≤ 512 KiB |
| CPU staging rows, accepted + staged | 2 × nodes × 256 | ≤ 512 KiB |
| Transient expanded quads at upload | quads × 32 | ≤ 2 MiB, dropped after upload |
| Per-frame camera/color uniform | 80 + bind group | per drawn frame |

The old and candidate scenes coexist during a text change, which can take text
GPU memory to about 13 MiB. That is in addition to wgpu upload staging and to
the CPU atlas planes counted in `native-text-input.md` (old plus candidate, up
to 8 MiB). The review fixture's shipping player reported a 4 MiB CPU alpha
plane. By construction its GPU texture holds only the referenced prefix, but
the window run does not report text GPU bytes, so that size was not measured
there.
`SceneStats.text_gpu_bytes` reports actual atlas prefix, quad and transform
bytes and stays the same across live/staging swaps (tested).

## Commands and results

All runs are local macOS 26.6 (25G72) on an Apple M5 Pro (`Mac17,9`) with
Metal, using rustc 1.99.0 (b940084d7), `CARGO_TARGET_DIR=/tmp/incant-agent-native-target`,
`./tools/cargo … --locked --offline -j2`, and the `ci` profile unless noted.
GPU tests are `#[ignore]` and ran with `--include-ignored` under the grant.
Final logs on the committed source are in `evidence/*.txt`.

| Command (abbreviated) | Result | Log |
| --- | --- | --- |
| `cargo fmt --all -- --check` | clean | `evidence/fmt.txt` |
| `cargo clippy -p incant_render -p incant_render_data -p incant_render_native -p incant_render_gpu_host -p incant_render_game -p incant_text --all-targets -- -D warnings` | clean | `evidence/clippy-render.txt` |
| `cargo clippy -p incant_game_player --all-targets -- -D warnings` (default and `--features native-window`) | clean | `evidence/clippy-player-*.txt` |
| `cargo test -p incant_render -- --include-ignored --test-threads=1` | lib 71/71 with 10 text tests (3 CPU planning, 1 typed-scene, 6 GPU draw/retention/native), `model_gpu` 50/50, all on Metal | `evidence/test-render-gpu.txt` |
| `cargo test -p incant_render_gpu_host -- --include-ignored` | 5/5, including the new hosted mixed text game | `evidence/test-gpu-host.txt` |
| `cargo test -p incant_render_native -p incant_render_game -p incant_render_data -p incant_text` | 27 + 8 + 0 + 26 passed (CPU text input, hosted mixed game, shaping and atlas suites unchanged) | `evidence/test-cpu-text.txt` |
| `cargo test -p incant_game_player --features native-window` | 31/31 unit tests, including window `--font-cache` parsing and package expansion; `routing.rs` 0/1, a pre-existing failure (see Known limitations) | `evidence/test-player-window.txt` |
| `cargo test -p incant_play` | 9/9 | `evidence/test-play.txt` |
| `cargo check -p incant_play -p incant_headless -p incant_editor --all-targets` | clean | `evidence/check-downstream.txt` |
| `cargo check -p incant_render --lib --target wasm32-unknown-unknown` (the nightly CI command) | clean | `evidence/check-wasm.txt` |
| `python3 -B tools/mutate.py` (each mutation applied, tests run, exact bytes restored) | 8/8 mutations caught (padding offset, unflipped Y, unstaged motion, atlas prefix missing its padding column, single minified tap, accepted missing binding, no depth test, unchecked foreign handles); the unmutated tree passes and each file was restored byte for byte | `evidence/mutations.txt`, `evidence/mutation-summary.txt` |

Measured numeric agreement, from `--nocapture` runs of the same tests (the
inputs are deterministic):
- Text pass against the independent CPU composition: maximum alpha error
  0.00135 at 1:1, 0.00201 at 0.42× rotated (3×3 taps), 0.00227 at 0.24×
  (4×4 taps) and 0.00324 at 1.7× magnification.
- Display check through the canonical graph: within 1 sRGB code value of
  `tone_map(1.0) = 0.9` over the backdrop.
- Retained frames are byte-identical to fresh frames over 8 moving frames and
  3 target sizes; staging allocates nothing; motion and rebuild upload zero
  text resources.

## Rendered acceptance (native, actual pixels)

**Entry point exercised:** the ordinary package/window route. Editor Play was
not launched natively in this packet; it uses the same consumer, which the
hosted GPU test covers with an offscreen target.

The steps:
1. `tools/export_native.py PROJECT OUT --camera 00000000000000000000000010 --mode window`
   built a fresh shipping player: `opt-level=3`, `codegen-units=1`, fat LTO,
   `panic=abort`, split debuginfo `packed`. The build ran in the exporter's
   private temporary target, which it deleted before its copied-player startup
   check. That check ran the CPU route with 6 text nodes, 3 fonts, 86 glyphs,
   17 RTL glyphs and 1 missing cluster.
2. The player then ran as `incant_game_player --package data --window` under
   `env -i`. Its only variables were PATH (the player directory), HOME (an
   empty scratch directory) and TMPDIR.

Identities:
- Source: `264fd088…` (above).
- Player: `a5d6a9bd455f4a79112abad78a1f234cfbe045517a2677233ec419df832fdc85`,
  12,540,448 bytes, aarch64-apple-darwin, unsigned.
- Authoring CLI: `incant_headless`, `ci` profile, same source,
  `b3d5139c8981fd466e2755c5f09d7e81c93b01c1f24673a3d96ae8a982c66fb9`.
- Fixture: `fixture/project.json`
  `045e3d50233e3c8f18ff7384c22eb9abf7d3b53f49f83d3ce01171cdf62a892f`, authored
  by `tools/fixture.py` using only `init`, `import` (the three packaged test
  fonts, a generated cube glTF and a silent WAV) and one atomic `command.execute`
  transaction, then `project.save`. The journal reopen matched; no project JSON
  was edited.
- Content: Latin ligatures (`Office affinity fluffy waffle`), a right-aligned
  Arabic paragraph (`مرحبا بالعالم`), CJK fallback (`中文 日本語`), a wrapped
  mixed line with an uncovered Hebrew letter, a label parented to a lit,
  rotating cube, and a line behind the cube. The TypeScript behaviour sways the
  mixed line and yaws the cube. The audio clip is silent on purpose, so the
  composed audio path runs without sound.

The player's exit report:
`{"tick":4073,"accepted_frames":4073,"adapter":"Apple M5 Pro","backend":"Metal","surface_reconfigurations":4,"surface_recreations":0,"surface_skipped":1,"presentation_refreshes":0,"dropped_wall_ticks":"0","on_screen":true}`.
I took three window-only captures (`screencapture -o -x -l`, Display P3
tagged); everything else is crops of those three. Each capture ran in a short
burst after at least 15 s of user inactivity, and the previous front app and
pointer were restored afterwards. The game window shows no account avatar, so
nothing is redacted.

| Image | What it shows | Review |
| --- | --- | --- |
| `screenshots/1-default.png` (640×544, startup 320×240 pt) | Whole scene after about 50 s of focused simulation (the window kept focus for about 45 s after launch; see Process) | All strings present and placed per contract. Arabic is right-aligned in its 640 px box; the CJK space is kept; the label lies in the yawed cube face; "Behind the cube" is depth-occluded. |
| `screenshots/1-default-latin-ligatures-x3.png` | 3× crop | `ffi`, `fl`, `ff`, `ffl` render as single ligature glyphs. Edges are clean, with no padding seams or atlas bleeding. |
| `screenshots/1-default-mixed-wrapped-x4.png` | 4× crop of the wrapped line | `.notdef` box visible. The space after it is missing and the line overflows — the layout defect below; the renderer drew the layout faithfully. |
| `screenshots/1-default-cube-label-occlusion-x4.png` | 4× crop | Label foreshortened with the face. Occlusion edges follow the cube silhouette. |
| `screenshots/2-large.png` (1920×1136, window maximum 960×540 pt) | Same scene, 16:9, after resize and motion | Layout, order and occlusion hold. Text is magnified about 1.9× per atlas texel. |
| `screenshots/2-large-latin-1to1.png`, `2-large-arabic-1to1.png` | 1:1 crops | Soft but clean bilinear magnification, no halos. Arabic joins correctly. |
| `screenshots/3-minimum.png` (174×80; content 174×16 px, macOS clamps the frame to 87×40 pt) and `3-minimum-x6.png` | Smallest supported size | Frames still present. Text is about 12× minified, beyond the 4-tap filter, so it reads as specks (expected and documented). No corruption. |
| `screenshots/gpu-test-hdr-graph-256x64.png` | Numeric GPU test output (1:1 pixel-perfect), not a window capture | Same strings as the test reference. |

Numeric checks on the native captures:
- The backdrop measures exactly (20, 21, 25).
- The most common bright value in the large Latin line is (243, 243, 243)
  (7,999 pixels). That is the documented `tone_map(1.0)` → sRGB 243 for white
  text radiance.

My judgement: correct canonical drawing under the stated contract, legible at
the startup and maximum sizes. The minimum size is unreadable by geometry, not
by a defect. Magnified text is soft. White on the light cube face has moderate
contrast; that is a content choice, since there is no authored color yet.

## Foundation defect found (nonvisual; for root)

`incant_text::TextEngine::shape` uses pinned cosmic-text 0.14.2. With `Wrap::Word`
or `Wrap::WordOrGlyph`, a line containing an RTL run followed by LTR text drops
the space after the RTL run and exceeds the declared width. Here is the CPU
layout (fonts: Noto Sans, Noto Sans Arabic, CJK fixture; 28 px, line height 36):

| String / width | Source spaces | Space glyphs | Lines (width) |
| --- | --- | --- | --- |
| `Wrapped ffi سلام 中文 ש and spaces` / 300 | 6 | 4 | `…中文 שand` (350.8 px > 300), `spaces` |
| same / 600 | 6 | 6 | one line, 454.4 |
| `Wrapped ffi سلام 中文 ש and` / 300 | 5 | 4 | one line, 350.8 > 300 |
| `Wrapped ffi 中文 x and spaces` / 300 | 5 | 4 | `…x and` (296.6), `spaces` |

The only dropped space that is expected is the one at the wrap point. The
renderer reproduces the layout exactly, as the brief asks, so the defect shows
in `1-default-mixed-wrapped-x4.png`.

Proposed narrow repair, owned by root:
1. Add an `incant_text/tests/shaping.rs` regression with the first row.
2. In `shape`, refuse or report any line whose `width` exceeds `style.width`
   for the wrapping modes, and any whitespace cluster that is missing other than
   a wrap-point trailing space.
3. Then fix or work around cosmic-text's bidi-run wrap. For example, re-run
   `layout_runs` with the paragraph split at the reported break, or take a
   reviewed cosmic-text update. Do not hand-adjust glyph positions.

## Known limitations

- **Filtering.** Coverage is rasterized at 1 texel per logical pixel with no
  mipmaps or signed distance field. Magnified text is soft, and minification
  beyond 4 taps per axis aliases. There is no device-pixel-ratio-aware
  rasterization or screen-space route.
- **Styling and placement.** There is no authored color, outline, shadow or
  style; all text is white. Text is two-sided, so it is mirrored from behind.
  There is no clip rectangle. Transparent model materials do not occlude text,
  and text coplanar with opaque geometry can z-fight.
- **Blending and draw cost.** Coverage blends linearly with no gamma or
  contrast tuning. Each node is one draw, up to 1,024 per frame. No timing was
  measured, and no performance claim is made.
- **Editor coverage.** Editor Play was not exercised natively. The editor's
  non-Play authoring preview has no text input, so it draws no text there
  (unchanged).
- **Platforms and profiles.** Only macOS/Metal ran. The `wasm32` library check
  compiles, but Vulkan, DX12 and WebGPU were not executed. Numeric tests used
  the `ci` profile; the reviewed player is a shipping build. Neither is a device
  or frame-pacing measurement.
- **Pre-existing test failure.**
  `incant_game_player/tests/routing.rs::window_observation_refuses_before_startup_but_headless_keeps_observation_route`
  fails with `--features native-window` because of its headless half:
  `main.rs` reads the scene before the observation plan, so a missing scene
  gives `NotFound`. This packet does not touch `main.rs`, `observe.rs` or
  `session.rs`. The test is gated on that feature, which the foundation's CPU
  run did not enable. The window half (usage text) passes.
- **Stale status.** `STATUS.md`'s "Native text … explicitly refuses Text before
  submission" sentence is now stale. I left `STATUS.md` to root.

## Remaining Game UI work

- Flexbox/widget layout and screen-space (HUD) anchoring with device pixel ratio.
- Text styling: color, outline/shadow, clipping and scrolling.
- Editable text with IME and the on-screen keyboard.
- Input focus and gamepad navigation.
- Emoji and color fonts.
- Runtime localization driving text updates.
- Reference-phone memory and performance measurements.
- The Phase 1 Game UI gate itself, including pseudo-localization, one RTL and
  one CJK locale in Core Sample on the devices.

## Process, disk and grant release

- **Owned processes, all finished or reaped.**
  - Cargo builds, tests and clippy runs.
  - `incant_headless`: fixture `init`/`import`, two `rpc` sessions, and one
    CPU `play.run` RPC check of the fixture.
  - The shipping exporter, which deleted its private build storage.
- **Window launches** (`evidence/processes.txt`):
  - Attempt 1795 failed before exec (`nohup` was not on the restricted PATH),
    so no player ran.
  - Subshell 2277 started player 2282. The helper had recorded the subshell's
    PID, found no window for it and exited before handing the front back, so
    the game window kept focus for about 45 s after launch. When I noticed, I
    returned the front to the previous app (Claude, PID 2095). `launch.sh` now
    uses `exec`, so the recorded PID is the player's.
  - Four bursts (default, large, minimum, restore) each started after at least
    15 s of user inactivity (waited 0, 20, 0 and 8 s) and handed the previous
    front app and pointer back. No keyboard or pointer events were posted to any
    app; window moves and resizes used AX on the player's own window only.
  - The player was closed with its own close button over AX and exited normally
    with the report above. Subshell 2277 exited too.
- **Monitors.** Three of my wait monitors matched their own command line
  through `pgrep -f`; I stopped them. They had no other effect.
- **Temporary files.** `crates/incant_text/tests/scratch_0054.rs` was created
  twice to dump the layout for the defect report, then deleted (never
  committed). A root `node_modules` symlink to the parent checkout (identical
  `package-lock.json`, sha256 `cbea068b…`) let the TypeScript cook run; it was
  used read-only and has been removed.
- **Disk.** The data volume hit 100% (135 MiB free) while other agents' target
  directories under `/tmp` held about 200 GB. I deleted only stale outputs
  inside my granted `/tmp/incant-agent-native-target`, about 8 GB:
  - `debug/`, `wasm32-unknown-unknown/` and `ci/examples/`
  - 481 extensionless executables in `ci/deps` older than this session

  Other agents' directories were not touched. Free space later recovered to
  438 GiB through others' cleanup.
- **Release.** Native/GPU use was released at 2026-10-12T03:19:00Z
  (`evidence/release.json`); after the marker, `launch.sh` and `cap.sh` refuse
  (verified). `CARGO_TARGET_DIR=/tmp/incant-agent-native-target` is released
  with no owned cargo process; its 15 GB contents stay as a warm cache.
- **Local only, not committed.** `/tmp/incant0054` (316 MB): the exported
  package with player and dSYM, the authoring directory with its cooked caches,
  logs and the `winctl` binary.
- **Root files.** None are left uncommitted. `brief.md` and `work.json` came
  in Astra's commit `122aefb` and are unchanged.
