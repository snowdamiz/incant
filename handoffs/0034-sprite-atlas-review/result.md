# 0034 cooked atlas review: identity, relocation, filtering and public workflows

The follow-up round comes first. Round one follows unchanged, from "Status" onward.

## Follow-up: skyline packer, import-time sprite checks and 64-texel spans

### Verdict

**Accepted, scoped to atlas-specific behaviour, and explicitly conditional on the
shared renderer's remaining limitations.** On the follow-up executable (source
`4194240`), every atlas-specific check passes. Every round-one atlas finding is fixed
except one narrow packing residual.

The shared limits recorded by 0032 round five still apply, and atlas frames inherit
them:
- one-axis ties above 128× fail;
- two-axis exact ties near frame edges, and in non-dyadic 96 × 96 views, fail on
  every build;
- 342 nearest 2048-square sprites exceed the geometry cap.

The camera/follower placement contract is under separate review. None of these is
atlas-specific, and none is fixed or approved here.

| Round-one finding | Follow-up result |
| --- | --- |
| Three frames rejected at 1204×1148 | They fit 1204×1148 under all three tested bounds |
| Page grew with `maximum_size` | Monotonic: 0 violations over all nested pairs of 69 bounds, for two frame sets |
| 130-frame page at a 2048×2048 bound: 1204×1865 | 1222×1104, 40% fewer texels |
| Import errors in Rust Debug form | Readable Display text, with separate dimension, texel and padding errors |
| Reimports that broke every render while `validate` passed | Rejected atomically at import, naming every affected sprite; `validate` reports errors and exits 1 |
| Missing-frame render error names no entity | Unchanged in the renderer, but import and `validate` now name the entity first |
| Raw atlas rectangles show unrelated content after repacks | Unchanged by design, now with a warning on every reimport and in `validate` |
| Linear frames differ by up to 2 levels across repacks | Unchanged; informational |
| Source-Y ties failing at the span cut at 36×, 40× and 56× | 0 failures out of 4,972 exact ties on the 64-texel build |

- **Rendering under the new packer is correct.**
  - The four cooked pages match their sources texel for texel: content, extrusion
    and empty space.
  - Relocation is real. 126 of 130 frames move for v2, all 130 for v3, and 95 of 129
    for a valid replacement.
  - Nearest atlas frames are byte-identical to the explicit-rectangle control in every
    camera, and identical before and after each repack. Only the resized ballast and
    the deliberate raw-rectangle sprite change, by design.
  - Strips, exact placements, pivots, nine-slices, sorting and fringe checks all
    pass, as do fresh-process repeats and source-free rendering.
- **The import and `validate` changes work as described.** All 13 rejections exit 1
  and leave the project and journal bytes, revision, history and every asset
  fingerprint unchanged. Binding rejections leave 2 orphan cache files each, which is
  allowed. Undo, Redo and a valid replacement restore exact fingerprints. A single
  watcher poll imported a valid badge and rejected a broken atlas.
- **Repeatability holds.** Repeated fresh processes match byte for byte. Against round
  one's frames, every nearest view is pixel-identical except 136 rolled-view
  knife-edge pixels; see below.

Remaining or new observations:
1. **Residual NoFit (low).** The 130-frame set is rejected at 1221×1104 and
   1221×1147. An engine-free proof shows a valid 1217×1104 layout exists. The message
   now correctly says the bounded heuristic could not pack, rather than that the
   frames do not fit.
2. **Missing-cache diagnostics are noisy (low).** One missing atlas cache yields 407
   identical errors, one per sprite.
3. **Shared, informational.** The 64-texel spans change 136 pixels in rolled views of
   long strips, compared with the 512-texel build. Every one lies within 0.0021 px of
   an exact sprite edge or texel boundary, and they are identical in atlas and
   control. No crack appears.
4. **Golden tests (informational).** PNG bytes differ between the two builds while
   decoded pixels are identical. Cross-build comparisons should decode pixels.

This verdict covers headless rendering on one Apple M5 Pro with Metal, at the tested
magnifications. It does not establish arbitrary zoom, portability, phone performance,
native editor presentation, hosted CI or any phase gate.

### Decisions acknowledged

- The new packet section supersedes the earlier execution and budget instructions.
  I ran a focused delta and reused round one's art, scene, cameras, control binding
  and frames. Round one's 229-frame runs were not repeated.
- **Binaries:**
  - only the active `incant_headless` (`734811cb…`) rendered in this round;
  - `incant_headless-pre-followup` (`b72389ee…`) was not run;
  - round one's preserved frames from it are the cross-build baseline;
  - every call was hash-checked before and after.
- I read the integrated 0032 round-five report. This acceptance stays scoped to atlas
  behaviour and conditional on its limits.
- **Not done here:** no Cargo, Rust, shader, source, schema, SDK, docs or `editor/ui`
  edit; no attempt to fix the renderer or the camera; no native-app automation,
  accounts, merging or publishing.
- Astra's updated `brief.md` stays uncommitted, as delivered. Round one's sections,
  evidence and screenshots are unchanged, and their hashes still verify.

### Model, effort and provenance

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`) at Max effort, in the
  resumed ACP runner session. The runner confirms both settings over ACP before
  sending the prompt (`configure_model_and_effort`); I cannot inspect that handshake
  from inside the session.
- **Worktree:** HEAD `08db97c`, whose tree is identical to `4194240`, the follow-up
  binary's source.

| Build | Source | sha256 |
| --- | --- | --- |
| Follow-up, `incant_headless` (37,859,296 bytes) | `4194240378971f03bd7f3d812b961ccc0d24eb4d`, tree `08acdd59…` | `734811cb6053e32cce1ccd0fbb98fb7a515106cf7a8442f93b0be297af065469` |
| Round one, `incant_headless-pre-followup`, not run | `3825439d9e1f746999487ada3bb866a443ca2683` | `b72389ee24276459938705850b5cfe3efe2ffb1b744ba94ecf1f5d2ba8cc27a2` |

All 505 engine calls of this round matched `binary.json` before and after.

### Packing (import only, no frames; `evidence/followup/packing/`)

- **Reproduction:** 1200×40, 1100×4 and 4×1100 with padding 2 now fit 1204×1148
  under bounds 1204×1148, 1204×1200 and 1204×2048. The regions are: wide at (2, 2),
  row at (2, 46), column at (1106, 46).
- **The 130-frame round-one set:**

  | maximum_size | Page | Texels |
  | --- | --- | --- |
  | 2048×2048 | 1222×1104 | 1,349,088 (round one: 1204×1865, 2,245,460) |
  | 1204×1170 and 1204×1148 | 1204×1148 | 1,382,192 |
  | 2048×1170, 1300×1200 and 1600×2600 | 1222×1104 | 1,349,088 |
  | 1204×1147 and 4096×1024 | rejected, genuinely impossible | – |

  At the 2048×2048 bound, the level-0 GPU copy (RGBA16Float) shrinks from about
  17.1 MiB to 10.3 MiB.
- **Monotonicity sweep:** widths 1204–4096 by heights 1024–3000, 69 valid bounds for
  each of the two sets.
  - For every nested pair, a fit at the smaller bound fits the larger one with no
    larger page: 0 violations.
  - Distinct pages: 1204×1148 and 1222×1104 for the 130 frames; 1204×1148 and
    1212×1104 for the three frames.
- **Residual:**
  - the 130-frame set is still rejected at 1221×1104 and 1221×1147;
  - `atlas_packing_probe.py --feasible` places all 130 padded frames, disjoint, inside
    1217×1104, by the engine's own rules;
  - the four-frame core of that layout packs correctly at 1217×1104 under every
    bound, so no smaller reproduction exists in my tests.

  Every other rejected bound in the sweep is infeasible.

### Cooked pages and relocation

| Version | Fingerprint | Page | Pad | Mips | KTX2 sha256 | Mismatches (content/extrusion/empty) |
| --- | --- | --- | --- | --- | --- | --- |
| v1 | `8079ce3724e4…` | 1222×1104 | 2 | 11 | `bf44f17af2a9…` | 0 / 0 / 0 |
| v2 (ballast 1240×150) | `dcaefe33ce20…` | 1262×1104 | 2 | 11 | `1e82e493de53…` | 0 / 0 / 0 |
| v3 (padding 5) | `04cd9527fe91…` | 1246×1110 | 5 | 11 | `748b6160874e…` | 0 / 0 / 0 |
| drop unused label (129 frames) | `bde672575ed7…` | 1246×1110 | 5 | 11 | `0bf07658d340…` | 0 / 0 / 0 |

A frame-free capture in a second project reproduces the v1, v2 and v3 fingerprints
exactly.

Packed origins (x, y):

| Frame | v1 | v2 | v3 |
| --- | --- | --- | --- |
| hstrip (1100×4) | 20, 77 | 147, 183 | 5, 55 |
| vmid (1×700, from source y 103) | 15, 2 | 15, 2 | 1240, 5 |
| card 7 | 348, 46 | 348, 156 | 368, 121 |
| panel (nine-slice) | 20, 46 | 20, 156 | 190, 106 |
| probe | 444, 46 | 444, 156 | 1194, 115 |
| orb | 156, 46 | 156, 156 | 1040, 115 |

- **v1 → v2:** 126 of 130 frames move. The ballast and the three vertical columns
  stay.
- **v1 → v3:** all 130 move.
- **Valid replacement:** dropping the unreferenced `label:PROBE 1X` moves 95 of the
  remaining 129.
- **Large origin:** v2 puts the 1100-texel horizontal strip at packed x = 147, and
  its frames equal v1's.

### Rendered invariance (byte comparison)

| Comparison | Identical frames | Differences |
| --- | --- | --- |
| v1 subset rendered again in fresh processes | 10 / 10 | none |
| Control (original images) against atlas v1 | 23 / 26 | only board L (linear): 1, 135 and 32 px, at most 1, 1 and 2 levels |
| v2 (relocation) against v1 | 20 / 26 | board L: 2, 407 and 87 px, at most 2 levels. Board B: only the ballast (2,283 px) and the raw rectangle (1,512 px) |
| Undo against v1; Redo against v2 | 10 / 10; 10 / 10 | none |
| v3 (repack) against v1 | 20 / 26 | board L: 2, 503 and 125 px, at most 2 levels. Board B: only the raw rectangle |
| After 13 rejected imports, against v3 | 6 / 6 | none |
| Valid replacement (95 frames moved) against v3 | 6 / 6 | none |
| Watcher batch against v3 | 5 / 6 | board O: only the two recoloured badge sprites, 64 px each |
| Badge reimported back, against v3 | 6 / 6 | none |
| Every source removed, against v3 | 26 / 26 | none |
| Round one's v1 (old build) against follow-up v1 | 0 / 26 bytes, 20 / 26 pixels | board L linear (at most 2 levels), plus rolled-view knife edges |
| Round one's control against follow-up control | 0 / 26 bytes, 23 / 26 pixels | rolled-view knife edges only |

- **Linear differences** lie inside linear sprites, as in round one.
- **Byte-only differences.** Every other cross-build difference is in PNG bytes only:
  same chunk layout, filters and zlib header, with a different deflate length.
- **Rolled-view knife edges** (`analysis-fu-1.json`, `round1_rolled_view_changes`).
  Every changed pixel was inverted through the camera:

  | View | Changed px | Within 0.01 px of a sprite edge | Within 0.01 px of a texel boundary | Largest distance |
  | --- | --- | --- | --- | --- |
  | W rolled 3° | 67 | 19 | 48 | 0.0019 px |
  | V rolled 3° | 68 | 25 | 43 | 0.0021 px |
  | B rolled 7° (minified ballast) | 1 | 0 | 1 | 0.0013 px |

  No background pixel appears inside a strip, nothing is unexplained, and the counts
  are identical for atlas and control. The new spans add internal vertices and move
  only ambiguous ties (figure `screenshots/followup/figures/knife-edges-control-rl-W-1280x288.png`).

### Decoded checks on the follow-up v1 frames

- **Strips:** 112 rows, 109,088 along-axis pixels, 0 mismatches against the
  documented nearest rule. This includes span cuts, flips, exact and near ties, and
  the 32× leading and trailing texels. Interior runs are exactly M px.
- **Exact predictions at 1×:** 152 sprites match with 0 mismatching pixels: board S
  84, B 9 (raw, stable and scoped card 7), T 50 (45 orientations and 5 pivots) and
  N 9 nine-slices.
- **Painter order:** six groups match exactly. The reversed-order controls mismatch
  232, 232, 232, 232, 50 and 232 px.
- **Fringes:** 0 green- or magenta-tinted pixels in the halo and orb regions.
- **Large-origin probe** (`evidence/followup/extra/`):
  - the page is still 7308×5, with w6 at (6266, 2);
  - all 9 rows decode exactly, including 8× windows on the cut at page x 6778;
  - the pixel-perfect frame equals the control;
  - only the 2.5× linear strip differs, by 1 level on 10 and 2 px.

### Import rejection, validate and watcher

All 13 rejections exit 1. In each, the project and journal bytes, the revision and
history (from `rpc project.read` / `history.read`) and every asset fingerprint are
unchanged. RPC reads were first confirmed not to rewrite either file.

| Rejected import | Message (first line, verbatim after `Error: `) | New cache files |
| --- | --- | --- |
| unfittable maximum | could not cook art/atlas.atlas.json: could not pack every frame within the requested maximum using the bounded unrotated skyline heuristic | 0 |
| 4096×2048 maximum | … atlas page exceeds the 4194304-texel limit | 0 |
| 8193×256 maximum | … atlas width and height must each be in 1..8192 | 0 |
| padding 0, padding 17 | … atlas padding must be in 1..16 | 0 |
| lower-case ULID | … atlas version must be 1 and atlas/frame IDs must be canonical ULIDs | 0 |
| rectangle past the image | … invalid asset: atlas source rectangle exceeds image dimensions | 0 |
| 257 frames | … atlas must contain 1..256 frames with nonempty bounded names and paths | 0 |
| 4,194,305 source texels | … asset exceeds atlas decoded source texels limit | 0 |
| atlas as linear | … invalid asset: sprite atlases require color interpretation | 0 |
| shrunken nine-slice frame | import rejected: 15 invalid sprite reference(s) | 2 |
| card B dropped | import rejected: 2 invalid sprite reference(s) | 2 |
| badge shrunk to 8×8 | import rejected: 2 invalid sprite reference(s) | 2 |
| badge as linear | import rejected: 2 invalid sprite reference(s) | 2 |

- **Binding rejections name every sprite.** Each line gives scene, entity, texture,
  frame and cause, for example "…frame 01JA34FR00000000000000000F: nine-slice borders
  [3, 6, 4, 8] exceed resolved frame [443, 46, 8, 8]" for each of the 15 nine-slice
  sprites. The other causes read "referenced frame does not exist in the cooked
  atlas", "source rectangle [0, 0, 16, 16] exceeds cooked texture [8, 8]" and
  "sprites require a cooked color texture". Full text: `evidence/followup/rejection-messages.json`.
- **Orphan cache files:** the 2 new files per binding rejection are the prepared page
  and its metadata. They are unreferenced, as the brief allows.
- **Reimport warnings:** every reimport reply carries a `raw_atlas_region` warning
  for `B:raw-card7` (`evidence/followup/import-diagnostics.json`).

`validate` results:

| Case | Exit | valid | Diagnostics |
| --- | --- | --- | --- |
| After scene creation, after rejections, after Undo, source-free | 0 | true | one `raw_atlas_region` warning, with scene, entity, texture and path `…/SpriteRenderer/region` |
| Frame ID authored on an ordinary texture (command accepted) | 1 | false | `not_atlas` error on `S:card0:1x`, with frame and path `…/region/frame`, plus the warning |
| Cooked atlas cache deleted, in a scratch copy | 1 | false | 407 `cache_unavailable` errors, "asset IO: No such file or directory (os error 2)" |

That frame-on-ordinary state still fails to render ("sprite atlas frame … is
unavailable in texture …"; no entity). Undo restores a valid project.

- **Watcher.** During one `watch-assets` run, the badge became a valid orange 16×16
  image and the atlas lost card B in the same poll.
  - The badge was imported: `changed`, fingerprint `58513fa5…`.
  - The atlas was rejected. Its diagnostic lists both card-B sprites, and the atlas
    keeps its v3 fingerprint.
  - Restoring the descriptor cleared the diagnostic, and the watcher exited 0.
  - Reimporting the original badge returns the v3 frames exactly.
- **Undo and Redo:**
  - the v2 reimport undoes to v1 and redoes to v2;
  - the valid replacement goes `bde67257…` → Undo `04cd9527…` → Redo `bde67257…` →
    Undo `04cd9527…`.
- **Source-free:** 11 files removed; 26 of 26 frames identical; `validate` passes.
  Reimport fails with "could not cook art/atlas.atlas.json: asset IO: No such file or
  directory (os error 2)".

### Source-Y span cuts on the 64-texel build (`evidence/followup/upper/`)

I re-ran round one's upper-span probe on the new build. It covers both board
positions, at exact ties over span texels 236 to about 548, in 1920 × 1080
pixel-perfect frames.

| Source axis, board centre | Failed exact ties at 36× / 40× / 48× / 56× | Round one |
| --- | --- | --- |
| x, origin | 0/324, 0/288, 0/320, 0/315 | 93, 278, 219, 269 |
| x, (500, 500) | 0/324, 0/288, 0/320, 0/315 | 53, 262, 198, 262 |
| y, both positions | 0/300, 0/297, 0/322, 0/320 | 124, 185, 36, 232 |

- Strip texels 512 to 526, where the cut failures were, now resolve correctly. There
  are no non-tie errors, and atlas and control are byte-identical in all four frames.
- This is the measured set only. It does not show correctness at other zooms or
  positions, and 0032 round five still finds failures above 128×.

### Appearance review (follow-up)

I viewed the frames and figures.
- **Board B over v1 to v3:**
  - the stable-ID and texture-scoped cards never change;
  - the raw pixel rectangle shows card 7, then night sky, then strip extrusion over
    empty page, as the limitation requires (`screenshots/followup/figures/board-b-v1-v2-v3-zoom3.png`).
- **The cooked pages are now compact:** 1222 × 1104 for v1, with the 1100-texel
  columns along one side (`screenshots/followup/figures/atlas-pages-v1-v2-v3-quarter.png`). My adversarial
  set still leaves most of the page empty, because the columns set its height.
- **The watcher batch** changed only the two ordinary badges on board O.
- **The decoded 40× source-Y cut** that showed red failing ties in round one is now a
  clean checker (`screenshots/followup/figures/decoded-upper-y-40x-cut-ties.png`).
- **The rolled-view knife edges** are isolated pixels along strip edges and texel
  boundaries, not lines or gaps.
- **Everything else** is pixel-identical to round one's reviewed frames, so round
  one's appearance notes stand.

### Requests for Astra (follow-up)

1. **Residual NoFit (low).**
   - Repro: import `evidence/art/atlas-v1.atlas.json` with `maximum_size` 1221×1104
     (as `atlas_packing_probe.py ART --sweep` does). The import is rejected, while
     `atlas_packing_probe.py ART OUT.json --feasible` proves a 1217×1104 layout.
   - Option: add widths built from the actual layout, such as each placed frame's
     right edge, to the candidate set. Otherwise document that sets near a tight
     bound can still be rejected.
2. **Missing-cache diagnostics (low).**
   - One missing atlas cache produces 407 identical `cache_unavailable` errors.
   - Option: report it once per texture, with the affected sprite count or list.
3. **No new renderer request.** Keep my source-Y cut case (strip texels 512 to 526,
   36×/40×/56×) in the shared regression; it passes now.

### Changed paths (follow-up)

All changes are inside `handoffs/0034-sprite-atlas-review/`. `brief.md` is Astra's and
stays uncommitted.

| Path | Change |
| --- | --- |
| `tools/atlas_followup.py` | new: the focused follow-up sequence, `--import-diagnostics`, `--full-messages` |
| `tools/atlas_followup_analyse.py` | new: follow-up checks, cross-build comparison, knife-edge classification |
| `tools/atlas_lookdev.py` | `--binary`/`--round` selectors with separate ledgers; keeps import diagnostics and longer stderr |
| `tools/atlas_analyse.py` | decoded checks factored into `decoded_checks`; follow-up descriptor lookup |
| `tools/atlas_packing_probe.py` | `--sweep`, `--residual`, `--feasible`, selectors |
| `tools/atlas_upper_probe.py`, `tools/atlas_extra.py` | selectors |
| `tools/atlas_figures.py` | state-name parameters, `--followup-round1`, knife-edge maps |
| `tools/atlas_evidence.py` | `--followup` assembly |
| `tools/test_atlas_tools.py` | 13 tests: camera inversion and selectors added |
| `evidence/followup/` | analysis, run records, scene, import and rejection records, packing/upper/extra records, budget and image hashes |
| `screenshots/followup/engine/` | 19 unedited engine frames |
| `screenshots/followup/figures/` | 8 derived figures |

- **Round one still reproduces.** Its analysis rebuilds `evidence/analysis-run-2.json`
  exactly with the updated tools, and its figures rebuild byte for byte.
- **Reproducing round one's renders** now needs `--binary pre-followup --round r1`,
  because the follow-up build rejects its s8 imports by design.

### Commands and results (follow-up)

From the repository root, `P=handoffs/0034-sprite-atlas-review/tools`,
`F=artifacts/0034-sprite-atlas-review/followup`, `R1=artifacts/0034-sprite-atlas-review/run-2`:

```sh
python3 -I $P/atlas_packing_probe.py --repro $F/packing-repro-1
python3 -I $P/atlas_packing_probe.py $R1/art $F/packing-probe-1
python3 -I $P/atlas_packing_probe.py $R1/art $F/packing-sweep-1 --sweep                # 139 imports, 0 violations
python3 -I $P/atlas_packing_probe.py --residual $F/packing-residual-1
python3 -I $P/atlas_packing_probe.py $R1/art $F/feasible-1217x1104-tool.json --feasible
python3 -I $P/atlas_followup.py $F/fu-1                                                # 269 calls, 184 frames
python3 -I $P/atlas_followup.py $F/import-diag-1 --import-diagnostics
python3 -I $P/atlas_followup.py $F/rejection-messages.json --full-messages $F/import-diag-1
python3 -I $P/atlas_followup_analyse.py $F/fu-1 $R1 $F/analysis/fu-1-final.json
python3 -I $P/atlas_upper_probe.py $F/upper-1
python3 -I $P/atlas_upper_probe.py $F/upper-2-swapped --swap
python3 -I $P/atlas_upper_probe.py $F/upper-1 --decode-only                            # and upper-2-swapped
python3 -I $P/atlas_upper_probe.py $F/analysis/upper-span-stats.json --stats $F/upper-1 $F/upper-2-swapped
python3 -I $P/atlas_extra.py $F/extra-1
python3 -I $P/atlas_analyse.py $F/extra-1 $F/analysis/extra-1.json --extra
python3 -I $P/atlas_figures.py $F/fu-1 $F/analysis/fu-1-final.json $F/figures-1 --upper $F/upper-2-swapped --followup-round1 $R1
python3 -I $P/atlas_evidence.py --followup
python3 -I $P/test_atlas_tools.py                                       # 13 tests OK
python3 -I handoffs/0032-sprite-rendering/tools/test_reference.py       # 19 OK
python3 -m unittest discover -s tools/tests                             # 25 OK
cd handoffs/0034-sprite-atlas-review && shasum -a 256 -c evidence/followup/screenshots.sha256   # 27 OK
cd handoffs/0034-sprite-atlas-review && shasum -a 256 -c evidence/screenshots.sha256            # 28 OK (round one)
```

The analysis that produced `fu-1-final.json` was run twice after its last change;
both outputs are identical. Not run: Cargo, Clippy, the Rust and GPU suites, the UI
suite (no UI file changed), PlaySession, native captures and hosted CI. The agent's
batch import path is account-gated and was not exercised; the watcher's multi-source
poll was.

### Budget (follow-up)

| Item | Frames | Raw |
| --- | --- | --- |
| `fu-1` focused workflow | 184 | 164.8 MiB |
| Upper-span probes, two positions | 8 | 63.3 MiB |
| Large-origin probe | 9 | 11.7 MiB |
| **Follow-up round total** | **201 of 600** | **239.8 of 512 MiB** |
| Round one (preserved, unchanged) | 744 | 810.1 MiB |
| Lifetime | 945 | 1,049.9 MiB |

- Each invocation wrote one frame, at most 7.9 MiB raw. Nothing was overwritten.
- The one expected failed screenshot wrote no frame.
- The ledgers are `evidence/followup/budget-ledger-followup.jsonl` and, for round one,
  `evidence/budget-ledger.jsonl`. Totals are in `evidence/followup/budget.json`.

### Limitations (follow-up)

- **Hardware:** one GPU and backend (Apple M5 Pro, Metal), headless only.
- **Coverage:** the upper-span and strip results cover only the listed magnifications
  and positions. The shared limits from 0032 round five were not re-measured here.
- **Packing:** results come from my adversarial frame set and grid of bounds. The
  packer remains a heuristic, not an optimal packer.
- **Tie tolerance:** the knife-edge classification uses a 0.01 px tie tolerance. All
  changed pixels fall within 0.0021 px.
- **Not covered:** perspective cameras, the native Inspector, phones, performance and
  the camera/follower contract. No phase gate is approved or claimed.

### Open questions (follow-up)

1. Should the packer's candidate widths include layout-derived widths, to close the
   1217-wide residual, or is the heuristic bound acceptable as documented?
2. Should missing-cache diagnostics be grouped per texture?

---

## Status

**Accepted, scoped, conditional on the shared wide-region review. I found no
atlas-specific rendering defect.** I reviewed actual cooked atlas frames through the
frozen headless binary on one Apple M5 Pro with Metal:

- **Stable frame identity holds.** Every sprite that references a frame ULID renders
  that frame. This holds through a relocation of 129 of 130 frames, a full repack of all
  130, Undo/Redo, a live watcher reimport and source removal. Lookup is scoped to the
  sprite's texture: a second atlas reusing two frame IDs shows its own content.
- **The cooked pages are exact.** Five cooked pages were decoded from KTX2 level 0.
  Every packed texel, every extruded edge texel and every empty texel matches the
  authored sources, with no overlap.
- **Nearest atlas frames render identically to their sources.** In every camera, they
  are byte-identical to a control project that draws the same entities from the
  original images with explicit rectangles. That covers pixel-perfect 1× and 3×, 2.5×
  orthographic with a sub-pixel offset, 7° and 3° rolls, 1.125× and 0.875×, and exact 3×.
- **Relocation and page-size changes leave nearest pixels untouched.** Only two things
  change by design: the resized ballast image, and one sprite I deliberately authored
  with a raw pixel rectangle.
- **Linear atlas frames are smooth and bleed-free, but not byte-stable.** Across
  repacks and against the control, a few hundred pixels differ by at most 2 levels.
- **The span cuts render cleanly.** 112 strip rows decode exactly against the
  documented nearest rule at 1× to 32×. They include frames packed at y = 1163 to
  1273, flips, exact and near ties, and the 32× leading texels that failed in 0032
  round three. Nine more rows decode exactly on a frame packed at x = 6266.

Findings for Astra, detailed under "Requests for Astra":

1. **Packing** (cook time, not rendering): the packer rejects a set that fits as "frames
   do not fit", and a larger `maximum_size` can yield a 60% larger page for the same frames.
2. **Diagnostics:** `import` prints Rust Debug text for failures. An accepted reimport
   can make every render of the project fail while `validate` reports valid. The
   missing-frame error names no entity.
3. **Authoring hazard:** an explicit pixel rectangle on an atlas texture silently shows
   unrelated page content after any repack.
4. **Linear repack stability:** at most 2 levels, as above. This is informational.
5. **Shared renderer, not atlas-specific:** atlas frames inherit 0032's tie limit above
   16× exactly (atlas = control byte for byte). On the source-Y axis I also measured
   failures at the span cut and the next texels after it, up to strip texel 526, where
   0032 reports none.

**Not claimed:** acceptance of the shared renderer, phase-gate approval, native editor
presentation, hosted CI, other GPUs, phones or performance. The pixel-camera near-tie
follower defect is not tested here and remains open.

## Director feedback acknowledged

The packet's status paragraph says: begin the independent atlas review on the frozen
executable now, keep overall acceptance conditional on 0032's shared review, use no
other binary, and run no Cargo. I applied it:
- **Binary:** only `artifacts/tools/incant_headless` ran, hash-checked before and after
  each scripted call. No other binary, live rebuild or Cargo was used.
- **Shared renderer:** I do not label it accepted. 0032's round-four result arrived on
  its branch during this session (`b9eca69`, "accepted, scoped, up to 16×"). I read it
  read-only and measured how atlas frames relate to it. Its acceptance and integration
  remain Astra's and the director's.
- **Scope:** no `editor/ui`, Rust, shader, SDK, transport, docs or account file changed.
  No merge or publish.
- **Interrupted attempt:** none. The worktree held only `brief.md` in this packet.

## Model, effort and ACP

- **Model:** Claude Opus 5.5, model ID `claude-opus-5-5`, as stated by the session.
- **Effort:** Max.
- **Transport:** the prompt arrived in the exact template of `tools/handoff/main.py`.
  That runner sends a prompt only after `configure_model_and_effort` confirms model
  "Opus 5.5" and effort "max" over ACP (`tools/acp/config.json`). The pinned adapter
  is `@agentclientprotocol/claude-agent-acp` 0.88.0 with `@anthropic-ai/claude-code`
  2.1.295.
- **What I cannot verify:** the ACP handshake itself, from inside the session. No model
  or effort substitution occurred.

## Provenance

| Item | Value |
| --- | --- |
| Worktree HEAD | `3825439d9e1f746999487ada3bb866a443ca2683`, the binary's source commit |
| Source tree (`binary.json`) | `28d089c1c8a4acf65bfe0c9798f0e2737ad4612c`; runtime sources match atlas merge `34d5044` |
| Binary | `artifacts/tools/incant_headless`, 38,108,864 bytes, sha256 `b72389ee24276459938705850b5cfe3efe2ffb1b744ba94ecf1f5d2ba8cc27a2` |
| Hash checks | Matched `binary.json` before and after every scripted call: 285 per final run, 1,005 in total |
| GPU and host | Apple M5 Pro (adapter reported by `screenshot`), macOS 26.6 (25G72), arm64 |
| Tooling | Python 3.14.8 standard library only; no Node, TypeScript or Cargo |

- **Earlier build hash in the docs.** `docs/spikes/sprite-atlases.md` cites headless
  `0257f761…` for Astra's probes. That is a different build, so I used only the frozen
  `b72389ee…`.
- **Unbracketed exploratory calls.** Ten import-only calls during design (smoke and
  layout probes) were not hash-bracketed individually. They fall between checked calls,
  and the binary file is read-only.

## What I built

All authoring went through `init`, `import`, `rpc command.execute` / `history.undo` /
`history.redo` / `project.save`, `watch-assets` and `validate`. No project or journal
file is ever written by my tools. Source art and descriptors are copied into the
project as authoring inputs.

### Original art (`tools/atlas_art.py`)

The new art is designed so that atlas failures are easy to see:
- **ID cards:** twelve 12×14 cards (0–9, A, B), each with a unique colour, inside an
  opaque magenta guard. A wrong or shifted frame shows a wrong digit or magenta.
- **Edge probe:** 11×9, with a different colour on each side and corner and an "F"
  glyph, so flips, turns and one-texel offsets are unambiguous.
- **Self-naming strips:** 1100-texel horizontal and vertical strips in which every texel
  names its index through (R, G), inside the display's identity range. The ramp phase
  puts the span cuts at texel 512 and 1024 mid-ramp, so a seam cannot hide in a colour
  wrap. They also supply 1-texel rows/columns and 700-texel sub-rectangles starting at
  source texel 100.
- **Halo:** a soft halo with garbage green RGB under zero alpha.
- **Ballast:** a night panorama in two sizes, 1200×40 and 1240×150, whose change
  relocates the other frames.
- **Labels:** 94 label words in 0032's 3×5 font. Labels are atlas frames too, so a wrong
  frame shows a wrong word.

Reused byte for byte, with provenance in `evidence/art/manifest.json`: 0032's
`sheet.png` (`7451ef95…`) and `badge.png` (`648441d4…`), plus its font and PNG writer
(`pngio.py` `5b73bef8…`, `sprite_art.py` `45aec9a2…`). The sheet supplies the Wisp
frames, the asymmetric dialog panel, the key, the 7×7 unique-texel ruler, the orb, the
glow, the crate, the tiles, the solid and the marker.

`evidence/art/` holds every source, descriptor and the manifest. The art regenerates byte
for byte; all three fresh runs produced identical art.

### Atlases

| Descriptor | Change | Frames |
| --- | --- | --- |
| `atlas-v1` | maximum 2048×2048, padding 2; 129 optional source rectangles; the ballast omits its rectangle (whole image) | 130 |
| v2 | only `ballast.png` grows to 1240×150; same descriptor and frame IDs | 130 |
| `atlas-v3` | padding 5 and maximum 1600×2600; v1 sources | 130 |
| `alt` | a second atlas reusing card 0's and card 1's frame IDs with swapped content | 2 |

Failing descriptors: `fail-nofit`, `fail-bounds-area` (4096×2048), `fail-bounds-axis`
(8193×256), `fail-padding0`, `fail-padding17`, `fail-identity` (lower-case ULID),
`fail-rect`, `fail-frames257` and `fail-texels` (4,194,305 decoded source texels).
`render-fail-shrink-panel` and `render-fail-drop-cardB` are accepted by import but leave
the scene unable to render. `limit-texels-exact` has exactly 4,194,304 decoded source
texels.

### Projects, boards and cameras (`tools/atlas_scene.py`, `tools/atlas_lookdev.py`)

- **Atlas project:** one transaction creates 445 entities: 411 sprites, 6 mirror parents
  and 28 cameras. The engine reports 0 diagnostic stand-in entities.
- **Control project:** the same 445 entity IDs and transforms, drawn from 9 ordinary
  textures with explicit rectangles.

Boards are at 16 px per metre:

| Board | Content |
| --- | --- |
| S | all 12 cards at 1× and 2×, Wisp frames, key, ruler 1× and 3×, crate, probe 1× and 3×, star, marker, 18-tile rows on grid and half-pixel placements |
| T | probe at 1× and 2× and the 7×7 ruler on tie (1×, 3×) and aligned placements, in nine orientations: none, flip X, flip Y, both, mirror parent, 90°, 180°, 270°, 90° + flip X; five pivots with origin markers |
| N | asymmetric nine-slice panel (borders 3, 6, 4, 8): native, 72×44 against a simple stretch, four compressed targets, flips, 180°, mirror parent, densities 8 and 32, 12° rotation, a 1-1-1-1 probe slice |
| O | layer, order, ID tie, Z ignored, interleaved atlas and ordinary textures, 50% overlaps on dark and light, nearest orb/glow/halo with transparent edges, a Wisp stack |
| L | linear: orb 4× on dark and light, halo 3× and 2.5×, glow, Wisp 2×, crate at 30°, nine-slice panel, probe 3× and flipped, card 7 at 2.5× and 1× |
| B | ballast; card 7 by raw v1 pixel rectangle, by stable ID, and through the alt atlas |
| W, V | 1× strips: aligned, tie, flips, ±1/256 px near ties, 700-texel sub-rectangles |
| X, Y | 44 magnified rows or columns each: 2× to 32× windows on the 512 cut, 1024-cut rows, 700-texel sub-rectangles, near ties at 8×, 32× leading and trailing edges |

Cameras:
- pixel-perfect at 320×180 (1×) and 960×540 (3×);
- ordinary orthographic at 2.5×, offset by (+4.15, −1.96) px;
- 7° rolled orthographic;
- exact 3× orthographic on boards S and N;
- for W and V: 1.125× and 0.875× with an offset, and a 3° roll.

States, in order, each rendered in fresh `screenshot` processes:

| State | What changes |
| --- | --- |
| s0 | v1, rendered twice |
| s1 | ballast image resized: relocation |
| s2 | Undo |
| s3 | Redo |
| s4 | the live watcher reverts only `ballast.png` |
| s5 | v3 descriptor |
| s6 | ten rejected replacements |
| s7 | the live watcher sees a failing descriptor, then the restored one |
| s8 | three accepted changes that cannot render, each undone |
| s9 | every source removed |

The whole sequence ran twice from scratch (run-2 and run-3).

## Results

All values come from `evidence/analysis-run-2.json` (cross-checked against run-3)
unless another file is named.

### Cooked pages: decoded from KTX2, independent of the renderer

| Version | Fingerprint | Page | Pad | Mips | KTX2 sha256 | Packed + extruded texels | Mismatches (content/extrusion/empty) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| v1 | `336afbb5b077…` | 1204×1865 | 2 | 11 | `97f9ed64b7ec…` | 121,548 (5.4%) | 0 / 0 / 0 |
| v2 | `cdf14e3ddd87…` | 1244×1975 | 2 | 11 | `d5f511c1db9a…` | 260,148 | 0 / 0 / 0 |
| v3 | `919685dfac12…` | 1210×2284 | 5 | 12 | `1ff4a458c88f…` | 195,696 | 0 / 0 / 0 |
| shrink panel | `02adc0713400…` | 1204×1865 | 2 | 11 | `1c18cde3704b…` | 120,908 | 0 / 0 / 0 |
| drop card B (129 frames) | `3e3763c27c49…` | 1204×1865 | 2 | 11 | `fb9489fe3871…` | 121,260 | 0 / 0 / 0 |

- **Coverage:** every frame keeps its authored size and name, and no extrusion
  rectangles overlap.
- **Format:** pages are `rgba8_srgb`. Every 8-bit texel in these sources survives the
  cook's decode-to-linear and re-encode exactly. Values absent from the sources are not
  tested.
- **Determinism:** both fresh runs produced identical fingerprints and identical bytes
  for all 7 cooked KTX2 files.
- **Negative controls** (`evidence/controls-run-2.json`): flipping one bit in a packed,
  an extruded or an empty texel each fails verification. The unmodified page passes.

### Stable identity and relocation

Packed origins, (x, y); every frame keeps its size:

| Frame (size) | v1 | v2 | v3 |
| --- | --- | --- | --- |
| hstrip (1100×4) | 2, 46 | 2, 156 | 5, 55 |
| hrow (1100×1) | 10, 54 | 10, 164 | 19, 69 |
| hmid (700×1, from source x 103) | 15, 59 | 15, 169 | 19, 80 |
| vstrip (4×1100) | 2, 54 | 2, 164 | 5, 69 |
| vcol (1×1100) | 10, 59 | 10, 169 | 5, 1179 |
| vmid (1×700, from source y 103) | 10, 1163 | 10, 1273 | 19, 91 |
| card 7 | 346, 82 | 343, 192 | 91, 115 |
| panel (nine-slice) | 15, 82 | 15, 192 | 30, 721 |
| probe | 442, 82 | 439, 192 | 91, 257 |
| 7×7 ruler | 457, 82 | 454, 192 | 74, 1107 |
| halo | 43, 82 | 43, 192 | 30, 755 |
| orb | 151, 82 | 151, 192 | 30, 1055 |

- **v1 → v2:** 129 of 130 frames move, some in both axes. Only the ballast at (2, 2)
  stays.
- **v1 → v3:** all 130 move.
- **Page size:** 1204×1865 → 1244×1975 → 1210×2284.
- **Scoped lookup:** frame IDs are scoped by texture. Through the alt atlas, card 0's ID
  shows card 1 and card 1's ID shows card 0, both exact.

### Rendered invariance (differential, byte comparison)

| Comparison | Identical frames | Differences |
| --- | --- | --- |
| v1 rendered twice in fresh processes | 34 / 34 | none |
| Control (original images) against atlas v1 | 30 / 34 | only board L (linear), in 4 views: 1, 255, 46 and 9 px, at most 1, 1, 2 and 1 levels |
| v2 (relocation) against v1 | 20 / 26 | board L: 2, 230 and 65 px, at most 2 levels. Board B: only the ballast and raw-rectangle regions |
| Undo against v1 | 10 / 10 | none |
| Redo against v2 | 10 / 10 | none |
| Watcher dependency revert against v1 | 10 / 10 | none |
| v3 (padding and page change) against v1 | 20 / 26 | board L: 3, 352 and 136 px, at most 2 levels. Board B: only the raw-rectangle sprite (1,512 px at 1×) |
| After ten rejected replacements, against v3 | 6 / 6 | none |
| After the three render failures are undone, against v3 | 6/6, 6/6, 1/1 | none |
| Every source removed, against v3 | 26 / 26 | none |
| run-2 against run-3, from scratch | 229 / 229 | none |

- **Where the linear differences are.** Attribution by projected sprite quads puts every
  board-L difference inside a linear sprite: orb and halo gradients, the 30° crate,
  card 7, the probe, the panel and the Wisp. The card-7 pixels lie 1.4 to 2.7 texels
  inside its edges, at glyph transitions. They are UV rounding, not bleed from atlas
  neighbours.
- **Pixel-perfect 3× against exact 3× orthographic.** They differ only in two places on
  board S, the half-pixel tile row and its rim, and in six cases on board N:
  non-integer stretches, compressed borders, density 32, the 1-1-1-1 probe slice, and
  the 12° panel with its rim. Pixel-perfect output replicates the 1× frame.

### Decoded checks, measured from engine pixels

- **Strips across span cuts.** Every pixel of 112 strip rows was checked against the
  documented nearest rule: 109,088 along-axis pixels, 0 mismatches. The rule is
  region-local texels, a bias of 1/1024 screen px toward increasing screen coordinate,
  and clamping to the region.
  - Interior runs are exactly M px at every M from 1 to 32. Steps are ±1, with no holes
    or extra pixels.
  - At every visible cut the texels on each side are consecutive. Examples are
    511 | 512, and 588 | 587 for a flipped strip, whose cut sits at w − 512.
  - The 700-texel sub-rectangle gives 611 | 612, and 288 | 287 flipped.
  - At 32×, where 1100 × 32 = 35,200 > 32,768, the leading and trailing texels are
    exactly 32 px. That includes the flipped leading texel 1099, at aligned and tie
    placements on both axes.
  - Negative controls fail as they must: a wrong tie side fails 44 of 112 rows (every
    exact-tie row), an origin one texel off and inverted flips fail 112 of 112.
  - Figures: `screenshots/figures/decoded-x-cuts-1024x832.png` and
    `decoded-y-cuts-832x1024.png`.
- **Exact predictions at exact placements, at 1× and 3×.** Each sprite is predicted from
  its source image and compared pixel for pixel. All 152 sprites match with 0
  mismatching pixels:
  - board S: 84 sprites;
  - board B: 9, including the raw, stable and scoped card 7;
  - board T: 45 orientation cases and 5 pivots, including tie rows;
  - board N: 9 nine-slices, covering native, 72×44, three flips, 180°, mirror,
    density 8 and the simple 24×24.

  Pixels whose source value exceeds the display's identity range (sRGB 234) are skipped,
  83 at 1×; they are the Wisp eyes, the star and the markers.
- **Painter order.** Six groups, predicted from (layer, order, stable entity ID) for
  opaque and binary-alpha sprites, all match exactly: layer, order, ID tie, Z ignored,
  atlas/ordinary batch interleave, and the Wisp stack. Drawing in reverse order
  mismatches 232, 232, 232, 232, 50 and 232 px.
- **Transparent edges and fringes.** The halo hides green RGB and the orb hides magenta
  RGB under zero alpha. With linear sampling at 3× and 2.5× on dark and light plates,
  and nearest at 2×, no pixel is tinted toward the hidden colour: 0 in every region.
- **Level-0 sampling, minified.** On board Z, the halo is minified to 6, 9 and 12 px
  and card 7 to 3 and 6 px, both linear and nearest. They are byte-identical to the
  control, so no neighbour or mip content reaches them.

### Public workflows

| Step | Result |
| --- | --- |
| Undo / Redo | `history.undo` reverts the v2 import (transaction `01M4KSP0PPE0KFQQPTPH0ZMGMY`). The fingerprint returns to v1 and the frames equal v1. Redo returns v2 exactly. |
| Live watcher, dependency only | Copying the v1 `ballast.png` back while `watch-assets` runs reimports the atlas: actor `source-watch`, `changed: true`, `cache_hit: true`, fingerprint equal to v1. The runtime atlas generation goes 1 → 4, and the frames equal v1. |
| Live watcher, failing descriptor | An unfittable descriptor yields "could not cook art/atlas.atlas.json: frames do not fit in one atlas within the requested dimensions". The runtime keeps the v3 atlas at generation 1 through every event. Restoring the file clears the diagnostic, the watcher exits 0, and project and journal bytes are unchanged. |
| Ten rejected `import`s | `NoFit`, `Bounds` ×4, `Identity`, `Invalid` (rectangle exceeds image), `Frames` (257), `Limit` (decoded source texels) and `Invalid` (linear usage) all exit 1. Project and journal are unchanged, and later frames are identical. |
| Exact texel limit | 4,194,304 decoded source texels are accepted (a 20×80 page); one more is rejected. |
| Shrunken nine-slice frame | Import exits 0 and `validate` reports valid. Then every screenshot fails, including boards without the sprite: "sprite entity 01JA34EN000000000000000056 (N:native), texture …, source region [429, 82, 8, 8], cooked texture size [1204, 1865]: nine-slice borders must fit…". Undo restores exact frames. |
| Removed frame (card B) | Import exits 0. Every screenshot fails with "sprite atlas frame 01JA34FR00000000000000000V is unavailable in texture …"; no entity is named. Undo restores exact frames. |
| Frame ID on an ordinary texture | The command is accepted. The render fails explicitly, "…unavailable in texture <badge>". Undo restores. |
| Source-free | 11 source files removed (both descriptors and all images); 26 of 26 frames are identical. Reimport then fails with `Io(NotFound)`. |
| Repeated fresh processes | Every screenshot is a new process. 34 of 34 repeats are identical, and run-2 equals run-3 across 229 frames. |

Retained prepared GPU scenes across a replacement cannot be reached through the public
harness. Each `screenshot` prepares one scene in a fresh process, and no public call
holds a prepared scene across a reimport. I verified only two forms of retention:
- old cooked versions stay loadable, through Undo and the watcher's cache hit;
- the CPU runtime keeps the old atlas on a failed reload, through the watcher.

Astra's retained-scene GPU test is not independently verified.

### Large packed origin probe (`tools/atlas_extra.py`, `evidence/extra/`)

A maximum height of 8 forces seven 1040-texel frames side by side. The page is
7308×5, and frame w6 sits at (6266, 2), sourced from strip texels 60–1099, so its cut is
at page x 6778.
- **Nearest:** all 9 rows decode exactly. That covers 1× aligned, tie, flipped and
  flipped-tie, the neighbouring frame w3, and 8× windows on the cut in four placements.
- **Control and repeat:** the pixel-perfect frame equals the control byte for byte, and
  the fresh repeat is identical.
- **Linear:** in the fractional and rolled views, 10 and 2 px differ by 1 level, only
  in the 2.5× linear strip at the large origin.

### Shared renderer: upper-span ties on atlas frames (`tools/atlas_upper_probe.py`, `evidence/upper/`)

0032 round four reports exact-tie failures at span texels 236–511 from 20×. I drew the
atlas's 1100-texel rows and columns at 36×, 40×, 48× and 56×, at exact ties, with windows
over span texels 236 to about 548, in 1920×1080 pixel-perfect frames. The probe ran
twice, with the boards' world positions swapped.

- **The atlas inherits the limit exactly.** Atlas and control are byte-identical in
  all four frames, so atlas packing adds nothing.
- **Every failure is a tie** and resolves to the lower texel. There are no non-tie errors.

| Source axis, board centre | Failed exact ties at 36× / 40× / 48× / 56× | Failing span texels |
| --- | --- | --- |
| x, origin | 93/324, 278/288, 219/320, 269/315 | 236–511 |
| x, (500, 500) | 53/324, 262/288, 198/320, 262/315 | 239–511 |
| y, (500, 500) and origin, identical | 124/300, 185/297, 36/322, 232/320 | 239–511, **and 512–526 just after the cut** |

On the source-Y axis, exact ties at strip texels 512 to 526 also fail at 36×, 40× and
56×. That includes texel 512 itself, the first texel after the cut, which shows 511 at
40× and 56×. Both board positions give identical counts, so this follows the axis, not
the world position.

0032 states "None is on a span cut" and "span texels 0 to 57 never failed, up to 128×",
and its 64-texel-span option leans on that observation. My measurements contradict
both statements for this axis. Figure: `screenshots/figures/decoded-upper-y-40x-cut-ties.png`.

### Packing efficiency (import only, no frames; `evidence/packing/`)

The same 130 frames and sources were cooked under several maximum sizes:

| maximum_size | Page | Texels |
| --- | --- | --- |
| 2048×2048 | 1204×1865 | 2,245,460 |
| 1204×1170, 2048×1170 or 1300×1200 | 1204×1166 | 1,403,864 |
| 1600×2600 | 1204×2260 | 2,721,040 |
| 1204×1148 | rejected, NoFit | – |
| 1204×1147 and 4096×1024 | rejected, NoFit (genuinely impossible) | – |

- **Why the page grows.** A taller bound makes best-short-side-fit drop the 700-texel
  column below the 1100-texel one instead of beside it. The packer searches several
  widths but never heights.
- **Memory.** Sprites upload level 0 as RGBA16Float, so the GPU copy grows from about
  10.7 MiB to 17.1 MiB for identical frames.
- **Minimal reproduction of the false NoFit:** three solid frames, 1200×40, 1100×4 and
  4×1100, with padding 2. Under maximum 1204×1148 import reports "frames do not fit". A
  valid layout exists by the engine's own rules: wide at (2, 2), the column at (2, 46),
  the row at (10, 46). With maximum 1204×2048 the page is 1204×1156.
  `tools/atlas_packing_probe.py --repro` regenerates this exactly.

## Appearance review

I viewed the engine frames themselves, not only the statistics.
- **Atlas frames are indistinguishable from their sources at pixel-perfect scales.**
  - Edges are hard and texel blocks uniform.
  - Each card shows its own digit and colour, and labels read correctly.
  - The magenta guards never appear, and extrusion is invisible.
  - Zero-alpha texels show the background exactly.
- **Relocation is invisible.** v1, v2 and v3 frames of every nearest board are
  pixel-identical. The only visible change is the raw-rectangle sprite:
  - v1: card 7;
  - v2: night sky from the grown ballast;
  - v3: a band of repeated strip-extrusion texels over empty page.

  That is the hazard stable IDs exist to avoid (`screenshots/figures/board-b-v1-v2-v3-zoom3.png`).
- **Nine-slice from the atlas is crisp.**
  - Corners keep their source density, and each corner gem and the gold tab sit in the
    correct corner after flips, the mirror and the 180° turn.
  - The 7° roll shows clean stair steps without cracks.
  - Density 32 on a 16 px/m camera drops border texels unevenly, as in 0032. Keep
    border density at or below camera density for pixel art.
- **Linear atlas frames are smooth.** Orb, halo and glow have no dark rim, halo or
  fringe on dark or light plates, and repack differences of 1 to 2 levels are invisible.
  Linear on pixel art (the Wisp at 2×, the probe at 3×, card 7) reads soft, as
  expected. Nearest remains the right default.
- **Wide strips are seamless.** Ramps run continuously across every span cut at 1× to
  32× and under the 3° roll. The decoded parity views are perfect checkers.
- **Cameras behave as expected.** At 2.5× ordinary orthographic, pixel art shows
  uneven 2 to 3 px texel widths, the expected reason to use pixel-perfect cameras.
  Exact 3× orthographic matches pixel-perfect 3× except on half-pixel placements.
- **The cooked page looks wasteful.** The decoded v1 page
  (`screenshots/figures/atlas-pages-v1-v2-v3-quarter.png`) is about 95% empty. The
  packing finding explains why.

## Requests for Astra

### 1. Packer: false NoFit, and page size that grows with the bound (low to medium)

- **Repro:** `python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_packing_probe.py --repro OUT`
  imports the three frames above at 1204×1148, 1204×2048 and 1204×1200.
- **Effects:**
  - Valid sets near a tight bound are rejected with a message that claims impossibility.
  - A generous bound can enlarge the page by 60% for the same frames (1204×1166 →
    1204×1865, both measured), costing phone memory.
- **Recommendations:**
  - Search candidate heights as well as widths, for example by retrying with a
    decreasing maximum height until NoFit. Alternatively, use a max-rects heuristic.
  - Reword NoFit as "could not pack these frames within the requested maximum
    (unrotated guillotine heuristic)".
  - Document that `maximum_size` influences the layout.
- **Regression tests:**
  - The three-frame set packs within 1204×1148.
  - For a fixed frame set, raising `maximum_size` never increases the page area.

### 2. Import diagnostics (low)

- **Repro:** `import` of any `fail-*` descriptor in `evidence/art/`.
- **What happens:** `import` prints the Debug form, for example
  `Error: Cook { path: "art/atlas.atlas.json", source: Atlas(Bounds) }`. The watcher
  prints readable text for the same failure. `Atlas(Bounds)` covers four different
  mistakes: padding 0, padding 17, too many texels, and too wide.
- **Recommendation:** print `ImportError`'s Display text, as `watch-assets` does.

### 3. Accepted reimports that make every render fail (medium for authoring)

- **Repro:** import `render-fail-shrink-panel.atlas.json` or
  `render-fail-drop-cardB.atlas.json` over the v1 project.
- **What happens:**
  - The import commits, and `validate` reports `valid: true`.
  - Every `screenshot` of the project then fails, including scenes that never draw the
    affected sprite.
  - The nine-slice error is precise: it names the entity, texture, packed rectangle and
    page size.
  - The missing-frame error names only the frame and texture. A frame ID on an ordinary
    texture is also accepted at command time.
- **Recommendations:**
  - When an atlas reimport removes a referenced frame, or shrinks one below its
    nine-slice borders, list the affected sprites in the import reply, as a warning or
    a rejection.
  - Let `validate` check sprites against cached layouts.
  - Add the entity ID to the missing-frame error.
  - This extends 0032's round-one request 2 to atlases.

### 4. Explicit pixel rectangles on atlas textures (low, authoring hazard)

- **What happens:** a `[x, y, w, h]` region on an atlas asset renders correctly until
  the next repack, then shows unrelated content or extrusion, silently.
- **Recommendation:** have `validate` or import warn, or reject explicit pixel regions on
  atlas textures, steering authors to frame IDs. The Inspector could show the resolved
  frame instead.

### 5. Linear sampling is not byte-stable across repacks (informational)

- **What happens:** linear frames differ by up to 2 levels on a few hundred pixels
  between page layouts and against the original image. The cause is normalized-UV
  rounding, and it is invisible.
- **Recommendation:** golden-image tests should not expect byte equality for linear atlas
  frames across repacks. If exact stability is wanted, region-local manual bilinear
  sampling with `textureLoad` would provide it.

### 6. Shared renderer high-magnification ties (low; for 0032 and the shared fix)

- **Status:** atlas frames inherit 0032's measured limit exactly.
- **Additional repro:** `python3 -I handoffs/0034-sprite-atlas-review/tools/atlas_upper_probe.py OUT`.
  On the source-Y axis, exact ties fail just after the cut: strip texels 515 to 526 at
  36×, 512 to 522 at 40× and 512 to 525 at 56×. At 40× and 56× that includes the cut
  texel 512.
- **Recommendation:** add this case to the shared regression. Re-check the 64-texel-span
  option's premise; my measurement suggests that the span's on-screen size, not only the
  texel coordinate, matters. That cause is an inference.

## Changed paths

All changes are inside `handoffs/0034-sprite-atlas-review/`. `brief.md` is unchanged.

| Path | Purpose |
| --- | --- |
| `result.md` | this report |
| `tools/atlas_art.py` | original art, the atlas descriptors and the manifest |
| `tools/atlas_scene.py` | boards, cameras, and the atlas and control texture bindings |
| `tools/atlas_lookdev.py` | the public-path state sequence, hash checks and the packet budget ledger |
| `tools/atlas_analyse.py` | page verification, differential and decoded checks, attribution |
| `tools/atlas_figures.py` | derived figures |
| `tools/atlas_extra.py` | the large packed origin and minified-linear probe |
| `tools/atlas_upper_probe.py` | the upper-span tie probe, with `--swap`, `--decode-only` and `--stats` |
| `tools/atlas_packing_probe.py` | import-only packing probe, with `--repro` |
| `tools/atlas_controls.py` | negative controls |
| `tools/atlas_evidence.py` | evidence assembly; never replaces a file with different content |
| `tools/test_atlas_tools.py` | 11 self-tests |
| `evidence/` | analyses, helper records for both final runs, the scene, art, extra/upper/packing records, controls, the budget ledger and image hashes |
| `screenshots/engine/` | 21 unedited engine frames |
| `screenshots/figures/` | 7 derived figures |

Nothing else changed: no Rust, shader, Cargo, SDK, transport, schema, docs or
`editor/ui` file. 0032's folder is as I found it; I removed only bytecode caches my
imports had created there.

## Commands and results

From the repository root, `P=handoffs/0034-sprite-atlas-review/tools`,
`A=artifacts/0034-sprite-atlas-review`:

```sh
python3 -I $P/atlas_lookdev.py $A/run-2        # 285 calls, 229 frames, 5 expected failed shots
python3 -I $P/atlas_lookdev.py $A/run-3        # identical
python3 -I $P/atlas_analyse.py $A/run-2 $A/analysis/final-run-2c.json $A/run-3
python3 -I $P/atlas_extra.py $A/extra-1
python3 -I $P/atlas_analyse.py $A/extra-1 $A/analysis/extra-1.json --extra
python3 -I $P/atlas_upper_probe.py $A/upper-1
python3 -I $P/atlas_upper_probe.py $A/upper-2-swapped --swap
python3 -I $P/atlas_upper_probe.py $A/upper-1 --decode-only            # and upper-2-swapped
python3 -I $P/atlas_upper_probe.py $A/analysis/upper-span-stats.json --stats $A/upper-1 $A/upper-2-swapped
python3 -I $P/atlas_packing_probe.py $A/run-2/art $A/packing-probe-1
python3 -I $P/atlas_packing_probe.py --repro $A/packing-repro-2
python3 -I $P/atlas_controls.py $A/run-2 handoffs/0034-sprite-atlas-review/evidence/analysis-run-2.json $A/analysis/controls-run-2.json
python3 -I $P/atlas_figures.py $A/run-2 $A/analysis/final-run-2c.json $A/figures-3 --upper $A/upper-2-swapped
python3 -I $P/atlas_evidence.py
python3 -I $P/test_atlas_tools.py                                       # 11 tests OK
python3 -I handoffs/0032-sprite-rendering/tools/test_reference.py       # 13 OK (reused module, unchanged)
python3 -m unittest discover -s tools/tests                             # 25 OK (repository baseline)
cd handoffs/0034-sprite-atlas-review && shasum -a 256 -c evidence/screenshots.sha256   # 28 OK
```

Preliminary runs are preserved, not overwritten, and not used for the verdict:
- `run-smoke1`: the earlier palette;
- `run-1`: the scene before the 32× edge rows and the scoped atlas, with the same
  verdicts;
- `run-smoke2`: smoke of the final scene.

Two harmless slips occurred and both were fixed. A loop variable shadowed the probe
summary's filename, so `upper-2-swapped`'s summary was written as `UV:56x:506` without
end-of-run fields; `evidence/upper/provenance.json` records this. A 3× pixel-perfect
prediction initially ignored integer replication; it was my model, not the engine.

Not run: Cargo, Clippy, the Rust and GPU suites, the UI suite (no UI file changed),
`play` or PlaySession captures, native editor captures, hosted CI, other GPUs and phones.
I did not run the watcher on the render-failing descriptors.

## Budget

| Item | Value |
| --- | --- |
| Frames per invocation | 1; largest 1920×1080, 7.9 MiB raw |
| Packet total | 744 frames, 810.1 MiB raw, against limits of 2,048 frames and 1 GiB |
| Final runs | each 229 frames and 237.0 MiB raw |
| Probes | extra 9 frames; upper-span 4 + 4 |
| Overwritten evidence | none |

Expected failed screenshots wrote no frame: 15 across the three full runs. Every frame
is in `evidence/budget-ledger.jsonl`, with per-run totals in `evidence/budget.json`.

## Artifact manifest

Committed in this packet:
- `evidence/`: the files listed under "Changed paths".
- `screenshots/engine/` (21 files):
  - v1 at 3×: boards S, T, N, O, L and B;
  - the 1× strip boards W and V, and the magnified boards X and Y;
  - orthographic views: S at 2.5× with an offset, S at exact 3×, N rolled 7°, W rolled 3°;
  - v2 board B, pixel-perfect and rolled;
  - v3 board B, pixel-perfect and 2.5×;
  - board Z with packed x = 6266;
  - upper-span X and Y at 1920×1080.
- `screenshots/figures/` (7 files, derived, not engine frames): decoded X and Y cuts,
  cooked pages v1 to v3, board B in v1 to v3, the ×64 linear difference, the
  upper-span Y cut ties, and the art contact sheet.

Ignored, under `artifacts/0034-sprite-atlas-review/`: every run directory with its
projects, journals, cooked caches and all raw frames, the analyses, figures, probes and
the ledger.

## Data binding needed later

A sprite and atlas Inspector (Phase 2) will need a typed, read-only view of each atlas
asset's current cooked layout:
- frame ID → name, packed rectangle, source path and rectangle;
- page size and fingerprint.

It also needs a way to flag sprites whose frames are missing or too small, without
reading cache files. Today this data is only in the `import` reply and the cache
metadata.

## Limitations

- **Hardware:** one GPU and one backend, Apple M5 Pro with Metal, headless only. No
  other adapter, no phone, no performance claim.
- **Display transform:** the exact predictions assume the measured identity range of
  sRGB 0 to 234 from 0032; brighter pixels are skipped, not judged.
- **Upper-span sampling:** counts depend on magnification, axis, position and frame
  size. The sample is positions I chose, not an exhaustive map. Causes are inferred.
- **Packing set:** my frame set is deliberately adversarial (1100-texel strips).
  Typical sprite sets may pack better.
- **Not covered:** perspective cameras, atlas pages used as model textures (their mips
  mix frames), multiple pages, rotation, compression tiers, tilemaps, animation
  libraries and 2D physics. None of these is claimed.
- **Gameplay path:** no legacy PlaySession or native ECS gameplay was exercised.
- **Gates:** no phase gate is approved or claimed, and the shared renderer is not
  labelled accepted.

## Open questions

1. Should the packer treat `maximum_size` purely as a bound, by searching heights or
   compacting, even at some extra cook time?
2. Should an atlas reimport that breaks referencing sprites warn or be rejected at
   import time? Should explicit pixel regions on atlas textures be linted?
3. Are byte-stable linear atlas frames across repacks a requirement for golden tests?
4. Should the shared regression adopt the source-Y cut case, and should the bound be
   documented as "ties hold to 16×" for atlases too?
