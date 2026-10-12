# 0055 mixed-direction wrap review — result

## Status

Verified locally. I found no new defect and changed no source. Root's reviewed
cosmic-text 0.14.2 correction wraps the mixed-direction text correctly, and the
canonical native text route draws that layout exactly. The route is
`SceneInput` → `NativeRenderHost` → `RetainedScene` → `GpuRenderConsumer`, the
same one accepted in 0054. I reviewed the pixels myself.

- **0054's failing line is fixed.** `Wrapped ffi سلام 中文 ש and spaces` now
  wraps inside 300 px and 180 px. Every interior space survives, including the
  one before `and` that 0054 lost. That holds in the CPU layout and in the
  actual window pixels.
- **The other content renders as laid out:**
  - the RTL-base mixed paragraphs;
  - the long token at width 60 in both Word and WordOrGlyph;
  - an oversized indivisible glyph at width 15;
  - ligatures, Arabic joins, CJK fallback and the `.notdef` Hebrew;
  - a rotated and scaled copy.
- **The window matches an independent CPU reference.** That reference is built
  only from the probe layout, its atlas and the published draw contract:
  - Every axis-aligned text is within 1 sRGB code value at 1:1, and within 3 for
    the rotated case and at 2.25× magnification.
  - All 250 non-whitespace glyphs are present.
  - About 4.1 M pixels outside the glyph quads have no stray ink.
  - The checker catches four injected defects (see *Checker self-test*).
- **Tests.** The focused shaping, text-input, GPU renderer and hosted text suites
  pass. The previously failing native-window routing test passes on this base.
- **Not claimed:**
  - any phone, shipping, performance or frame-pacing result;
  - full Unicode or bidi conformance;
  - widget layout, IME or the Game UI gate;
  - any phase gate.

## Priority revisions and director feedback

The current brief has no priority-revision section and no new director feedback.
There was no interrupted attempt: `/tmp/incant0055` did not exist and the
worktree was clean at start. I followed the brief as written.

## Model and effort

Claude Opus 5.5, model ID `claude-opus-5-5`, with thinking effort **max**. The
model ID comes from this session's system configuration and the effort from its
reasoning-effort setting. I cannot inspect the ACP transport myself, so Astra's
ACP verification of model and effort is the authoritative record.

## Native entry point exercised

I used the ordinary package/window route, `incant_game_player --package DATA
--window`. Editor Play was not launched.

1. **Player build.** `tools/package.py` built a **fresh `ci`-profile** player
   from this exact source. It follows the build step of `tools/export_native.py
   --mode window`:
   - package `incant_game_player`, features `project,native-window`, `--locked`,
     `-j2`;
   - the project cooked by the player's build script through
     `INCANT_NATIVE_PROJECT`.

   Only two things differ from the exporter, both granted: the `ci` profile
   (release with `opt-level = 1`) and the packet's
   `CARGO_TARGET_DIR=/tmp/incant-agent-native-target`.
2. **Packages.** It assembled one package per camera with the exporter's own
   `native_package.data_package`, then ran the exporter's CPU startup check
   (`--package data --ticks 1`) under an empty environment.
3. **Window runs.** Each player ran under `env -i` with only `PATH` (the player
   directory), `HOME` (an empty scratch directory) and `TMPDIR`.

This is not a shipping export: no LTO, symbol packaging or signing. The player
is a `ci`-profile binary, and nothing here is a performance measurement.

## Reproduction: strings, metrics and placement

All texts use 28 px with a 36 px line height and the exact 0054 chain, in order:
Noto Sans, Noto Sans Arabic, then the CJK fixture. There is no Hebrew face, so
Hebrew draws as `.notdef`. The cases are defined in
[`tools/cases.json`](tools/cases.json).

There are two 640 × 480 orthographic pages (`vertical_size` 4.8 m), so the
startup window maps one layout pixel to one frame pixel. Thin cube "guides"
frame each declared width box `[0, width]`, 1–3 px outside it, behind the text
plane.

| Key | Page | Content | Width | Wrap |
| --- | --- | --- | --- | --- |
| `mixed300` | wrap | `Wrapped ffi سلام 中文 ש and spaces` | 300 | word |
| `mixed180` | wrap | same | 180 | word |
| `rtl250` | wrap | `سلام hello world سلام hello again` (RTL base) | 250 | word |
| `rtl180` | wrap | `مرحبا بالعالم and hello سلام world` (RTL base) | 180 | word |
| `mixed300_turned` | wrap | same as `mixed300`, group rotated +5° about Z and scaled 1.15 | 300 | word |
| `narrow60_glyph` | narrow | `سلام ABCDEFGHIJKLMNOPQRSTUVWXYZ שלום xyz` (RTL base) | 60 | word_or_glyph |
| `narrow60_word` | narrow | same | 60 | word |
| `glyph15` | narrow | `中文 W` | 15 | word_or_glyph |
| `latin` | narrow | `Office affinity fluffy waffle` | 600 | word |
| `arabic` | narrow | `مرحبا بالعالم` | 340 | word |
| `cjk` | narrow | `中文 日本語` | 600 | word |
| `short300` | narrow | `Wrapped ffi سلام 中文 ש and` (0054 row 3) | 300 | word |

## Results

### CPU layout (public `TextEngine`, packaged chain)

The data comes from `tools/layout_probe.rs`, run by `tools/probe.sh`. Full dump:
[`evidence/layout.json`](evidence/layout.json); summary:
[`evidence/probe.txt`](evidence/probe.txt).

For all 12 texts, three regression-style checks pass:
- The non-whitespace clusters (cluster, font, glyph ID, bidi level) equal
  unwrapped shaping.
- No non-whitespace character is lost.
- No interior space is missing.

| Key | Lines (logical text, width px) | Spaces (source / glyphs) |
| --- | --- | --- |
| `mixed300` | `Wrapped ffi سلام 中文 ` 283.808 · `ש and spaces` 170.576 | 6 / 6 |
| `mixed180` | `Wrapped ffi ` 160.972 · `سلام 中文 ש` 139.636 · `and spaces` 146.496 | 6 / 5 (wrap point) |
| `rtl250` | `سلام hello world` 206.276 · `سلام hello again` 204.484 | 5 / 4 (wrap point) |
| `rtl180` | `مرحبا بالعالم ` 142.184 · `and hello` 121.996 · `سلام world` 134.512 | 5 / 4 (wrap point) |
| `narrow60_glyph` | 13 lines, widest 59.556: `سلام`·`ABC`·`DEF`·`GHIJ`·`KLM`·`NO`·`PQR`·`STU`·`VWX`·`YZ`·`שלו`·`ם`·`xyz` | 3 / 3 |
| `narrow60_word` | `سلام ` 59.556 · `ABC…XYZ` **461.888** · `שלום` **67.2** · `xyz` 42.252 | 3 / 1 (wrap points) |
| `glyph15` | `中` **28** · `文` **28** · `W` **26.04** | 1 / 0 (wrap point) |
| `latin` | one line 338.24; ligature clusters `ffi` `ffi` `fl` `ff` `ffl` (glyphs 1654, 1654, 1653, 1651, 1655) | 3 / 3 |
| `arabic` | one line 134.904, right-aligned to 340; contextual forms (for example ب 103 and 101, م 766 and 765) | 1 / 1 |
| `cjk` | one line 147.28; all five ideographs from the CJK fixture font | 1 / 1 |
| `short300` | `Wrapped ffi سلام 中文 ` 283.808 · `ש and` 74.312 | 5 / 5 |

Bold widths exceed the declared width, as the brief permits. In Word mode an
indivisible word stays whole. In WordOrGlyph a single glyph stays whole even
when it is wider than the line. Missing glyphs (`.notdef` clusters) number 12 in
total: one ש in each of the three main-string variants, and four in each narrow
case.

The reverse-direction glyph split is the path that root's `reverse-range`
change corrects. It breaks the LTR alphabet inside the RTL-base
`narrow60_glyph` paragraph as `ABC / DEF / GHIJ / KLM / NO / PQR / STU / VWX /
YZ`, and all 26 letters are kept in order.

For comparison, 0054 recorded `…中文 שand` (350.8 px > 300) with only 4 of 6
spaces, and `short300` as one line of 350.8 px > 300.

**The native player draws exactly this layout.** Its own startup report shows
12 texts, 3 fonts, 289 glyphs, 73 RTL glyphs and 12 missing clusters, the same
totals as the probe. Its 2048 × 2048 atlas plane hashes to `2420ed02…`, which is
byte-identical to the probe's atlas placed in the same plane. The packaged font
payloads are the three source TTFs by content hash.

### Native window captures (actual pixels)

I took four window-only captures (`screencapture -o -x -l <window>`, Display P3
tagged) within a six-capture budget; the crops are cut from them. The game
window shows no account or avatar, so nothing is redacted.

| Image | What it shows | My review |
| --- | --- | --- |
| [wrap-1-default.png](screenshots/wrap-1-default.png) | Wrap page at the startup size: 320 × 240 pt content, 1:1 layout pixels | Every line sits inside its guides. `▯ and spaces` keeps the space after the `.notdef`. The 180 px version wraps to three lines. RTL-base lines are right-aligned, with the first logical word (سلام / مرحبا) at the right. The turned copy keeps its two-line layout under rotation and scale. |
| [wrap-2-large.png](screenshots/wrap-2-large.png) | Wrap page at the maximum window (960 × 540 pt, 2.25 px per layout px) | Same layout and order. Bilinear magnification is soft but clean, with no seams or halos. |
| [narrow-1-default.png](screenshots/narrow-1-default.png) | Narrow page at the startup size | Thirteen-line WordOrGlyph column inside 60 px. The Word-mode alphabet overflows leftward from the RTL right edge, which is allowed. Each glyph at width 15 sits on its own line, wider than the box. |
| [narrow-2-large.png](screenshots/narrow-2-large.png) | Narrow page at the maximum window | Same, magnified 2.25×. |
| [wrap-1-default-mixed300-x3.png](screenshots/wrap-1-default-mixed300-x3.png) | 3× crop | `Wrapped ffi سلام 中文` / `▯ and spaces`. Interior spaces are visible. The `ffi` ligature, the joined سلام (lam-alef) and CJK fallback are intact. |
| [wrap-1-default-mixed180-x3.png](screenshots/wrap-1-default-mixed180-x3.png) | 3× crop | `Wrapped ffi` / `سلام 中文 ▯` / `and spaces` inside the 180 px guides. |
| [wrap-1-default-rtl-x2.png](screenshots/wrap-1-default-rtl-x2.png) | 2× crop | RTL-base paragraphs at 250 and 180, right-aligned, with Arabic joined. |
| [wrap-1-default-turned-x2.png](screenshots/wrap-1-default-turned-x2.png) | 2× crop | Rotated and scaled wrapped text with its rotated guides. |
| [narrow-1-default-glyph60-x2.png](screenshots/narrow-1-default-glyph60-x2.png) | 2× crop | WordOrGlyph at 60: every cluster present and in order. Hebrew draws as 3 + 1 `.notdef` boxes. |
| [narrow-1-default-word60-x2.png](screenshots/narrow-1-default-word60-x2.png) | 2× crop | Word at 60: the alphabet word (461.9 px) and the 4-glyph Hebrew word (67.2 px) are kept whole. |
| [narrow-1-default-context-x2.png](screenshots/narrow-1-default-context-x2.png) | 2× crop | Ligatures, joined Arabic, CJK with its space, and the `中`/`文`/`W` glyphs on width-15 lines. |
| [narrow-1-default-short300-x3.png](screenshots/narrow-1-default-short300-x3.png) | 3× crop | 0054's one-line overflow row now reads `Wrapped ffi سلام 中文` / `▯ and`. |

**My judgement.** The corrected wrapping is right in the actual window at both
sizes. Spaces, ligatures, Arabic joins, CJK fallback and transformed text are
intact. Text is legible at the startup size and the maximum size; it is softer
at the maximum, as documented in 0054.

The 0054 originals, such as
[1-default-mixed-wrapped-x4.png](../0054-native-text-draw/screenshots/1-default-mixed-wrapped-x4.png),
are untouched for before/after comparison.

### Pixel agreement with the independent CPU reference

`tools/pixels.py` builds the reference without any renderer code. It uses only
the probe's `layout.json` and `atlas.pgm`, `cases.json` and the published
contract:
- an orthographic camera giving content height / 4.8 px per unit;
- 100 layout px per local unit;
- quads at `position + origin − 1`;
- one bilinear tap clamped to the padded texel centres, as in `text.wgsl`;
- premultiplied white over the backdrop, then `srgb(mix(backdrop, 0.9, α))`, as
  in `output.wgsl`.

It masks the lit guide bars and ignores the rounded corners. Reports:
[`evidence/pixels-*.json`](evidence/). The startup-size reference and diff
images are [reference-wrap-1-default.png](evidence/reference-wrap-1-default.png),
[diff-wrap-1-default.png](evidence/diff-wrap-1-default.png),
[reference-narrow-1-default.png](evidence/reference-narrow-1-default.png) and
[diff-narrow-1-default.png](evidence/diff-narrow-1-default.png). These are CPU
renderings, not captures.

| Capture | Content px / px per unit | Max \|Δ\| (sRGB code values) | Pixels with \|Δ\| > 2 | Glyphs checked; ink ratio | Non-backdrop pixels outside glyph quads |
| --- | --- | --- | --- | --- | --- |
| wrap, startup | 640×480 / 100 | 1 for all axis-aligned texts; 3 for `mixed300_turned` | 2 (turned) | 123; 0.998–1.002 | 0 of 258,110 |
| wrap, maximum | 1920×1080 / 225 | 2; 3 for `rtl180` | 1 (`rtl180`) | 123; 0.998–1.001 | 0 of 1,827,821 |
| narrow, startup | 640×480 / 100 | 1 for every text | 0 | 127; 0.997–1.001 | 0 of 250,811 |
| narrow, maximum | 1920×1080 / 225 | 2; 3 for `narrow60_word` | 1 (`narrow60_word`) | 127; 0.997–1.000 | 0 of 1,791,026 |

The checked glyphs are 250 in total, which is all 289 shaped glyphs minus the 39
spaces. The ink ratio is captured over expected ink, with overlapping glyphs
shared the same way in both, so a present glyph reads 1 and a lost one reads 0.

Per line, measured from the captured ink:
- Every wrapped line's ink lies within its declared width, except the permitted
  oversized Word and WordOrGlyph cases above. For example, `mixed300` line 1 ink
  spans 0.5–275.5 and the RTL lines end at 248.5 of 250.
- The core of every interior space (the middle 40 % of its advance) contains no
  ink, matching the reference.

**Checker self-test** ([`evidence/checker-mutations.json`](evidence/checker-mutations.json),
`tools/check_mutations.py`). The unmutated startup wrap capture passes. All four
injected defects are caught:

| Mutation | Max \|Δ\| | Stray pixels | Lowest glyph ratio |
| --- | --- | --- | --- |
| Lost cluster (the `a` of "and" painted over) | 223 | — | 0.0 |
| 0054-style dropped interior space (the rest of the line shifted) | 223 | 253 | — |
| Wrong break (`ש` moved to line 1) | 222 | 168 | — |
| Unexpected extra glyph (`W` dropped from the reference) | — | 285 | — |

## Tests and checks run

All runs are local on macOS 26.6 (25G72), Apple M5 Pro (`Mac17,9`), Metal, with
rustc 1.99.0 (b940084d7) and `CARGO_TARGET_DIR=/tmp/incant-agent-native-target`.
Each cargo command used `./tools/cargo … --locked --offline -j2 --profile ci`.
GPU tests ran with `--include-ignored` under this packet's grant. Logs are in
`evidence/*.txt`.

| Command (abbreviated) | Result | Log |
| --- | --- | --- |
| `test -p incant_text -p incant_render_native -p incant_render_game -p incant_render_data` | 63 passed, 0 failed (same count as root's evidence) | [test-cpu-text.txt](evidence/test-cpu-text.txt) |
| `test -p incant_render -- --include-ignored --test-threads=1` | lib 72/72 and `model_gpu` 50/50 on Metal | [test-render-gpu.txt](evidence/test-render-gpu.txt) |
| `test -p incant_render_gpu_host -- --include-ignored` | 5/5 | [test-gpu-host.txt](evidence/test-gpu-host.txt) |
| `test -p incant_game_player --features native-window` | 31/31 unit and `routing.rs` 1/1 | [test-player-window.txt](evidence/test-player-window.txt) |
| `tools/probe.sh` (probe test only, copied in temporarily) | 1/1; `crates/` unchanged afterwards; output byte-identical after `rustfmt` | [probe.txt](evidence/probe.txt) |
| `tools/check_mutations.py` | checker discriminates, 4/4 | [checker-mutations.json](evidence/checker-mutations.json) |
| Vendored patch check (read-only against `~/.cargo/registry/src/*/cosmic-text-0.14.2`) | Only the 4 changed files differ; their `diff -u` equals `INCANT_PATCH.diff` | `evidence/identities.json` |

The `incant_text` total of 28 includes `bidi_wrap.rs` 2/2 and `shaping.rs`
16/16. The `incant_render` lib run includes the text draw, plan, retained and
native tests, among them
`drawn_text_matches_layout_baselines_bearings_bidi_order_and_missing_glyphs` and
`rotated_scaled_minified_and_magnified_text_matches_the_affine_reference`. The
`incant_render_gpu_host` run includes
`hosted_text_child_draws_with_models_script_and_audio_without_motion_uploads`.

The routing test that failed in 0054,
`window_observation_refuses_before_startup_but_headless_keeps_observation_route`,
now passes.

There were no existing failures to retain. Neither `fmt` nor `clippy` was
rerun: this packet changes no crate source. `layout_probe.rs` passes
`rustfmt --check` (edition 2024). The vendored crate's own upstream tests were
not run, matching root's evidence scope.

## Identities

All are recorded in [`evidence/identities.json`](evidence/identities.json).

- **Source.**
  - HEAD `90fbb18037bf6b9440a3fb43048b52a02c309f73`, tree
    `d80276eaf4fa61bd318fd0af932a22db18316709`. This equals work.json's source
    `69f740e0` plus only this packet's brief and work.json.
  - The tree was clean outside this packet for every build.
  - `Cargo.lock` `f6e26cf6…`, `shape.rs` `3f313fd3…`, `INCANT_PATCH.diff`
    `d7a692da…` and `bidi_wrap.rs` `11383a54…` all equal root's frozen evidence.
- **Authoring CLI.** `incant_headless`, `ci`,
  `a2970ed925e60c6e8b7f7ed2a30052a0f5ffe3fe6d4f3009a6c73fea172ce4e1`. It was
  used only for fixture `init`, `import` and RPC.
- **Player.** `incant_game_player`, `ci` profile, features
  `project,native-window`, unsigned, 29,803,888 bytes,
  `3b6e2294b4be42b38677d94debd1b96b75a2f114e18041f63dc2d5ceccc928f8`. One
  frozen copy is used in both packages; the launcher rechecks the hash.
- **Fixture.**
  - [`fixture/project.json`](fixture/project.json)
    `d8724010ef3dfaf4c800bc357b4d489a4c1116c5ab841afbcdaee1c476d9b6bf` and
    [`fixture/fixture.json`](fixture/fixture.json) `ede09190…`.
  - Authoring used only `init`, `import` (the three fonts and a generated unit
    cube) and one atomic `command.execute` transaction of 45 commands (2
    cameras, 1 light, 12 groups, 12 texts and 18 guides), then
    `project.save`.
  - The journal reopen matched, and no project JSON was edited. There is no
    script and no audio.
- **Packages.** Both share `world.scene` `339faab5…`. They differ only in
  `package.json` (camera `…20` or `…21`).
- **Fonts (content SHA-256).** Noto Sans `2ec33f84…`, Noto Sans Arabic
  `ee489b99…`, CJK fixture `3299d6ab…`. These are the same bytes as
  `crates/incant_text/tests/fonts`. No system font is involved.
- **Captures (SHA-256).**
  - `wrap-1-default` `636ed05a…`
  - `wrap-2-large` `e0451a89…`
  - `narrow-1-default` `4a9ced36…`
  - `narrow-2-large` `9ff93ac0…`
- **Player exit reports** ([player-reports.txt](evidence/player-reports.txt)).
  Both used `adapter` Apple M5 Pro, `backend` Metal, `on_screen` true and 0
  dropped wall ticks. Wrap ran 659 ticks/frames and narrow 631, each with 2
  surface reconfigurations.

## Observations for root

None of these is a wrap defect, and I changed nothing for them.

1. **Baselines shift with the fonts on a line.** In `mixed180` the baselines are
   28.864, 62.904 and 100.864, so the gaps are 34.04 and 37.96 px inside fixed
   36 px line boxes. Lines containing Arabic sit about 1.96 px higher (for
   example 26.904 compared with 28.864 for Latin-only lines). This is
   cosmic-text's per-line ascent/descent placement and is not introduced by the
   correction. It reads as slightly uneven spacing in mixed paragraphs, which a
   future Game UI style may want to normalize.
2. **RTL-base overflow extends to negative x.** For a right-aligned line that
   overflows, `ShapedText::size()` still reports `[461.888, 144]`, but the glyphs
   start at x = −401.888 (`narrow60_word`). That matches CSS-style RTL overflow,
   and the renderer draws it faithfully, since it has no clip. However, a future
   widget clip or hit-test must use glyph bounds, not `[0, size.x]`.
3. **Wrap-point spaces are kept inconsistently.** A wrap-point space is
   sometimes kept as a trailing glyph (`mixed180` line 1) and sometimes not
   emitted (`mixed180` line 2 → 3). Both are invisible, and the regression
   explicitly allows them. Every interior space is kept.

## Limitations

- **Native coverage.**
  - Only the package/window entry point ran, on macOS with Metal and a
    `ci`-profile player. Editor Play was not launched natively.
  - There is no phone, Vulkan, DX12 or WebGPU run, no shipping profile, and no
    timing or memory measurement.
  - The window minimum size was not captured; 0054 covers minification.
- **Scene.** The scene is static (no script motion). Retained motion and rebuild
  identity are covered by the GPU suites (`retained_motion_rebuild…`,
  `source_free_native_text_moves…`).
- **Pixel reference.** It models white text over the display backdrop with
  single-tap footprints, which is all this fixture uses. Guide bars and other
  geometry are masked rather than modelled. Display P3–tagged captures matched
  the sRGB-encoded reference without conversion: the backdrop reads exactly
  (20, 21, 25) at both sizes, and the lit guides read (153, 167, 190).
- **Text coverage.** The content is a focused set, not a Unicode or bidi
  conformance suite. Hebrew is intentionally uncovered and draws `.notdef`.
- **Out of scope.** Widget layout, styling, clipping, IME, focus and the Phase 1
  Game UI gate, as in 0054.

## Changed paths

Everything is under `handoffs/0055-bidi-wrap-review/`; there are no crate or
source changes.

- `result.md`
- `tools/`:
  - authoring and build: `cases.json`, `fixture.py`, `package.py`, `probe.sh`,
    `layout_probe.rs`;
  - window bursts: `launch.sh`, `burst.sh`, `cap.sh`, `review.sh`,
    `winctl.swift`;
  - analysis and evidence: `pngcrop.py`, `pixels.py`, `check_mutations.py`,
    `collect.py`.

  `winctl.swift` and `pngcrop.py` are adapted copies of the 0054 tools; 0054's
  released tools were not run.
- `fixture/{project.json,fixture.json}`
- `screenshots/`: 4 captures and 8 crops.
- `evidence/`: the layout probe output, atlas, pixel reports, reference and diff
  images, identities, test logs, player reports, process, burst and capture
  logs, and the release record.

`brief.md` and `work.json` are root files from Astra's commit `90fbb180`; they
are unchanged.

## Process, other lanes, disk and release

- **Owned processes, all finished.**
  - Cargo builds and tests, and `incant_headless` for fixture authoring.
  - The packaging script, including two CPU startup checks.
  - Two window players:
    - wrap, PID 39653, launched 04:01:48Z and closed 04:03:15Z;
    - narrow, PID 40669, launched 04:03:19Z and closed 04:04:06Z.

    Each recorded PID is the player itself: `exec` was used, and the command
    line was verified to start with the player path. Each exited normally
    through its own close button over AX and printed its report.
- **Focus.**
  - Every launch and burst started only after at least 15 s of user inactivity.
    Idle was sampled before four of the six steps, at 177–307 s.
  - The tao window took the front at each launch for about 2 s. The front and
    pointer went back to the previous app (Brave, PID 1866) immediately.
  - Four capture bursts took 5–7 s each and restored the front app and pointer.
    They wait with a `hold` that yields at once on real input; none triggered.
  - No keyboard or pointer events were posted. Moves, resizes and closes used AX
    on the player's own window only.
- **Other lanes, left alone.** A CPU-only lane in the `agent-budget-settings`
  worktree ran a shipping export probe in its own temporary target and a
  `cargo check -p incant_editor`. Both used
  `CARGO_TARGET_DIR=/tmp/incant-agent-font-target`, not mine.
- **Temporary files.** The probe copy `crates/incant_text/tests/zz_0055_layout_probe.rs`
  existed only during `probe.sh` runs. It was removed, and the script verified
  that `crates/` was unchanged. No `node_modules` link was needed, because the
  fixture has no TypeScript.
- **Disk.** 410 GiB was free at the end. Nothing was deleted anywhere.
- **Release.** Native/GPU use and the target cache were released at
  2026-10-12T04:08:09Z ([release.json](evidence/release.json)).
  - The `/tmp/incant0055/released` marker makes `launch.sh`, `burst.sh`,
    `cap.sh`, `probe.sh` and `package.py` refuse; I verified all five.
  - `CARGO_TARGET_DIR=/tmp/incant-agent-native-target` (about 15 GB) has no
    owned process and stays as a warm cache.
- **Local only, not committed.** `/tmp/incant0055` holds the packages and
  player, the authoring directory with its cooked caches and journals, raw logs,
  and scratch analysis.

## Open questions

- Should Game UI normalize per-line baselines in mixed-script paragraphs
  (observation 1)?
- Should widget layout clip or measure RTL overflow from glyph bounds
  (observation 2)?

Both belong to the future Game UI work, not this packet.
