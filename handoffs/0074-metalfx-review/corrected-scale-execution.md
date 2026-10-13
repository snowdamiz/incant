# 0074 corrected-scale execution — native MetalFX review 002 on `077f7c39`

## Verdicts

The two verdicts are kept separate on purpose.

**Numerical and contract verdict: PASS.**

- **Cases:** the listing child and all five cases ran once each, serially, in plan
  order. Every case exited 0, and all 35 required report checks are true.
- **Strict native steps:** MetalFX was actually selected at 12/16 (75%) and 8/16
  (50%). That covers the transitions d12 and d8 steps (720×405 and 480×270 to
  960×540) and the viewport framed-d8 step (384×216 to 768×432 inside the offset
  viewport). This is the first observation of native MetalFX upscaling in this
  packet.
- **Fallback contracts:** the planned FractionalViewport fallbacks still hold, at
  10/16 and for the fractional viewport. The camera cut, owner reuse, UI and
  motion checks all pass.
- **Budget:** exactly the plan was used: 108 world frames, 11 UI submissions (119
  of 128) and 18 PNGs (18 of 20) at 960×540. Nothing was retried.
- **Releases:** the GPU and the target were both released cleanly.
- **Analyzer:** `analyze.py check` passed with no problems. `publish` re-validated
  the evidence and wrote `visual-screenshots-002`.

**Appearance verdict (my own inspection of all 18 PNGs, plus derived crops):**

- **The full-resolution controls are unchanged.**
  - The off, portable and current-only PNGs are byte-identical to the first
    execution (2dc2db04, source 535003d7).
  - The 16/16 native PNGs differ only slightly (mean 0.02–0.08 codes, on edges),
    and they look the same.
  - The first execution's reading stands: no native-specific defect at scale 1, and
    native is not accepted as finished anti-aliasing.
- **Native upscaling at 75% and 50% is validated for selection and initial quality
  only.** Both retained captures are reset frames: `d12-7` and `d8-10` are inferred
  resets on the content-size change. This run does not judge converged quality.
  - **75% (`transition-d12-f7.png`):** close to the 16/16 reset frame.
    - Interiors and shading are smooth, and the image stays registered.
    - There is no ghost, offset or colour shift.
    - Silhouettes step at the input pitch, and the one-texel MASK grille breaks
      into fragments.
    - I found no native-specific artifact.
  - **50% (`transition-d8-f10.png`):**
    - Silhouettes show 2-px stair steps, the grille breaks into blobs and the
      distant posts fragment.
    - Interiors stay smooth except for one native-specific artifact: a faint
      vertical striation on a flat grey face, about ±1 code with a period of about
      5–6 px (measured below). It appears only in this native 8/16 reset frame. It
      is absent from the 16/16 and 12/16 native frames and from the portable 8/16
      frame.
    - From one reset frame it is unknown whether it persists.
  - **Native versus portable at the same scales:** run 1's portable frames at 12/16
    and 8/16 look nearly identical on silhouettes and the grille. Those pixels come
    from the jittered current frame under the engine's current-coverage boundary
    policy in both paths, so the scale correction cannot change them.
- **Still open:** the silhouette boundary policy is an engine defect. The
  scale-unit correction did not fix it, and I do not reinterpret it.
- **No source fix is indicated.** The striation evidence below goes to the
  late-frame follow-up, which needs a separate review.

## Identities and verification before device work

| Item | Value |
|---|---|
| Model and effort | Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max. Line 2 of root's ACP log `claude-0074-corrected-scale-gpu-acp.log` reads "Claude session verified: Opus 5.5; thinking: max" (log SHA-256 `910322bc…` when read). No subagents were used. |
| Brief | SHA-256 `4044592f…`, root's uncommitted revision, left uncommitted; unchanged at the end of the session |
| Grant (authority) | `0074-metalfx-visual-002/review-ready.json`, SHA-256 `fe6bfa07dd2cfc3591d2ea0b81baf8e998fc826306b89f88064fb226cc9090e6`, equal to the brief's authority SHA. Visual tree `0bb15f5c…` equals HEAD's (`442be465`) |
| Root build review | `0074-corrected-scale-build-root-20261013.json`, SHA-256 `9cbed8d0…`, status accepted, no findings. All 5 records it pins verified unchanged (grant, supervisor result, review/lint/checkout records). |
| Source | commit `077f7c39c0f4122aae36ea3a88537be006722822`, tree `408c4bb2…`, freeze `62db43cb…`; 18,088 exported files; jitter table unchanged |
| Features | grant: `cooked-temporal`, `native-metalfx`; built library: all `cooked-*`, `native-metalfx`, `default`, `physics-deterministic` |
| Frozen binary | `bin/review/metalfx_visual_review`, SHA-256 `059d28ac7dbefd8d1b761da2a03b607957b767e7545e8ed1cfdee5e659085376` (18,729,976 bytes, mode 0555, freshly compiled); identical before and after every child |
| Root build and lint | ledger rows `review` (pid 34191) and `lint` (pid 35014), both exit 0 with empty process groups; supervisor result exit 0 for lease acquire, checkout, toolchain, review and lint |
| Lease | held by root's acquisition under the same grant, not released before the run, no processes on the target at acquire |
| Predecessor | 0069 `0069-gpu-003/release.json`, SHA-256 `55ee54d0…` (the grant's pin); released 09:29:06Z, 18 children, no owned processes, no target users |
| Device | Apple M5 Pro (`Mac17,9`), Metal, integrated GPU; macOS 27.0.1 (26A434) |
| Global GPU lock | free before the run. My check took the flock non-blocking and released it immediately, writing nothing. No other owned process was running. |

I did not acquire the lease, build, lint or recook anything. I did not touch the
fixture, source, helpers or grant.

## Run

Every command ran once, serially and in the foreground, as `python3 -I -B
handoffs/0074-metalfx-review/visual/tools/<tool>` (Homebrew Python 3.14.8).

| Child | PID | Start (UTC) | Duration | Exit | Debit (frames, UI, PNGs) |
|---|---|---|---|---|---|
| `gpu.py list review` | 36316 | 09:42:11 | 0.28 s | 0 | none (listing; all 5 cases match the plan) |
| `review stationary` | 36349 | 09:42:19 | 2.82 s | 0 | 33, 0, 4 |
| `review motion` | 36368 | 09:42:26 | 2.43 s | 0 | 34, 0, 5 |
| `review excluded` | 36426 | 09:42:34 | 1.20 s | 0 | 11, 11, 3 |
| `review transitions` | 36504 | 09:42:40 | 1.04 s | 0 | 18, 0, 4 |
| `review viewport` | 36544 | 09:42:47 | 1.33 s | 0 | 12, 0, 2 |

- **Case order:** the brief says "in their plan order" but lists stationary, motion,
  transitions, viewport, excluded. The committed plan, `gpu.py` (it refuses
  out-of-order cases) and the analyzer all require stationary, motion, excluded,
  transitions, viewport. I ran that order, so nothing was refused.
- **Child logs:** I read every stdout and stderr log in full. Each stderr holds only
  the required-backend line ("Metal; actual adapter … Apple M5 Pro"). Each stdout is
  libtest `1 passed; 0 failed`.
- **`gpu.py release`, 09:42:53Z:**
  - 8 children, including root's 2 build rows; 6 of them GPU children.
  - Debited 108 frames, 11 UI submissions and 18 PNGs.
  - `owned_processes: []`, `ledger_pids_alive: []`, `stopped: null`.
  - Latch SHA-256 `81aa15a6…`.
- **`lease.py release`, 09:42:57Z:** no processes on the target; the warm cache was
  left intact. Lease SHA-256 `52218a7b…`. No owned process remained afterwards.
- **`analyze.py check`:** pass with 0 problems and 46 bound files. Summary
  `analysis-2026-10-13T094303.733143+0000.json`, SHA-256 `c29186f5…`.
- **`analyze.py publish`:** 18 PNGs in `visual-screenshots-002/`, manifest SHA-256
  `6bdea57b…`, bound to the grant, the visual tree and that summary.

## Actual native and fallback selections

| Case and step | Frames | Selection | Internal size |
|---|---|---|---|
| stationary, native arm | 16 | `MetalFx` every frame | 960×540 |
| motion, object and pan | 8 + 6 | `MetalFx` every frame | 960×540 |
| excluded | 8 | `MetalFx` every frame | 960×540 |
| transitions, view A / cut / view B | 4 + 2 | `MetalFx` | 960×540 |
| **transitions, 12/16** | 3 | **`MetalFx`** (was `Portable(Some(Scale))` at 535003d7) | 720×405 |
| **transitions, 8/16** | 3 | **`MetalFx`** (was `Portable(Some(Scale))`) | 480×270 |
| transitions, 10/16 | 2 | `Portable(Some(FractionalViewport))` | 600×338 |
| transitions, resumed 16/16 | 2 | `MetalFx` | 960×540 |
| viewport, integral, 16/16 | 6 | `MetalFx` | 960×540 attachment |
| **viewport, integral, 8/16** | 3 | **`MetalFx`** (was `Portable(Some(Scale))`) | 480×270 |
| viewport, fractional | 2 | `Portable(Some(FractionalViewport))` | 960×540 attachment |

The control arms (portable, current-only, off) had no MetalFX owner in any frame.
The selections match the preparation's models: the corrected model predicted
MetalFx at the three reduced steps, and the 535003d7 model had reproduced the first
run's fallbacks.

- **Motion history:**
  - View A is epoch 1. The cut starts epoch 2 at phase 0 with no history.
  - `has_previous` stays true through the DRS steps. That means portable output
    history continues, while native history resets by policy.
- **Native resets**, inferred from selection, `has_previous`, epoch and internal
  size, since no reset flag is exposed:
  - `a-1`: the owner's first native frame;
  - `cut-5`: no valid motion history;
  - `d12-7`: content size changed;
  - `d8-10`: content size changed;
  - `full-15`: the previous frame was portable and the content size changed.

  This matches 077f7c39's `owner.rs`: "Conservatively reset native history on
  content-size changes".

## Native upscale at 75% and 50%: initial quality

The review frames are the four native reset frames of the same static view B:
`cut-f5` (16/16), `d12-f7` (12/16), `d8-f10` (8/16) and `resume-f15` (16/16). I
compared them with run 1's portable `d12-f7` and `d8-f10` at 3× and 4×, using the
crops in `inspection/`.

- **Silhouettes:** horizon, posts, cube outlines and sphere rim are hard stair steps
  at the input pitch: 1 px at 16/16, about 1.33 px at 12/16 and 2 px at 8/16. Run 1's
  portable frames are nearly identical there. These pixels come from the jittered
  current frame under the current-coverage boundary policy (clear coverage and depth
  discontinuities), so MetalFX does not reconstruct them in either run.
- **One-texel MASK grille:** clean thin lines at 16/16. At 12/16 they become
  irregular fragments, and at 8/16 large blobs. Again this is nearly identical to
  the portable frames: alpha-tested mixed-depth footprints follow the same boundary
  policy.
- **Interiors and shading:** smooth at every scale, with no ghost, offset, colour
  shift or ringing. The single native-specific finding is a faint vertical striation
  on the white cube's flat grey left face in `d8-f10`:

  | Frame (region x 422–461, y 330–379) | Column-mean range | Mean \|second difference\| |
  |---|---|---|
  | native 16/16 reset `cut-f5` | 185.4–185.8 | 0.109 |
  | native 12/16 reset `d12-f7` | 185.4–185.9 | 0.109 |
  | **native 8/16 reset `d8-f10`** | **184.5–186.4** (≈ ±1 code, period ≈ 5–6 px) | **0.367** |
  | native 16/16 reset `resume-f15` | 185.4–185.9 | 0.101 |
  | run 1 portable 12/16 `d12-f7` | 185.3–185.8 | 0.122 |
  | run 1 portable 8/16 `d8-f10` | 185.2–185.7 | 0.104 |

  It is barely visible at 1× and visible at 4×. Whether it persists after the reset
  needs the late-frame follow-up.

**Numerical observations against the full-resolution off reference of view B.**
Run 1's portable values are in parentheses. That comparison is not like-for-like,
because portable kept output history across the DRS change while native reset.

| Observation | Edge mean | Interior mean |
|---|---|---|
| d12 first frame vs off | 17.68 (15.90) | 0.126 (0.075) |
| d12-7→8, d12-8→9 consecutive | 16.47, 10.72 (18.02, 12.95) | 0.062, 0.052 (0.032, 0.020) |
| d8 first frame vs off | 22.50 (21.06) | 0.141 (0.099) |
| d8 last frame vs off | 22.24 (21.83) | 0.139 (0.107) |
| d8 last vs unreconstructed 8/16 off | 21.61 (20.51) | 0.241 (0.226) |
| d8-10→11, d8-11→12 consecutive | 17.24, 20.64 (16.71, 21.36) | 0.076, 0.074 (0.019, 0.020) |
| viewport 8/16 last vs off, inside edges / rest | 22.59 (22.79) | 0.157 (0.115) |

- **Distance to off:** native reset frames sit slightly further from off than run
  1's history-carrying portable frames, as expected after a reset.
- **Interiors still settling:** native interiors keep changing frame to frame (about
  0.05–0.08 against portable's 0.02), consistent with history still accumulating
  after the reset.
- **Viewport:** at native 8/16 all 184,220 pixels outside the viewport are
  byte-identical to off, as at 16/16.

## Unchanged full-resolution controls

Run 2 numbers, with run 1 in parentheses:

- **Byte-identical PNGs (7):** `stationary-off`, `stationary-portable-f08`,
  `motion-off-f8`, `motion-portable-f8`, `motion-pan-off-f6`, `excluded-off` and
  `viewport-off`. Their report metrics are identical too. The non-native path
  reproduced exactly across the two sources.
- **Native PNGs differ slightly from run 1** (mean delta / pixels over 2 codes / max):

  | PNG | Mean delta | Pixels over 2 codes | Max |
  |---|---|---|---|
  | stationary f16 | 0.074 | 2,169 | 66 |
  | motion f8 | 0.079 | 2,711 | 48 |
  | pan f6 | 0.082 | 1,637 | 36 |
  | excluded f8 | 0.024 | 1,221 | 15 |
  | cut-f5 | 0.032 | 1,053 | 17 |
  | resume-f15 | 0.040 | 1,600 | 19 |
  | viewport f6 | 0.055 | 2,034 | 53 |

  - Difference maps put this on silhouettes, the grille and shadow outlines.
  - Even the history-free reset frame `cut-f5` differs, so MetalFX's scale-1 output
    itself changed. Two causes fit: the descriptor now declares input content
    scales [1, 3] instead of [1, 1], or vendor output is not bit-reproducible. A
    rerun would be needed to separate them, and none was authorized.
  - Visually the two runs are the same. At 3× the stationary grille in native f16
    matches run 1's, and is slightly softer than off and portable.
- **Stationary:**
  - Native f16 vs off: edge mean 13.24 (12.98).
  - Native vs portable at the same phase: 2.67 (2.56).
  - Native one cycle apart at the same phase: 0.80 (0.77).
  - The consecutive-change rhythm repeats with the jitter phase, as in run 1.
- **Motion:**
  - Native f8 vs off: edge mean 14.12 (13.94).
  - In the 7→8 trail mask: native 5.65 (5.30), portable 3.51, current-only 1.65.
  - In the older trail: native 1.89 (2.02), portable 2.25.
  - The pan f6 vs off: 18.50 (18.30).
  - No ghosting; the panned grille is rebuilt regularly.
- **Excluded:**
  - The UI rectangle is byte-identical to off in all 10 frames.
  - Native f8 vs off: edge mean 2.61 (2.46).
  - Sprites, world text and the glass "Ag" are unchanged.
  - Activity stays on the checker's texel edges and border.
- **Viewport:** registration slopes 0.040 / 0.056 (0.037 / 0.055); the edges and
  canvas are clean.

## Registration and the current-coverage boundary

- **Stationary registration:** one Lucas-Kanade step per frame against the off
  frame, fitted on the applied image shift.
  - Native slopes are 0.060 in x and 0.025 in y (0.058 / 0.023), with mean shift
    −0.007 / −0.117 px.
  - Portable (−0.030 / −0.016) and current-only (−0.031 / −0.003) are identical to
    run 1.
  - The jitter convention holds, so no sign change is warranted.
  - The shared y offset of about −0.1 px in every arm remains. It is not a MetalFX
    property.
- **Boundary band:** 400,262 pixels (399,598) are identical between native and
  current-only in all 8 compared frames, including 13,131 (13,443) reference-edge
  pixels. The later edge change outside the band is 6.27 (6.27).
- **Reading:** native silhouettes still take the jittered current frame, by policy.
  The visible aliasing and phase shimmer therefore remain an engine limitation in
  both arms. This correction does not fix it.

## Limitations

- **The 75% and 50% captures are early reset frames.** Each scale step has only 3
  frames, and both retained PNGs are the reset frame. The viewport case keeps no
  8/16 PNG, only numeric observations. This run validates actual native selection
  and initial quality, not converged native upscale quality. Late-frame capture is
  reserved for a separately reviewed follow-up. The plan's reserve is 9
  submissions and 2 PNGs; this run did not use it.
- **The 8/16 striation is a single-frame observation** on one flat face. Whether it
  persists or converges away, and whether it depends on content, is unknown.
- **The native-versus-portable comparison at 75%/50% crosses runs and states.** The
  portable frames come from run 1's 535003d7 binary and carried history. They are
  context, not a same-run control.
- **The run-to-run native difference at scale 1 is unattributed.** It could be the
  descriptor range change or vendor nondeterminism.
- **Reset facts are inferred**, since no reset flag is exposed.
- **Narrow scope:** one device, offscreen 960×540, cooked-temporal plus
  native-metalfx. There are no iOS, phone, performance, timing or memory claims,
  and no phase approval is requested or implied.

## Files and provenance

- **`visual-screenshots-002/`:** the 18 published PNGs and `manifest.json`
  (`6bdea57b…`).
- **`visual-evidence-002/manifest.json`** (`d693bd6c…`) is the lossless provenance
  manifest:
  - **Pins:** grant, visual tree, source commit/tree/freeze, frozen binary, root
    build review, 0069 release, ACP line, brief and publication manifest.
  - **33 records copied byte for byte** and re-read after writing:
    - analysis summary, grant, ledger, budget, GPU latch, lease and supervisor
      result;
    - the four build records;
    - the five reports and five event logs;
    - the child logs. The five libtest stdout logs are deterministic gzip,
      round-trip verified, with raw and stored hashes, because their final blank
      line fails `git diff --check`.
  - **18 files by hash only:** checkout manifest, frozen binary, build/lint logs,
    root's supervisor script and streams, packet lock.
  - **10 inspection aids:** the transitions crop grids, the 4× white-cube grid,
    three ×8 difference maps, the stationary grille strip and the stdlib helper
    `png.py` (`c3adc34d…`). They live in the evidence directory's `inspection/`.
    They are not renders and were not published.
- **Not touched:** all earlier packet records (`visual-screenshots/`,
  `visual-evidence/`, `visual-execution-result.md`, the preparation receipts) and the
  first evidence directory.
- **Transients:** `/tmp/0074-002-check-stdout.txt` and
  `/tmp/0074-002-publish-stdout.txt` held tool stdout and were deleted after reading.
  `/tmp/0074-002-inspect/` held the helper while I worked; it was copied to
  `inspection/` and deleted. The `lease.py release` stdout was discarded; the lease
  file is the record.
- **Not done:** no memory writes, credentials, accounts, push, PR, desktop capture,
  native window or audio. No children or background tasks remain.
