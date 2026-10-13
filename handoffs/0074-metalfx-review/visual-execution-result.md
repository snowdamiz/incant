# 0074 visual execution — cooked native MetalFX review on `535003d7`

## Verdicts

The two verdicts are kept separate on purpose.

**Numerical and contract verdict: PASS.**

- **Cases:** the listing child and all five cases ran once, serially and in plan
  order. Every case exited 0, and all 35 required report checks are true.
- **Budget:** exactly the plan was used: 108 world frames, 11 UI submissions
  (119 of 128) and 18 PNGs (18 of 20) at 960×540. Nothing was retried.
- **Releases:** the GPU and the target were both released cleanly.
- **Analyzer:** `analyze.py check` passed with no problems. `publish` then
  re-validated the evidence and copied the 18 PNGs.

**Appearance verdict (Claude's own inspection of all 18 retained images):**

- **Native MetalFX at content scale 1: no native-specific defect found.**
  - It stays registered to the unjittered reference, so there is no jitter-sign
    error.
  - Interiors and shadows are stable after convergence.
  - There is no disocclusion ghost behind the moving cube.
  - Overlays and the UI never enter MetalFX and stay unchanged.
  - The integral offset viewport lands exactly where the off reference puts it.
  - Resets after the camera cut and after portable fallback are clean.
  - It is at least as good as the same-source portable arm, and better on the
    one-texel MASK grille under camera motion.
- **Not accepted as finished temporal anti-aliasing.** In both the native and the
  portable arms, silhouettes against the sky and at depth discontinuities stay
  aliased and shimmer frame to frame. The engine's conservative current-coverage
  boundary takes those 2×2 footprints from the jittered current frame, so MetalFX
  never reconstructs them. This is engine policy, not vendor quality.
- **Native upscaling was not reviewed.** Every 12/16 and 8/16 step selected
  `Portable(Some(Scale))`, so MetalFX was only ever selected at scale 1 (see
  Limitations).
- **The portable upscale's known quality failure is retained, not relabelled.** On
  those fallback frames the portable path breaks up the grille. At 8/16 it also
  stair-steps silhouettes, blurs the image and makes shadows blocky.

## Identities and verification before device work

| Item | Value |
|---|---|
| Model and effort | Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max. Root's ACP log `claude-0074-cooked-execute-acp.log`, line 2: "Claude session verified: Opus 5.5; thinking: max" |
| Brief | SHA-256 `7a0b9162…` (root commit `0c145f08`) |
| Grant | `0074-metalfx-visual/review-ready.json`, SHA-256 `40e64a7b030de29ea4de428579595d1ec19cadc97a89eca7c60058e06403a99c`; visual tree `b2c3e14e…`, equal to HEAD's |
| Root build receipt | `metalfx0074-build-root-20261013.json`, SHA-256 `ba6f2a06…` (accepted; strict clippy exit 0) |
| Build record | `build/review.json`, SHA-256 `f88a64e4…`; source record `build/source.json`, `1d52c532…` |
| Source | commit `535003d766bcc1386fd2d2f454c83808a0fcc4c8`, tree `e98b2484…`, freeze `bd8295b0…`; exact coverage of 2448 build inputs plus the probe |
| Features | grant: `cooked-temporal`, `native-metalfx`; built library: all `cooked-*`, `native-metalfx`, `default`, `physics-deterministic` |
| Frozen binary | `bin/review/metalfx_visual_review`, SHA-256 `e00b947a53e6219182a89f4783c1edbf39664cd45230ff73ce56f1fc089fce4c` (18,729,912 bytes, mode 0555); identical before and after every child |
| Predecessor | 0072 `gpu-released.json`, SHA-256 `3c54be77…`; released 08:46:07Z; no owned processes, no live ledger PIDs, not stopped |
| Device | Apple M5 Pro (`Mac17,9`), Metal, integrated GPU; macOS 27.0.1 (26A434); rustc 1.99.0 |
| Global GPU lock | free before the run; no other renderer running |

Two other lanes were running at the same time, both CPU-only: a wasm conformance
`cargo test` (with its `animation_parity` test binary) and idle ACP sessions. I
recorded them and left them alone.

## Run

Every command ran serially in the foreground as
`python3 -I -B handoffs/0074-metalfx-review/visual/tools/<tool>`:

| Child | PID | Start (UTC) | Duration | Exit | Debit (frames, UI, PNGs) |
|---|---|---|---|---|---|
| `gpu.py list review` | 14124 | 08:51:58 | 0.22 s | 0 | none (listing) |
| `review stationary` | 14152 | 08:52:02 | 3.25 s | 0 | 33, 0, 4 |
| `review motion` | 14190 | 08:52:13 | 2.62 s | 0 | 34, 0, 5 |
| `review excluded` | 14207 | 08:52:20 | 1.28 s | 0 | 11, 11, 3 |
| `review transitions` | 14232 | 08:52:26 | 1.09 s | 0 | 18, 0, 4 |
| `review viewport` | 14253 | 08:52:31 | 1.39 s | 0 | 12, 0, 2 |

- **`gpu.py release`, 08:52:35Z:** 8 children (including root's 2 build rows), 6 of
  them GPU children. Debited 108 frames, 11 UI submissions and 18 PNGs.
  `owned_processes: []`, `ledger_pids_alive: []`, `stopped: null`. Latch SHA-256
  `9d90d3dd…`.
- **`lease.py release`, 08:52:39Z:** no processes on the target. Lease SHA-256
  `73233401…`.
- **`analyze.py check`:** pass with no problems. Summary
  `analysis-2026-10-13T085245.421082+0000.json`, SHA-256 `cb2b546e…`.
- **`analyze.py publish`:** 18 PNGs in `visual-screenshots/`. Manifest SHA-256
  `2f8d9664…`.

## Actual native and fallback selections

| Case and step | Frames | Selection | Internal size |
|---|---|---|---|
| stationary, native arm | 16 | `MetalFx` every frame | 960×540 |
| motion, object and pan | 8 + 6 | `MetalFx` every frame | 960×540 |
| excluded | 8 | `MetalFx` every frame | 960×540 |
| transitions, view A / cut / view B | 4 + 2 | `MetalFx` | 960×540 |
| transitions, 12/16 | 3 | `Portable(Some(Scale))` | 720×405 |
| transitions, 8/16 | 3 | `Portable(Some(Scale))` | 480×270 |
| transitions, 10/16 | 2 | `Portable(Some(FractionalViewport))` | 600×338 |
| transitions, resumed 16/16 | 2 | `MetalFx` | 960×540 |
| viewport, integral, 16/16 | 6 | `MetalFx` | 960×540 attachment |
| viewport, integral, 8/16 | 3 | `Portable(Some(Scale))` | 480×270 |
| viewport, fractional | 2 | `Portable(Some(FractionalViewport))` | 960×540 attachment |

The portable and current-only control arms had no MetalFX owner, and every frame
of theirs reported none.

**Motion history across transitions:**

- The cut started epoch 2 at phase 0 with no history.
- Across the DRS steps the motion history continued (same epoch, `has_previous`),
  as the source predicts for output-resolution history.

**Native resets**, inferred from existing observables only, since no reset flag is
exposed:
- `a-1`: the owner's first native frame;
- `cut-5`: no valid motion history;
- `full-15`: the previous frame was portable and the content size changed.

## Registration and the jitter convention

The check is one Lucas-Kanade step per frame against the unjittered off frame over
its edge mask, fitted against the image shift each jitter phase applied:

| Arm | Frames | Slope x | Slope y | Mean shift x | Mean shift y |
|---|---|---|---|---|---|
| native, stationary | 9–16 | 0.058 | 0.023 | −0.006 px | −0.120 px |
| portable, stationary | 1–8 | −0.030 | −0.016 | 0.004 px | −0.136 px |
| current-only, stationary | 1–8 | −0.031 | −0.003 | −0.000 px | −0.130 px |
| native, inside viewport | 1–6 | 0.037 | 0.055 | −0.013 px | −0.098 px |

- **Jitter convention:** all slopes are effectively zero, so every arm, MetalFX
  included, undoes the applied jitter. This confirms root's reading of Apple's
  convention numerically, and no sign change is warranted. In the pixels I saw no
  doubling, swimming or offset against the off frame.
- **Shared y offset:** every arm, the pure current-only resample included, sits
  about 0.10–0.14 px off in y against the off frame. It is common to all
  arms, so it is not a MetalFX property. It may be a bias of the estimator or a
  shared resample offset. I did not judge it a defect.

## The current-coverage boundary policy

Stationary case, native frames compared with same-phase current-only frames:

- **The band:** 399,598 pixels (77% of the frame) are identical between native and
  current-only in all 8 compared frames. That includes 13,443 of the 34,327
  reference-edge pixels. Flat interiors can coincide too, so the band is an upper
  bound on policy pixels.
- **Activity map** (native frames 9–16): after convergence, change concentrates on
  silhouettes and the grille. These are the horizon against the sky, the outlines
  of the posts, cubes, wall, sphere and ramp, and the one-texel MASK grille.
  Interiors and shadows are nearly still.
- **Converged consecutive change on edges:** 13,120–15,885 edge pixels per frame
  change (mean 9.8–14.6 codes). The portable arm is the same order, 13,422–16,790
  (mean 10.6–16.1). At the same phase one cycle apart, native changes only 3,623
  edge pixels (mean 0.77). So the activity is periodic with the jitter phase, not
  drift.
- **Reading:** native frames take silhouettes against clear coverage and across
  depth discontinuities from the jittered current frame, by design. They inherit
  the same aliasing and phase shimmer as the portable path. In the crops, the
  horizon and post silhouettes are equally stepped in off, native and portable.
  This is the main remaining appearance limitation, and it belongs to the engine,
  not the vendor.

## Same-source arm comparison

**Stationary (16/16):**
- Against off, the edge mean delta is native f16 12.98, portable f08 11.84 and
  current-only f08 11.62. Native against portable at the same phase is 2.56.
- They look visually equivalent. The grille in native is slightly softer, with a
  faint pale smear where the bars meet the ground at the card's lower edge.

**Motion (16/16):**
- The mask of pixels changed from off step 7 to 8 covers 3,250 px. Inside it,
  native f8 differs from off by a mean of 5.30, portable by 3.52 and current-only
  by 1.65.
- Difference maps place this on 1–2 px silhouette outlines. The vacated wall and
  ground region is clean in every arm, so this is not ghosting.
- In the older trail (changed only since step 6), native is slightly better than
  portable: 2.02 against 2.25.

**Camera pan (native only):**
- Under motion, native rebuilds a complete, regular grille, while off's single
  aliased frame breaks it up. There is no smearing along the motion.
- Native against off at f6 has an edge mean of 18.3, which reflects the edge
  treatment rather than misregistration.

**Excluded:**
- The UI rectangle is byte-identical to off in all 10 frames across both arms
  (maximum delta 0).
- Sprites, world text and the blended glass "Ag" match off at 2×, with no halos.
- Only the accumulating checker's texture edges and outer border show activity,
  converging from 7,049 to about 3,400 changed edge pixels.
- Native against portable at the same phase has an edge mean of 2.3–2.6.

**Transitions:**
- The cut frame shows no residue of view A.
- The 12/16 and 8/16 frames are portable, with the retained failure described in
  Verdicts. Against off at 8/16 the edge mean is 21.8, and against the
  unreconstructed 8/16 control it is 20.5.
- The resumed native frame shows no carried-over low-resolution residue, and the
  edge mean against off falls from 18.6 to 11.9 one frame later.

**Viewport:**
- All 184,220 pixels outside the viewport are byte-identical to off.
- The edges and corner are clean at 4×, with no bleed.
- Registration slopes are 0.037 in x and 0.055 in y.

## Limitations

- **MetalFX was only observed at content scale 1** (native-resolution
  anti-aliasing). On this device and source, the engine-admitted range includes
  1.0 and excludes 0.75 and 0.5. That is inferred from the selections; no range
  value is exposed.
  - The pinned header describes these properties as the "smallest scale factor
    … to generate output textures" and "minimum input content scale".
  - The device might report upscale factors (≥ 1) that the engine intersects as
    content scale (≤ 1) with [0.25, 1]. That would collapse the range to [1, 1],
    which fits what I observed, but it is unverified.
  - **Strictly necessary extra observation:** the raw
    `supportedInputContentMinScaleForDevice` /
    `supportedInputContentMaxScaleForDevice` values on this M5 Pro, before any
    native-upscaling review.
- **Reset facts are inferred** from selection changes, `has_previous`, epoch and
  internal size.
- **Narrow scope:** one device, offscreen 960×540, cooked-temporal plus
  native-metalfx features. There are no iOS, phone, performance, timing or memory
  claims, and no phase approval is requested or implied.
- **Aggregate observations only:** just 18 frames are retained. Per-frame
  attribution of edge activity to policy or vendor pixels would need re-rendering,
  which was not done.
- **No HDR-specific highlight fixture** is in this stage, so highlight
  fireflies were not specifically probed.
- **Inspection aids:** the crops and amplified difference maps are derived from the
  retained PNGs and live in the evidence directory's `inspection/`. They are not
  renders and were not published.

## Files

- **`visual-screenshots/`:** the 18 published PNGs and `manifest.json`, which binds
  the grant, the visual tree and the passing summary.
- **`visual-evidence/`:** 32 records copied unchanged, listed in `manifest.json`
  with source, SHA-256 and size:
  - the analysis summary, grant, ledger, budget, GPU release and lease;
  - the build source, review, lint and toolchain records;
  - the five reports and five event logs;
  - the child logs. The five case stdout logs are stored as deterministic gzip,
    because libtest's final blank line fails `git diff --check`; their raw hashes
    are kept.
- **Hash only:** the 3.1 MB checkout manifest, the frozen binary and root's build
  logs.
