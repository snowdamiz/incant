# 0074 native late-frame execution on `077f7c39` (stage `metalfx-late`)

## Verdicts

The two verdicts are kept separate on purpose. Neither is inferred from the
selections or the contracts.

**Numerical and contract verdict: PASS.**

- **Cases:** the listing (which showed only `late_motion` and `late_stationary`)
  and both cases ran once, serially, in plan order. Both exited 0, and all 14
  required report checks are true.
- **Budget:** exactly the plan was used: 126 world frames, 0 UI submissions and 15
  PNGs at 960×540, against caps of 160 and 16. Nothing was retried.
- **Selection:** MetalFX was actually selected in every native frame at 16/16,
  12/16 and 8/16, with no fallback.
- **Releases and analyzer:** the GPU and the target were released cleanly.
  `analyze.py check` passed, and `publish` wrote `visual-screenshots-late-001`.

**Appearance verdict (my own inspection of all 15 PNGs, plus derived crops):**

- **Interiors converge.**
  - At 75% and 50%, shading and flat faces are clean by frame 8 and stay clean at
    frame 16.
  - Native interiors match the scale-1 native frame 16 within 0.07 (75%) and 0.09
    (50%) mean codes.
  - Under modest motion there is no lag and no trail behind the mover.
- **The 50% reset striation does not persist.**
  - It reproduces in the reset frame (`late-stationary-native8-f01.png`).
  - It is gone by frame 8 and stays at the converged level at frame 16.
  - The same column structure is present in single, unaccumulated renders of that
    face: the off reference, the off 8/16 frame and current-only.
  - It is therefore a property of reset-frame input, not a lasting MetalFX
    artifact.
- **Silhouettes and thin features are not reconstructed at 75% or 50%:**
  - the horizon, cube and sphere outlines, the receding posts and the one-texel
    MASK grille stay stepped at the input pitch (about 1.33 px and 2 px) and
    fragmented, from the reset frame through frame 16;
  - at 8/16 they are nearly identical to the same-run portable frame 16;
  - edge error against off barely moves with accumulation, while it falls from
    15.7 to 10.2 at 16/16.

  This is the shared current-coverage boundary policy, still an open engine
  defect. The scale correction does not fix it. Converged reduced-resolution
  quality is **not accepted** at silhouettes.
- **Minor motion observation:** a soft band trails the mover's moving shadow edge
  on the static floor (box mean 2.2 codes at 12/16 and 2.9 at 8/16 against the
  same-step off reference).

## Identities and verification before device work

| Item | Value |
|---|---|
| Model and effort | Claude Opus 5.5 (`claude-opus-5-5`), thinking effort max. Line 2 of root's ACP log `claude-0074-native-late-execution-acp.log` reads "Claude session verified: Opus 5.5; thinking: max" (log SHA-256 `859b5631…` when read). No subagents were used. |
| Brief | SHA-256 `1fdffe3a…`, root's uncommitted revision "Root current priority: execute prepared native late-frame review", left uncommitted |
| Root build receipt | `0074-late-build-root-20261013.json`, SHA-256 `949c2f9d…`: accepted CPU build and strict Clippy. Its source, build and lint record hashes (`19971e6b…`, `b97bf750…`, `aad8a442…`) match the files. |
| Grant | `0074-metalfx-late-001/review-ready.json`, SHA-256 `ac949af2…`. Stage `metalfx-late`; visual tree `f9699658…` equals HEAD's (`9998e60c`). Both prior pins verified (0072 coverage lease `ffdee29b…`, GPU release `730249ba…`). |
| Source | commit `077f7c39…`, tree `408c4bb2…`, freeze `62db43cb…` with exact coverage of 2,448 inputs plus the probe; features `cooked-temporal`, `native-metalfx` |
| Frozen binary | `bin/review/metalfx_visual_review`, SHA-256 `41929cf53ff1ad825582b99d77f09807163a64066f05d49ce1ca70326f803a15` (18,801,368 bytes, mode 0555); identical before and after every child |
| Lease | held by root under the same grant, with no processes on the target at acquire; released by me after the run |
| Predecessor | 0072 `gpu-released.json` `730249ba…`: released 10:04:19Z, 12 children, no owned processes, not stopped |
| Device | Apple M5 Pro (`Mac17,9`), Metal, integrated GPU; macOS 27.0.1 (26A434) |

I did no build, lint, recook, or source, helper, plan or grant edit.

## Run

Every command ran once, serially and in the foreground, as `python3 -I -B
handoffs/0074-metalfx-review/visual/tools/<tool>` (Homebrew Python 3.14.8).

| Child | PID | Start (UTC) | Duration | Exit | Debit (frames, UI, PNGs) |
|---|---|---|---|---|---|
| `gpu.py list review` | 54527 | 10:35:57 | 0.27 s | 0 | none (argv `--list --ignored late_`) |
| `review late_stationary` | 54564 | 10:36:03 | 3.36 s | 0 | 75, 0, 8 |
| `review late_motion` | 54592 | 10:36:11 | 2.10 s | 0 | 51, 0, 7 |

- **Child logs:** I read every stdout and stderr log in full. Each stderr holds only
  the required-backend line ("Metal; actual adapter … Apple M5 Pro"). Each stdout is
  `1 passed; 0 failed; 7 filtered out`. The scratch directories are empty.
- **`gpu.py release`, 10:36:19.8Z:**
  - 5 children, including root's 2 build rows; 3 of them GPU children.
  - 126 frames, 0 UI submissions and 15 PNGs.
  - `owned_processes: []`, `ledger_pids_alive: []`, `stopped: null`.
  - Latch `168cdac7…`.
- **`lease.py release`, 10:36:19.9Z:** no processes on the target. Lease
  `a9d3934f…`. No owned process remained afterwards.
- **`analyze.py check`:** pass with 0 problems and 31 bound files. Summary
  `analysis-2026-10-13T103626.261649+0000.json` (`5cb0cdaf…`).
- **`analyze.py publish`:** 15 PNGs in `visual-screenshots-late-001/`, manifest
  `8c846d19…`.

## Selections, resets and source-bound controls

- **`late_stationary`:**
  - One native owner ran at 16/16 (960×540), then 12/16 (720×405), then 8/16
    (480×270). It reported `MetalFx` in all 48 frames, with sequence 1–48 and phase 0
    at the start of each segment.
  - Inferred resets: `n16-01` (the owner's first frame), and `n12-01` and `n8-01`
    (content size changed). These are exactly the 002 `d12-7` and `d8-10`
    conditions.
- **`late_motion`:** fresh owners at 12/16 and 8/16 reported `MetalFx` in all 16
  frames each, with resets at `f01`.
- **Controls:** portable, current-only and off had no MetalFX owner.
- **Cross-run binding to the 002 records:**
  - The 16/16 off reference of view B is byte-identical to 002's `off-b` (RGBA
    `4f3f379e…`). The scene is therefore the 002 striation condition.
  - The 8/16 off reference is **not** identical to 002's `off-b-d8`: RGBA
    `1730684b…` against `ea8f6b94…`. The late case reached 8/16 through 12/16 on
    one plain scene; 002 went 16/16 → 8/16. I did not attribute this further
    without another run.

  The in-run 8/16 off control is consistent with the other controls of this run.
  It is an observation, not a contract.

## Stationary at 75% and 50% (view B)

Edge and interior mean max-channel sRGB difference against the 16/16 unjittered
off frame (29,053 edge pixels):

| Arm | f01 (reset) | f08 | f16 |
|---|---|---|---|
| native 16/16 | 15.65 / 0.128 | 10.71 / 0.117 | 10.19 / 0.115 |
| native 12/16 | 17.51 / 0.121 | 17.41 / 0.122 | 16.98 / 0.122 |
| native 8/16 | 23.06 / 0.145 | 22.02 / 0.129 | 21.73 / 0.129 |
| portable 8/16 (fresh owner) | 23.17 / 0.103 | 22.58 / 0.116 | 22.56 / 0.118 |

**Further comparisons:**
- **Against native 16/16 at f16:** 12/16 gives edges 11.88 and interior 0.070; 8/16
  gives edges 16.54 and interior 0.086.
- **Native against portable at 8/16, f16:** edges 2.98, interior 0.134.
- **Against the unreconstructed references at f16:**
  - native 12/16 against off 12/16: 17.15 / 0.182;
  - native 8/16 against off 8/16: 21.38 / 0.244;
  - portable 8/16 against off 8/16: 20.72 / 0.230.

**Temporal behaviour:**
- **Edge change between consecutive frames, frames 9–16:**
  - native 16/16: 8.7–12.4;
  - native 12/16: 9.9–13.8;
  - native 8/16: 12.3–19.3;
  - portable 8/16: 14.2–21.7.

  Silhouettes keep shimmering with the jitter phase at every scale. It is worse at
  8/16 and slightly lower for native than for portable.
- **Interiors:** native interiors settle to 0.011–0.014 mean frame-to-frame change
  by about frame 11 at every scale; portable 8/16 settles to 0.015–0.020.
- **Registration**, frames 9–16 against off:
  - slopes on the applied jitter: native 16/16 −0.042 / 0.005, native 12/16
    0.012 / 0.014, native 8/16 0.023 / 0.000, portable 8/16 0.013 / −0.002. All are
    effectively zero.
  - the shared y offset is −0.08 / −0.11 / −0.16 px for native and −0.15 px for
    portable. It appears in every arm, so it is not a MetalFX property.
- **Current-only band at 8/16:** 403,621 pixels are identical to the same-phase
  current-only control in all 8 compared frames. That includes 9,466 of 29,053
  reference-edge pixels (32.6%), which are policy pixels at 50%.

**Striation measure** on the 002 face (`STRIATION_RECT`, x 422–461, y 330–379):
mean absolute second difference of the column means of mean sRGB (column-mean
range in parentheses).

| Frame | Value |
|---|---|
| off 16/16 (unjittered, single frame) | 0.425 (184.91–185.81) |
| off 12/16 | 0.251 |
| off 8/16 | 0.541 (184.49–186.17) |
| current-only 8/16 f08 | 0.309 |
| **native 8/16 f01 (reset)** | **0.546** (184.35–186.27); 002 `d8-f10` measured 0.367 |
| native 8/16 f08 / f16 | **0.075 / 0.122** |
| native 12/16 f01 / f08 / f16 | 0.091 / 0.032 / 0.086 |
| native 16/16 f01 / f08 / f16 | 0.109 / 0.072 / 0.076 |
| portable 8/16 f16 | 0.051 |

**Reading:**
- The face's single-frame renders carry this column structure at every scale. At
  8/16 MetalFX passes it through on the reset frame (0.546, about the same as off
  8/16).
- At 16/16 and 12/16 its reset frames come out smoother than the raw renders.
- Accumulation removes the structure by frame 8 at 8/16. At 4× crop the frame 8 and
  frame 16 faces show no banding.

**Pixels** (crops in `inspection/`):
- 12/16 frames 1, 8 and 16 all show the same 1.33 px stair-stepping on the horizon
  and cube outlines, and the same fragmented grille.
- 8/16 frames 1, 8 and 16 show 2 px steps, a blobby grille and blocky posts, with no
  visible improvement from accumulation. The portable 8/16 frame 16 looks the same
  there.

## Modest motion at 75% and 50% (view A)

The mover advanced 0.0875 world units per frame while the camera panned 0.03 units
and −0.003 rad per frame. Edge and interior mean difference against the off
reference at the same path step:

| Arm | step 1 (reset) | step 8 | step 16 |
|---|---|---|---|
| native 12/16 | 20.84 / 0.128 | 21.46 / 0.176 | 21.32 / 0.209 |
| native 8/16 | 26.22 / 0.149 | 26.73 / 0.191 | 26.76 / 0.228 |
| portable 8/16 | 26.19 / 0.090 | 26.84 / 0.262 | 27.05 / 0.290 |

- **Interior error grows over the run in both arms.** Interior pixels over 2 codes:
  native 8/16 2,339 → 2,238 → 2,047; portable 2,295 → 4,420 → 5,142. Native softens
  less than portable.
- **Native against portable at 8/16, step 16:** edges 4.66, interior 0.306.
- **No lag.** Registration against the same-step reference:
  - native 12/16: (0.011, −0.085) at step 8 and (0.019, −0.087) at step 16;
  - native 8/16: (−0.009, −0.158) and (−0.007, −0.151).

  This is the same y offset as in the stationary case.
- **Pixels:**
  - The mover sits exactly where the same-step off frame puts it.
  - There is no trail or halo behind it.
  - Its vertical edges step at the input pitch.
  - The ×8 difference maps concentrate on silhouettes, the grille and shadow
    outlines, plus ±1-code arcs on the floor gradient.
  - The one clear local difference is a soft band along the lower edge of the
    mover's moving shadow on the static floor. That is mild shadow lag on a receiver
    whose own motion vectors are zero.

## Limitations

- **"Late" means 16 frames**, two jitter cycles. Longer horizons and other content
  are untested.
- **The reduced-resolution silhouette result is policy-bound.** It describes
  077f7c39's current-coverage boundary. A repair would need a new review; see
  `native-coverage-repair.md` and its addendum.
- **The off 8/16 mismatch with 002 is unexplained.** No run was authorized to
  attribute it.
- **Motion speeds are approximate** (about 5 px per frame for the mover); the run
  did not measure speed.
- **Narrow scope:** one device, offscreen at 960×540. There are no phone,
  performance, timing or memory claims, and no phase approval.

## Files and provenance

- **`visual-screenshots-late-001/`:** the 15 published PNGs and `manifest.json`
  (`8c846d19…`), bound to the grant, the visual tree and the passing summary.
- **`visual-evidence-late-001/manifest.json`** (`16492522…`) is the lossless
  provenance manifest:
  - **Pins:** grant, visual tree, source, freeze, binary, root build review, 0072
    release, ACP line, brief and publication manifest.
  - **21 records copied byte for byte** and re-read after writing:
    - analysis, grant, ledger, budget, GPU latch, lease and supervisor result;
    - the four build records;
    - the two reports and two event logs;
    - the child logs. The two libtest stdout logs are deterministic gzip with raw
      and stored hashes, round-trip verified.
  - **19 files by hash only:** checkout manifest, binary, build and lint logs, and
    root's supervisor script, log and streams.
  - **6 inspection aids:** the crop grids and difference maps under the evidence
    directory's `inspection/`. They were made with the stdlib helper retained beside
    the 002 aids (`png.py`, `c3adc34d…`). They are not renders and were not
    published.
- **Not touched:** all earlier packet records.
- **Transients:** `/tmp/0074-late-inspect/` held a copy of the helper and dry model
  outputs while I worked; it was removed.
