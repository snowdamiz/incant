# 0069 GPU review, attempt 003 (g6 oracle): passed, presentation accepted

## Status

**Passed and released.**

- All 8 exact GPU regressions passed under root's corrected test oracle.
- Then all 10 capture processes rendered the planned 22 appearance frames. I
  inspected every frame personally.
- **Verdict:** the Core result and HUD presentation is accepted for this scoped
  review, with two minor follow-ups (below).
- The release is verified and no owned process remains.
- Nothing was retried, and no gate, source, layout or authority was changed.

## Model and effort

- **This session:** Claude Opus 5.5 (`claude-opus-5-5`), thinking effort **max**.
- **Root's ACP record** (`~/.codex/incant-evidence/claude-0069-g6-gpu-acp.log`):
  "Claude session verified: Opus 5.5; thinking: max". I checked it before
  running.

## Inputs and the two generations

| Input | Identity |
| --- | --- |
| Authority `0069-gpu-review-ready-003.json` | `5c242df5…`; every field matched |
| Wrapper `gpu_only_g6.py` | `22d2bf0c…` (commit `0a677941`); it imports the unchanged `gpu_only.py` (`0cf37992…`) |
| Brief | `0bdd7eba…`, bound by the authority |
| Root review `core0069-g6-wrapper-root-20261013.json` | `f49ae41c…` |
| Predecessor GPU release (`0070-effects-build-003`) | `e2aedc7e…` |
| Output `0069-gpu-003` | absent beforehand |

The read-only `gpu_only_g6.py check` passed with 0 children just before the run.

**Regressions ran on generation 6.**

- Source commit is `e193af4d`, root's oracle correction. Among the inventoried
  engine inputs, only the test file `crates/incant_render/src/ui_draw_tests.rs`
  (`cfg(test)`) differs from g5.
- Freeze `255c7f15…`, build `c9c12546…`, case manifest `fffb7857…`.
- Test binaries: `render-lib` is the g6 frozen copy (`ddf12438…`); `gpu-host-ui`
  is the unchanged g5 copy (`8b85ceb1…`).

**Captures used the unchanged generation-5 packages.**

- These are the four CI-profile locale packages and adapters at g5 freeze
  `acb68a62…`: separate CI builds, not shipping exports.
- The wrapper admitted them only after checking that the current tree equals the
  g5 source inventory, except the root-owned brief and that one test file. So the
  production halo, UI and sample code in these packages is today's source.
- Capture receipts therefore carry g5 and regression receipts carry g6. Nothing
  was relabelled.

**The shipping proof was reused, not re-run.** The numerical route, restart, HUD
state and PCM proof is the CPU lane's `shipping-002` evidence (`cpu-result.md`).

## Run

The command ran once, in the foreground, from 09:28:19Z to 09:29:06Z (47 s), and
exited with status 0. All 18 children exited 0 and were reaped. The longest took
3.2 s, and none timed out. Adapter: Apple M5 Pro, Metal.

| Regression | Coverage | Frames | Result and diagnostics |
| --- | --- | --- | --- |
| `render-scales` | existing | 4 | pass. 0 outside at 1x, 2x and 1.5x sRGB and 1x UNORM. vs exact worst 1 (UNORM: 0). |
| `render-nested` | existing | 2 | pass. 0 outside. |
| `render-halo-backdrops` | new | 2 | pass. White: 0 outside, text 16.85:1 against the darkest pixel. Black: 0 outside, text 18.08:1. |
| `render-over-world` | existing | 3 | pass. 0 outside. Text under the selection is still rejected (279 outside, by up to 47). |
| `render-retained` | existing | 6 | pass |
| `render-region` | existing | 1 | pass. 0 outside. |
| `host-composed` | existing | 17 | pass |
| `host-relayout` | existing | 4 | pass |

**Captures:** 10 processes, 22 frames, each at most 960×540.

- Every frame was one consumer accept.
- The four review frames are byte-identical to the matching final-run frames, so
  rendering is deterministic across processes.
- Both restart frames are byte-identical to their start frames: Start at 960×540
  and R at 640×480.
- That leaves 16 distinct images.

## Release

| Check | Result |
| --- | --- |
| `release.json` | `55ee54d0…`, latched at 09:29:06.341Z |
| Children | 18 started, 18 reaped |
| Frames debited | 22 appearance, 39 regression, within the 24 and 128 ceilings |
| Owned processes / target users | none / none |
| Ledger | 56 rows; its hash equals the release record; the latch binds `release.json` |
| After return | no 0069 process; no process has `native-gpu.lock` open |

## Appearance verdict

I inspected all 16 distinct frames at full size, and close-up crops of every
title and of the narrow top bar. **Accepted:** the presentation now does what the
brief asked, in a restrained, professional style.

**Result and restart are immediately clear.**

- "Samples secured" and "Surveyor down" are the dominant element, centred in the
  upper third above the surveyor and never covering it in a real locale.
- "Press R or Start to play again" sits directly beneath, naming both the
  keyboard and the gamepad action.
- On a win the objective clears and the pips show remaining health. On a loss the
  pips are all empty.
- After a restart the frame returns exactly to the opening view.

**The 0065 contrast failure is fixed.** The title and prompt now sit over exactly
the bright horizon haze, the derrick lattice, hoodoos and sunlit sand that failed
before, and they read clearly at every size. Close up, each glyph has crisp light
fill and a thin, sharp dark contour: about 2 px on the 32 px title and about 1 px
on 12–17 px text.

- It reads like a clean subtitle outline: no glow, smear or panel.
- Over the dark sky and canyon it nearly disappears, so the HUD stays light.
- The stats and legend stay readable over sky and canyon, so the contrast failure
  was not moved to them.
- No descender halo is clipped by the 1 px vertical label padding, so I keep the
  padding as is.

**Narrow, 420×320.**

- The top bar still fits on one row in English, with about 30 px between the
  objective and the legend: tight but clean.
- The result block clears the top bar and the surveyor.
- The 12 px legend is the smallest text: fine on desktop, small for a phone.

**Arabic.**

- The top bar mirrors correctly.
- The objective, samples, legend, title and prompt shape properly, with Latin "R"
  and "Start" ordered correctly inside RTL text.
- The contour follows joined script and the hamza without gaps.

**Simplified Chinese.**

- Every glyph renders, including the regenerated 按 and 或.
- The title is crisp. The contour slightly darkens the small counters of dense
  characters (器, 机) but costs no legibility at 32 px; the 17 px prompt is clear.

**Pseudo-locale.**

- At 640×480 the expanded strings fit.
- At 420×320 the top bar wraps the progress block onto a second row as designed,
  the title wraps to two lines, and the prompt crosses the surveyor. That is the
  stress case PRESENTATION.md already records. The text stays readable.

### Minor follow-ups (not blocking)

1. **Arabic health pips.** The `ar` health string reuses the English pip order
   (`● ● ○`). With no strong RTL character it lays out left-to-right, so in RTL
   the empty pip sits at the start of reading, on the far right.
   - This predates 0069.
   - A strings-only fix is to mirror the `ar` plural cases or prefix U+200F.
     `fontcheck` already ignores U+200F.
2. **Pseudo-locale at the narrowest size.** The expanded prompt reaches the
   surveyor. Real locales don't. A narrower title would need a layout change; I
   would not change it for the pseudo-locale alone.

## Screenshots

Lossless RGBA-to-PNG conversions: no resampling, crop or colour change. All 22
are committed under `gpu-attempt-003/frames/`; `frames-index.json` maps each to
its source RGBA and both hashes.

Compact selection:

| State | Frame |
| --- | --- |
| Playing, 960×540 | `frames/final-en-960x540--01-start.png` |
| Won over the horizon haze, 960×540 | `frames/final-en-960x540--02-won.png` |
| Lost after a Start restart (generation 2), 960×540 | `frames/final-en-960x540--04-lost-generation-2.png` |
| Narrow playing, 420×320 | `frames/final-en-420x320--01-start.png` |
| Narrow won, 420×320 | `frames/final-en-420x320--02-won.png` |
| Arabic won, 640×480 | `frames/final-ar-640x480--02-won.png` |
| Chinese lost, 640×480 | `frames/final-zh-Hans-640x480--02-lost.png` |
| Pseudo-locale lost, narrow, 420×320 | `frames/final-en-XA-420x320--01-lost.png` |

The 16 regression images are in `0069-gpu-003/regression/images-g6`.
- 14 of them are byte-identical to attempt 002's images, which I inspected then.
- I viewed the 2 new ones (the 1.5x sRGB and 1x UNORM panels): the halo label
  stays legible over the noisy test background.

## Retained evidence (this commit)

- `gpu-attempt-003/`: start and finish times, exit status (0), stdout (the
  regression rows), empty stderr, the 22 frame PNGs and `frames-index.json`.
- `gpu-provenance-003.json`: authority, wrapper, brief, review, ACP record,
  predecessor release, both generations, CPU packet, every run record, report,
  plan, log and frame hash, identical-frame groups, the 16 regression image hashes
  and the release checks.
- The full external record, including raw RGBA frames and reports, is in
  `/Users/andreyyurlov/.codex/incant-evidence/0069-gpu-003`.

Earlier reports, attempts and PNGs are unchanged, and the untracked CPU and
report paths are untouched.

## Limitations

- **Platform and builds:** Apple M5 Pro with Metal; desktop CI-profile packages
  and test binaries.
- **What the frames are:** single fresh frames at scale 1, not
  accumulated-history rendering.
- **What was not tested:** no native window, input device or audio.
- **No gate claims:** no phone, performance, shipping-profile or phase-gate
  claim. The verdict covers only the appearance of these captured states.
