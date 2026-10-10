# 0032 sprite rendering: original art and real-engine review

The round-four wide-region follow-up comes first. Rounds three and two follow
unchanged, then round one from "Status" onward.

## Round 4: 512-texel spans with a flat base

### Verdict

- **The round-three defect is fixed.** Every flipped strip on boards I and J is now
  uniform at exact ties:

  | Board | Pre-wide build | Candidate |
  | --- | --- | --- |
  | J, region widths 512 to 8192 at 1× to 32×, 60 strips | 10 failures | 0 |
  | I, a 4096-texel strip at 1× to 16×, 40 strips | 2 failures | 0 |

- **Accepted, scoped, up to 16×.** Four new boards hold 606 span rows. At 1× to
  16×, every exact tie and every near tie one raster step away resolves as the rule
  requires:

  | Ties at 1× to 16× on boards K to N | Pre-wide build fails | Candidate fails |
  | --- | --- | --- |
  | 33,510 exact ties | 6,671 | 0 |
  | 54,654 near ties, 1/256 px away | 1,428 | 0 |

  The rows cover both source axes, both texel directions, and windows on span cuts
  and inside spans. They use offsets of 0 and ±1/256 px, odd region origins and
  widths, and end spans of 1 and 511 texels. They also include mirrors, half and
  quarter turns, and wide and compressed nine-slices.
- **Concrete failures above 16×.** A tie at span texels 236 to 511 can resolve to the
  wrong texel, usually the lower one. On boards K to N, 225 of 3,060 exact ties at
  24× to 64× fail. A 20× to 128× probe finds the first failure at 20×:

  | Probe ties, 20× to 128× | Pre-wide build fails | Candidate fails |
  | --- | --- | --- |
  | 9,684 exact ties | 4,578 | 1,095 |
  | 9,684 near ties | 5,124 | 132, from 44× up |

  The pre-wide build fails far more of the same ties, so this is a remaining limit of
  the correction rather than a new defect. Two rows pass only on the pre-wide build,
  and both are the same 48× and 64× kind. See "Remaining limit".
- **No seams.** No decoded row has a hole, an extra pixel or an edge error: every
  candidate mismatch is a tie. Translucent strips at 1× and 8× match the reference
  exactly in both directions, so spans neither overlap nor leave gaps.
- **Nothing else changed.** The same scene on both builds:

  | Frames | Byte-identical | Changed | Changed pixels |
  | --- | --- | --- | --- |
  | 39 | 31 | 8 | 56,094 |

  The 8 changed frames all show wide strips. Their changed pixels are 53,523 texel
  ties, 2,537 rule deltas from the new span vertices and 34 edge ties on the rolled
  view. None has a linear top sprite and none is unexplained. Tone, the gray ramp,
  alpha, sorting, the linear fringe and the live frames match round three.
- **Repeatability and resize hold.**
  - Two fresh candidate runs match byte for byte across 263 common images.
  - Every source-free frame equals its with-source frame.
  - Reimporting the strip twice as tall and the column twice as wide leaves all
    seven span frames byte-identical.
- **Budget kept.** Five runs used 474 frames and 609.0 MiB raw, out of 2,048 frames
  and 1 GiB. The largest invocation wrote 34 frames, and the largest frame was
  7.9 MiB raw.

This verdict covers headless rendering on one Apple M5 Pro with Metal. It does not
imply portability, phone performance, native editor acceptance, hosted CI or any
phase gate.

### Decisions acknowledged

- The candidate cuts nearest geometry every 512 source texels and adds a flat integer
  base after the bias, as Astra describes. I read the source snapshots only. Nothing
  was built, and no Rust, shader, Cargo or engine file changed.
- I kept to the packet's budget and wrote no native app or `editor/ui` code: 0033
  owns that work. I did no merging, publishing or account work.
- The pixel-camera near-tie follower defect remains open. This round did not test it.
- Rounds one to three are not rewritten. Their evidence, screenshots and result
  sections are unchanged. Astra's updated `brief.md` stays uncommitted, as delivered.

### Model, effort and provenance

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`) at Max effort, in the
  resumed ACP runner session. The runner confirms both settings before sending the
  prompt; I cannot inspect that handshake from inside the session.
- **Worktree:** `7950fa8`.
- **Candidate source:** the four snapshot files under `artifacts/tools/wide-source/`
  match the hashes in `artifacts/tools/wide-fix-evidence.json`. They are the shader,
  the sprite module, the span geometry and Astra's wide-region GPU test.

| Build | Source | sha256 |
| --- | --- | --- |
| Active candidate, `incant_headless` | `0494b40cf8fcc5e1ad0fc689655a62cb264a2f50` | `1f338d8403fae4829d01c0a3e2dfcf5af81cac3c876a155b7d9f489231096c50` |
| Pre-wide | `f9aa1da0818313c5c6cac580e54d7107646378bc` | `a4b5ddb905122a55b7d156ad39b39e7f97f5266e70159614fe4cdc7589796828` |
| Pre-magnification, not run | `c2c7758c14c37d36aa13e322aa2c87e63ce489ad` | `fd65133fc98a0c6d533f23d9583a820271858eecdb0849b44f8b7e996a5e072b` |
| Pre-ties, not run | `39e5c38107451ba44a02ef7b80f27bef11d4141d` | `848b8294854add3bc672e6ab97e1c3dc1f5f39a9c5b1c234e61e5edace5820ba` |

Each binary matched its metadata before and after all 370 invocations.

### Changed paths (round 4)

All changes are in `handoffs/0032-sprite-rendering/`.
- `tools/sprite_lookdev.py`:
  - `--binary` also takes `pre-wide`;
  - adds boards K, L, M and N, their cameras, and reimport steps for the strip and
    column at another size;
  - `--repeats`, `--live` and `--static-only` select a run plan;
  - `--probe` adds boards O and P and captures only their four views;
  - every renderer invocation is counted and refused beyond 128 frames or 256 MiB.
- `tools/sprite_art.py` adds a 2 × 8192 column, resized twins of the strip and column,
  and a separate label sheet for the new boards. Every earlier art file, including
  `labels.png`, is byte-identical.
- `tools/reference.py` models the candidate as rule `span-band`:
  - an independent port of the span tessellation;
  - span-local texels with a flat base;
  - exact predictions for axis-aligned ties, keeping tolerance only for rotated
    geometry.
- `tools/analyse.py`:
  - maps `0494b40` to `span-band`;
  - checks every pixel of every span row against its exact rational mapping;
  - counts exact and near ties by magnification and span texel;
  - checks translucent and linear rows against the reference;
  - handles static-only and probe runs.
- `tools/compare_runs.py` reads the shared reimport texture variants.
- `tools/round4_figures.py` is new.
- `tools/test_reference.py` has 17 tests. The new ones include a port of the
  candidate's own geometry invariants, which pass against my tessellation port.
- `evidence/round4/` holds four analyses, the before/after comparison, five helper
  records, the scene, the budget ledger, the figure manifest and image hashes.
- `screenshots/round4/` holds nine images, listed under "Round-four frame set".

The extended scene leaves earlier boards untouched. On the same `f9aa1da` binary, all
33 earlier frames are byte-identical to round three's run.

### Commands and results (round 4)

```sh
T=handoffs/0032-sprite-rendering/tools; R=artifacts/0032-sprite-rendering/round4
python3 -I $T/sprite_lookdev.py $R/c1 --repeats 2 --live e,ne
python3 -I $T/sprite_lookdev.py $R/pw1 --binary pre-wide --repeats 1 --static-only
python3 -I $T/sprite_lookdev.py $R/c2 --repeats 1 --live e
python3 -I $T/sprite_lookdev.py $R/probe-c1 --probe --repeats 2
python3 -I $T/sprite_lookdev.py $R/probe-pw1 --probe --repeats 1 --binary pre-wide
python3 -I $T/analyse.py $R/c1 $R/final/analysis-c1-committed.json $R/c2
python3 -I $T/analyse.py $R/pw1 $R/final/analysis-pw1-committed.json
python3 -I $T/analyse.py $R/probe-c1 $R/final/analysis-probe-c1.json
python3 -I $T/analyse.py $R/probe-pw1 $R/final/analysis-probe-pw1.json
python3 -I $T/compare_runs.py $R/pw1 $R/c1 $R/final/compare-pw1-c1.json
python3 -I $T/round4_figures.py $R/pw1 $R/c1 $R/figures3
python3 -I $T/test_reference.py                  # 17 tests OK
python3 -m unittest discover -s tools/tests       # 10 tests OK
cd handoffs/0032-sprite-rendering && shasum -a 256 -c evidence/round4/screenshots.sha256   # 9 OK
```

| Run | Build | Plan | Invocations | Frames | Raw MiB |
| --- | --- | --- | --- | --- | --- |
| c1 | candidate | every frame twice, reimport, source-free, live e and ne | 159 | 265 | 276.2 |
| pw1 | pre-wide | static frames once | 53 | 39 | 81.8 |
| c2 | candidate | every frame once, reimport, source-free, live e | 118 | 158 | 179.5 |
| probe-c1 | candidate | probe views twice | 22 | 8 | 47.6 |
| probe-pw1 | pre-wide | probe views once | 18 | 4 | 23.8 |

- **Every run:** the binary was unchanged across every call, and authored files were
  unchanged by rendering and play. Frames rendered twice in c1 and probe-c1 were
  byte-identical, and so were both plays of each live direction.
- **Budget:** frames are counted per invocation before it runs, including the two
  expected badge-small failures per reimporting run. `evidence/round4/budget.json`
  holds the ledger. Nothing is overwritten. Earlier figure drafts sit in ignored
  `figures/` and `figures2/` directories and are not evidence.
- **Not run:** I did not rerun the UI suite; no UI file changed. I also did not run
  Cargo, the GPU suites, native captures or hosted CI.

### Span boards

Each board is 1024 × 1024 px, so pixel-perfect projections stay dyadic. Each row is a
long strip from an 8192-texel strip or column whose colours name every texel. The row
is placed so one texel boundary sits on the pixel centre 512.5, plus an offset, and
the rest runs far off screen. Each row records its exact screen-to-texel mapping. The
analysis then names the expected texel of every pixel without the CPU model.
- **K, along screen x:** 1× to 64× in 13 steps, both directions, offsets 0 and
  ±1/256 px. Windows sit on a span cut and at span texels 256 and 448. Quarter-turned
  columns put source y on screen x.
- **L, along screen y:** the same sweep on the column, plus quarter-turned strips and
  a half-turned column.
- **M, variants:**
  - regions `[300, 0, 4097, 1]`, `[1, 0, 8191, 1]` and `[4000, 0, 1000, 1]`, ending in
    1-, 511- and 488-texel spans;
  - mirrors and half turns;
  - a 2000-texel nine-slice with 600- and 700-texel borders at 8× and 32×, and a
    compressed one;
  - translucent and linear rows.
- **N, isotropic:** 32×, 48× and 64× rows, also viewed at 1920 × 1080, whose 60 m
  half-width is not dyadic, and through a 7° rolled camera.
- **Probe O and P, opt-in:** 20× to 128× in 13 steps along x and y, at span texels
  32, 288 and 480, offsets 0 and +1/256 px, at 1024 × 1024 and 1920 × 1080.

Before rendering, I checked each board against the CPU model: the decoders and the
model agree on every decoded pixel.

| Failed rows | Pre-wide build | Candidate |
| --- | --- | --- |
| K, along x | 105 of 226 | 3 |
| L, along y | 95 of 234 | 6 |
| M, variants | 36 of 120 | 6 |
| N, isotropic | 12 of 18 | 2 |
| N at 1920 × 1080 | 11 of 18 | 6 |

Every candidate failure is an exact tie at 32× to 64×. The pre-wide failures include
3,021 near-tie pixels; the candidate has none.

### Remaining limit: magnified ties in the upper half of a span

The probe, all four views together. Each cell gives failures out of ties:

| M | Exact ties: candidate | Exact: pre-wide | Near ties: candidate | Near: pre-wide |
| --- | --- | --- | --- | --- |
| 20× | 17 / 1,512 | 755 | 0 / 1,512 | 789 |
| 24× | 0 / 1,266 | 684 | 0 / 1,266 | 568 |
| 28× | 14 / 1,092 | 508 | 0 / 1,092 | 584 |
| 32× | 67 / 942 | 478 | 0 / 942 | 464 |
| 36× | 164 / 846 | 345 | 0 / 846 | 501 |
| 40× | 188 / 750 | 301 | 0 / 750 | 449 |
| 44× | 122 / 684 | 219 | 13 / 684 | 465 |
| 48× | 146 / 630 | 240 | 0 / 630 | 390 |
| 56× | 124 / 552 | 268 | 13 / 552 | 284 |
| 64× | 59 / 474 | 244 | 0 / 474 | 230 |
| 80× | 108 / 378 | 250 | 39 / 378 | 128 |
| 96× | 58 / 318 | 174 | 47 / 318 | 144 |
| 128× | 28 / 240 | 112 | 20 / 240 | 128 |

- **Where:** all 1,452 failing pixels, on boards K to N and in the probe, sit at span
  texels 236 to 511. None is on a span cut. In the probe, span texels 0 to 57 never
  failed, up to 128×.
- **Which way:** exact ties fail almost only where texels increase along the screen
  (1,276 of 1,320). Near ties fail only where texels decrease. In 1,408 of the 1,452
  failures the engine shows the lower of the two texels.
- **When:** failures are more frequent in the 1920 × 1080 views and at magnifications
  that are not powers of two. At 1024 × 1024 along x, 32× and 64× never failed, while
  36× to 56× did.

**Why, inferred from these measurements, not from hardware documentation.** Span
texels from 256 to 512 have an f32 spacing of 2^-15, about 3.1 × 10^-5 texel. The tie
bias is 1/1024 px, which is 1/(1024 M) texel: 6.1 × 10^-5 at 16×, 4.9 × 10^-5 at 20×
and 3.1 × 10^-5 at 32×. On this GPU the interpolated span texel can come out up to
about two f32 steps low. An exact tie then lands on the lower texel once the bias
falls below that error. The 512-texel span removed the region-width dependence of
round three, but at high magnification the span texel itself is still too large.

**Severity:** low. A failure needs three things at once:
- a source region wider than about 236 texels;
- 20× or more magnification;
- texel boundaries exactly on pixel centres.

One texel then gives one pixel to its neighbour, at most 1/20 of its width. The
pre-wide build failed in many more of these cells.

**Options for Astra, who owns the decision.**
- Document the measured bound: exact and near ties hold up to 16×.
- Use shorter spans. Span texels below 58 never failed up to 128× here, so 64-texel
  spans would likely move the bound near 128×; this is an inference. They cost 8×
  more vertices per axis. A simple 8192 × 8192 region would grow from 1,536 to 98,304
  vertices.
- A larger bias alone cannot fix it. It must stay below half the 1/256 px raster step
  to keep near ties apart, so it can at most double the bound.

### Other checks (round 4)

- **Seams:** decoded rows have no hole, extra pixel or edge error.
  - Translucent strips at 1× and 8× match the reference exactly along their full
    length. At 48× the only difference is one tie pixel, off by two levels.
  - Linear rows stay within one level.
  - The 7° rolled view of board N differs from the model only at 56 pixels the model
    flags as ambiguous.
- **Resize invariance:** reimporting the strip at 8192 × 4 and the column at 4 × 8192,
  with red texels outside every region, leaves boards J, K, L, M and N byte-identical.
  The taller sheet again changes no nearest pixel on boards A to H, and one linear
  pixel by one level, as before.
- **Linear and tone:** the comparison changes no pixel whose top sprite is linear.
  The gray ramp shows 0 to 234 exactly and 252 to 255 as 243. The fringe numbers
  equal round three's.
- **Earlier boards:** ruler stations, orientation, nine-slice flips, painter probes,
  cracks and the minification mirror results all equal round three. Boards F, G and
  H report no failures.
- **Live:** directions e and ne match the model in every frame, with zero latency and
  equal repeats. All 68 of their frames are byte-identical to round three's. The other
  six directions were not rerun, to save budget.
- **Model accounting:** for this build the model predicts axis-aligned span ties
  instead of flagging them.
  - Its 7,255 unexplained pixels are exactly the tie failures above, over each row's
    full height. Reimported copies of the span frames count them again.
  - Its 209 ambiguous mismatches are 150 such failures on quarter-turned rows, which
    the model flags, and 59 flagged pixels on rolled views.

### Appearance review (round 4)

I viewed the four engine frames and the five decoded views.
- **Engine frames.** Rows are continuous, with no background inside a row and no
  visible step at span cuts. Neighbouring texels differ by two levels of red, so a
  one-pixel tie error cannot be seen in them.
- **Decoded views.** These mark each wrong texel in red. At 8× and 16×, the pre-wide
  flipped rows carry red lines at most texel edges; the candidate rows carry none.
- **32× to 64×.** In the upper half of a span, the candidate shows red lines only on
  unflipped rows: at 48× along x, and at 32× and 48× along y. The pre-wide build shows
  them on most of these rows.
- **Nine-slice.** Every nine-slice window at 32× is clean on the candidate. On the
  pre-wide build, three border and centre joins show four red pixels each, and one
  border cut shows eight.

### Round-four frame set

In `screenshots/round4/`:
- `engine-board-k-spans-along-x-active-1024x1024.png`,
  `engine-board-l-spans-along-y-active-1024x1024.png`,
  `engine-board-m-span-variants-active-1024x1024.png` and
  `engine-board-n-isotropic-active-1920x1080.png` are unedited candidate frames.
- `decoded-*.png` files are diagnostics derived from engine pixels, not engine images.
  Each pixel is redrawn by the parity of the texel it names, and red where that
  texel differs from the exact rule. The pre-wide window is above the candidate
  window in each case. `evidence/round4/figure-rows.json` lists every case with its
  red-pixel counts.
  - Board K upper-span ties at 32×, 48× and 64×, and K at 8× and 16×.
  - Board L upper-span ties at 32×, 48× and 64×.
  - Board M nine-slice windows at 32× and the short end spans at 48×.
  - Board N at 1920 × 1080, 48× and 64×.

### Limitations (round 4)

- **Hardware:** one GPU (Apple M5 Pro) and one backend (Metal), headless only.
- **Model inferences:** the f32 explanation of the remaining limit is inferred from
  measurements, not from hardware documentation.
- **Coverage gaps:**
  - Magnifications above 16× sample span texels 0 to 57, about 226 to 286 and about
    418 to 512. Other span texels were tested only at 16× and below.
  - Wide nine-slices with both axes above 512 texels are checked only by the geometry
    unit test.
  - Perspective cameras are untested.
  - The 128 MiB geometry bound is not exercised.
- **Gates:** no phase gate is approved or claimed. The high-magnification limit, the
  near-tie camera follower defect, the exact display option, the Inspector and
  native review remain open.

---

## Round 3: derivative-scaled tie band

### Verdict

- **Accepted, scoped: the magnified-edge regression is fixed.** On the active build
  (source `f9aa1da`), every near-boundary case renders uniform texels:

  | Case | Pre-magnification build | Active build |
  | --- | --- | --- |
  | Board G sweep, 1× to 8×, 140 cells | 15 failures | 0 |
  | Board H, 12× to 32× strips, quarter turns, anisotropic, minification, 102 cases | 45 failures | 0 |
  | Board I offset cases near texel 4096, 1× to 16×, 30 cases | 13 failures | 0 |

- **Earlier corrections still hold.** All 40 board F orientation cases pass, and so
  does board A's ruler station. A taller sheet with identical texels changes no
  nearest pixel on any board.
- **Concrete limitation, found by trying to falsify the change.** Exact ties at large
  region-local texel coordinates under magnification lose the tie rule. A flipped
  strip's leading texel then gains one pixel, for example 9 px instead of 8. On this
  GPU it happens exactly where region width × magnification reaches 32,768. That
  affects 10 of 60 board J strips and 2 of 40 board I strips. The pre-magnification
  build passed all of these. See "Remaining limit".
- **Nothing else changed.** The same scene on both builds:

  | Files compared | Byte-identical | Changed | Changed pixels |
  | --- | --- | --- | --- |
  | 342 | 321 | 21 | 74,520 |

  65,099 changed pixels are where the two documented rules predict different
  colours, and 9,421 are ties that either model marks ambiguous. None are
  unexplained. Linear sampling, tone, alpha, sorting, depth and all 272 live frames
  are unchanged.
- **Repeatability holds.** Two fresh active runs match byte for byte across 647
  images, and every source-free frame equals its with-source frame.
- **Diagnostics.** The undersized-reimport error now names the entity, texture,
  requested region and cooked size, as requested in round one:
  ```
  Error: "sprite entity 01JA32SP000000000000000092, texture 01M4KM0XBNCAGTYS3VTWZNAEX4, source region [0, 0, 16, 16], cooked texture size [8, 8]: sprite source region must be nonempty and fit inside a texture of at most 8192 pixels per axis"
  ```

This verdict covers headless engine rendering on one Apple M5 Pro with Metal. It does
not imply phone performance, native editor acceptance, hosted CI or any phase gate.

### Decisions acknowledged

- The neutral curve and linear-light blending stay. An exact display option can be
  per-camera when implemented.
- Optional sprite-grid snapping is separate work.
- The pixel-camera near-tie follower defect is separate, untested here and not
  claimed fixed.
- The Sprite Inspector and the app-wide redesign stay with packet 0033.
- No Rust, shader, build, Cargo or `editor/ui` file changed. Astra's updated
  `brief.md` stays uncommitted, as delivered.
- Rounds one and two are not rewritten. Their evidence, screenshots and result
  sections are unchanged.

### Model, effort and provenance

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`) at Max effort, in the
  resumed ACP runner session. The runner confirms both settings before sending the
  prompt; I cannot inspect that handshake from inside the session.
- **Worktree:** `305a95b`. The `f9aa1da` source was read from the shared object store
  without checking it out. It differs from `c2c7758` in two renderer changes: the
  nearest bias `direction*(1.0/1024.0)` and the typed reimport diagnostic.
- **Astra's evidence:** `artifacts/tools/magnification-fix-evidence.json` records a
  70-case GPU sweep going from 15 failures to 0. My independent sweep extends it.

| Build | Source | sha256 |
| --- | --- | --- |
| Active, `incant_headless` | `f9aa1da0818313c5c6cac580e54d7107646378bc` | `a4b5ddb905122a55b7d156ad39b39e7f97f5266e70159614fe4cdc7589796828` |
| Pre-magnification | `c2c7758c14c37d36aa13e322aa2c87e63ce489ad` | `fd65133fc98a0c6d533f23d9583a820271858eecdb0849b44f8b7e996a5e072b` |
| Pre-ties, not run this round | `39e5c38107451ba44a02ef7b80f27bef11d4141d` | `848b8294854add3bc672e6ab97e1c3dc1f5f39a9c5b1c234e61e5edace5820ba` |

Each binary matched its metadata before and after all 141 invocations of each run.

### Changed paths (round 3)

All changes are in `handoffs/0032-sprite-rendering/`.
- `tools/sprite_lookdev.py`:
  - `--binary` now takes `active`, `pre-magnification` or `pre-ties`;
  - adds boards H, I and J and a rolled camera on board G.
- `tools/sprite_art.py` adds a 64 × 64 ruler and 4096- and 8192-texel strips. Every
  texel names itself by colour, inside the tone curve's identity range. It also
  appends label words. All earlier art and label regions are byte-identical.
- `tools/reference.py`:
  - models the derivative-scaled rule as the active one;
  - scales its precision allowance with magnification, and with f32 resolution for
    rotated geometry;
  - flags biased coordinates that land within f32 error of an integer as ambiguous,
    including exact integers.
  
  With these changes, both round-three analyses report 0 unexplained pixels.
- `tools/analyse.py` maps each build source to its nearest rule. An unknown build
  fails explicitly. It also decodes boards H, I and J from engine pixels.
- `tools/compare_runs.py` classifies each changed pixel as a rule delta, a tie or
  "other" for any pair of builds.
- `tools/round3_figures.py` is new.
- `tools/test_reference.py` now has 13 tests, including the band rules at 8× and
  the large-coordinate knife edge.
- `evidence/round3/` holds both analyses, the before/after comparison, three helper
  records, the scene, the figure rows and the image hashes.
- `screenshots/round3/` holds eight images, listed under "Round-three frame set".

The extended helper reproduces earlier rounds exactly. Its pre-magnification run
matches all 635 images of round two's corrected-build run byte for byte.

### Commands and results (round 3)

```sh
T=handoffs/0032-sprite-rendering/tools; R=artifacts/0032-sprite-rendering/round3
python3 -I $T/sprite_lookdev.py $R/a1
python3 -I $T/sprite_lookdev.py $R/a2
python3 -I $T/sprite_lookdev.py $R/pm1 --binary pre-magnification
python3 -I $T/analyse.py $R/a1 $R/final/analysis-a1.json $R/a2
python3 -I $T/analyse.py $R/pm1 $R/final/analysis-pm1.json
python3 -I $T/compare_runs.py $R/pm1 $R/a1 $R/final/compare-pm1-a1.json
python3 -I $T/round3_figures.py $R/pm1 $R/a1 $R/figures
python3 -I $T/test_reference.py                  # 13 tests OK
python3 -m unittest discover -s tools/tests       # 10 tests OK
cd handoffs/0032-sprite-rendering && shasum -a 256 -c evidence/round3/screenshots.sha256   # 8 OK
```

Each run printed:
```
binary_unchanged: true, engine_calls: 141, authored_unchanged: true,
screenshot_repeats_equal: true, screenshots: 33, source_free_shots: 13,
play_repeats_equal: all 8 true, play_frames: 34 each
```

- **Budget:** each invocation stays within the packet's limits. The largest frame is
  3.1 MiB raw, and a play capture writes 34 frames, 7.5 MiB raw. A whole run writes
  647 frames, 259 MiB raw, over 141 invocations, so it exceeds 256 MiB only if the
  limit is read per helper run rather than per capture. Nothing is overwritten.
- **Not run:** I did not rerun the UI suite; no UI file changed. I also did not run
  Cargo, the GPU suites, native captures or hosted CI.

### Falsification boards

- **Board H, 1280 × 640:** four case families, each starting from tie placements
  (edges on pixel centres) and moving by offsets measured in 1/256 px:
  - the ruler's 7 × 1 top row at 12×, 16×, 24× and 32×, unflipped and flipped,
    offsets 0, 0.625, 1, 2 and 8;
  - quarter turns of 90° and 270° at 4× and 8×, offsets 0, 0.625, 1 and 2;
  - anisotropic rulers at 1 × 8 and 8 × 1, both orientations, the same offsets;
  - the 64 × 64 ruler minified 2×, 3× and 4×, on aligned and tie placements, plus
    offsets.
- **Board I, 1024 × 640:** a 4096-texel strip at 1×, 2×, 4×, 8× and 16×, unflipped
  and flipped, at offsets 0, 0.625, 1 and 3. Its visible texels lie near 4096.
- **Board J, 1024 × 640:** exact-tie strips of width 512, 1024, 2048, 4096 and 8192
  at 1× to 32×. Plain strips end on screen and flipped strips start on screen, so
  visible texels sit near the region width.
- **Rolled view:** a 7° orthographic camera over board G.

All of these are decoded from engine pixels alone. Each must show every texel
spanning exactly M pixels, in order. The reference model predicts all other pixels.
Both analyses report 0 unexplained pixels. In the active run, 3,776 of 3,779
ambiguous mismatches lie on the board I and J strips: these are large-coordinate
knife edges the model flags instead of predicting. The other 3 are on rolled frames.

### Remaining limit: exact ties at large region-local coordinates

Board J, flipped strips at exact ties. X marks a strip whose texels are not all M
pixels wide. Plain strips pass everywhere on both builds.

| Width \ M | 1 | 2 | 4 | 8 | 16 | 32 |
| --- | --- | --- | --- | --- | --- | --- |
| 512 | pass | pass | pass | pass | pass | pass |
| 1024 | pass | pass | pass | pass | pass | X |
| 2048 | pass | pass | pass | pass | X | X |
| 4096 | pass | pass | pass | X | X | X |
| 8192 | pass | pass | X | X | X | X |

The pre-magnification build passes every cell. In each active failure, only the
strip's first texel is affected. It gains exactly one pixel, for example 9 px at 4096
× 8 and 33 px at 1024 × 32. Board I agrees: its only active failures are the flipped
4096-texel strips at exact ties at 8× and 16×.

**Why.** Region-local coordinates reach the region width, and f32 spacing grows with
them. Near t = 4096 the spacing is about 0.0005 texel, while the scaled bias at 8×
is 1/8192 texel. Once width × M exceeds about 2^14, adding the bias is lost to
rounding, and the tie falls back to raw interpolation. The measured failures begin at
2^15 = 32,768, the first width × M tested past that point.

**Trade-off.** The old fixed band survived large coordinates but swallowed non-ties
after magnification. No single texel-space bias can do both near t = 4096 at 8×,
because f32 cannot resolve a 1/256 px raster step there.

**Severity:** low. It needs a region at least 1024 texels wide magnified 32×, or 4096
at 8×, or 8192 at 4×, placed exactly on a tie. It moves one leading texel by one
pixel. Typical sprite and tile regions of up to 512 texels pass at every tested
magnification.

**Recommendation for Astra.** One option is to document the bound: exact ties hold
while region width × magnification stays at or below 16,384. To remove it, keep
interpolated coordinates small, for example by splitting regions wider than 512
texels into spans with a flat integer base. The derivative-scaled band then stays
above f32 resolution. This is a recommendation only; Astra owns integration and
verification.

### Other checks (round 3)

- **Minification:** behaviour is identical on both builds. For 14 of 15 placement
  pairs, a flipped sprite is not the mirror image of the unflipped one. At 2×
  minification on an aligned placement, the unflipped sprite shows texels 1, 3, 5
  and so on, and the flipped one shows 62, 60, 58. The rule that ties take the
  increasing-screen side picks a different texel subset per orientation. This is an
  inherent property of the rule, not a regression. Only 3× on an aligned placement,
  which has no ties, is mirror-consistent.
- **Resized sheet:** reimporting the 128 × 80 sheet changes no nearest pixel on
  boards A to H. One linear pixel moves by one level, as before.
- **Fractional and rolled views:** all match the model. Their only changes from the
  pre-magnification build are rule deltas and ties.
- **Linear, tone and live:** of 74,520 changed pixels, only one has a linear top
  sprite, and it is a rule delta beneath translucent content. The gray ramp still
  shows 0 to 234 exactly and 252 to 255 as 243. The fringe counterfactual is
  unchanged, no cracks appear, and live frames are identical.

### Appearance review (round 3)

I viewed the active frames and the decoded before/after views.
- **Board G.** The 8× and 6× cells at offsets 0.625 and 1 lose the narrow first texel
  and wide last texel they had before. All cells now read as even checkers.
- **Board H.** In the 32× view the old one-pixel errors are barely visible, so the
  measured widths carry board H.
- **Board J.** At width × M of 32,768, the active strips' first stripe is visibly
  wider than the rest. The pre-magnification strips are even.
- **Everything else.** The pixel-perfect frames of boards A to F, the showcase and
  the live follow are byte-identical between builds. The fractional and rolled views
  of boards B, D and E differ by 2 to 95 pixels each, all rule deltas, and look the
  same.

### Round-three frame set

In `screenshots/round3/`:
- `engine-board-g-offset-sweep-active-1024x640.png`,
  `engine-board-h-falsification-active-1280x640.png`,
  `engine-board-i-wide-strip-active-1024x640.png` and
  `engine-board-j-region-width-map-active-1024x640.png` are unedited active frames.
- `decoded-*.png` files are diagnostics derived from engine pixels, not engine images.
  Each texel is redrawn by the parity of the texel it names. The pre-magnification
  build is the upper strip and the active build the lower one.
  - Board G: 8× unflipped and 6× flipped at offsets 0, 0.625, 1 and 3.
  - Board H: 32× strips at offsets 0, 0.625, 1, 2 and 8.
  - Board J: flipped leading texels at width × M of 2048 × 8, 4096 × 4, 4096 × 8,
    8192 × 4 and 8192 × 8.

### Limitations (round 3)

- **Hardware:** one GPU (Apple M5 Pro) and one backend (Metal), headless only.
- **Model inferences:** the f32 tolerances in the model and the 2^14 explanation
  are inferred from measurements, not from hardware documentation.
- **Coverage:** the large-coordinate bound is measured at widths 512 to 8192 and
  magnifications 1× to 32×. Perspective cameras and textures wider than 8192
  texels are untested.
- **Gates:** no phase gate is approved or claimed. The large-coordinate limit, the
  exact display option, the Inspector and native review remain open.

---

## Round 2: corrected nearest sampling

### Verdict

- **Accepted, scoped: the duplicated-texel defect is fixed.** On the corrected build
  (source `c2c7758`), a 7 × 7 unique-texel ruler shows every texel exactly once on
  tie placements:
  - 40 of 40 cases pass on the new orientation board F. The pre-ties build passes
    24 of 40.
  - Board F covers the eight symmetries of the square, a 180° transform and a mirror
    parent, on tie and aligned placements, at 1× and 3×.
  - Board A's round-one ruler station passes in all ten placements, before and after
    the taller-sheet reimport.
- **Resized sheets no longer change nearest pixels.** Reimporting the same texels in
  a 128 × 80 sheet changes no nearest pixel on any board. One linear pixel moves by
  one level, from the unchanged linear path. The pre-ties build changed 6,442 nearest
  pixels in the same test.
- **The engine follows the documented rule.** The reference model now implements it:
  region-local texels, a 1/1024-texel band toward increasing screen coordinate on the
  dominant derivative axis, and an integer fetch.
  - 324 corrected frames match it with 0 unexplained pixels.
  - Only 19 pixels differ at positions the model marks ambiguous. Round one had 3,372.
- **Concrete failure: a new near-boundary regression at 4× and above.** Magnified
  nearest sprites whose edge lies one or two 1/256 px raster steps past a pixel centre,
  toward increasing screen coordinate, render their first texel one pixel narrow and
  their last one pixel wide. At 8× that gives widths of 7, 8, 8, 8, 8, 8 and 9 px. The
  pre-ties build drew these cases correctly. It affects 15 of 140 offset-sweep cells.
  See "Request for Astra: scale the tie band".
- **Nothing else changed.** Pre-ties and corrected runs of the same scene differ only
  at former ties:

  | Files compared | Byte-identical | Changed | Changed pixels |
  | --- | --- | --- | --- |
  | 334 | 296 | 38 | 62,365 |

  Every changed pixel was a nearest texel tie under the pre-ties model. Linear
  sampling, tone, alpha, painter order, depth and all 272 live frames are unchanged.
- **Repeatability holds.** Two fresh corrected runs are byte-identical across 635
  PNGs. In both, every source-free frame equals its with-source frame.

This scoped verdict covers headless engine rendering on one Apple GPU. It approves no
phase gate and claims no native, Inspector or hosted-CI result.

### Decisions acknowledged

- The undersized-reimport diagnostics request stays with Astra.
- The neutral curve and linear-light blending stay for this follow-up. An exact
  display option can be per-camera when implemented.
- Optional sprite-grid snapping is separate work.
- The pixel-camera near-tie follower defect is separate. It is neither tested nor
  claimed fixed here.
- The Sprite Inspector and native review still follow packet 0033.
- No Cargo, Rust, shader or `editor/ui` file changed. I did not commit Astra's
  updated `brief.md`, so it remains as delivered in the working tree.

### Model, effort and provenance

- **Model and effort:** Claude Opus 5.5 (`claude-opus-5-5`) at Max effort, in the same
  resumed ACP runner session as round one. The runner confirms both settings before
  sending the prompt; I cannot inspect that handshake from inside the session.
- **Worktree:** `5d662f6`.

| Build | Source | sha256 |
| --- | --- | --- |
| Corrected, `incant_headless` | `c2c7758c14c37d36aa13e322aa2c87e63ce489ad` | `fd65133fc98a0c6d533f23d9583a820271858eecdb0849b44f8b7e996a5e072b` |
| Pre-ties, `pre-ties-incant_headless` | `39e5c38107451ba44a02ef7b80f27bef11d4141d` | `848b8294854add3bc672e6ab97e1c3dc1f5f39a9c5b1c234e61e5edace5820ba` |

Each build matched its metadata before and after all 126 invocations of each run.

### Changed paths (round 2)

All changes are in `handoffs/0032-sprite-rendering/`. Round-one evidence, screenshots
and result sections are untouched. The round-one tools were extended in place. Run
on the pre-ties build, the extended helper reproduces all 623 round-one images byte
for byte, so the new boards disturb nothing.
- `tools/sprite_lookdev.py` gains a `--binary pre-ties` option. It also adds boards F
  and G and authors board G's offsets exactly.
- `tools/reference.py` implements the corrected rule and keeps the pre-ties
  hardware rule for older runs. Three precision refinements model the GPU, not the
  nearest rule:
  - vertex positions round to f32, as in the vertex buffer;
  - half steps of the 1/256 px raster grid round up, as measured;
  - triangles clipped at the frame edge get a tolerance of two raster steps.
  
  With these, the round-one frames still match with 0 unexplained pixels.
- `tools/analyse.py` takes the rule from the run's binary, decodes boards F and G,
  checks every resized-sheet frame and adds a model-only counterfactual.
- `tools/compare_runs.py` and `tools/followup_figures.py` are new.
- `tools/sprite_art.py` only appends label words. The sheet and all earlier label
  regions are byte-identical.
- `tools/test_reference.py` adds three tie-band tests.
- `evidence/followup/` holds both analyses, the before/after comparison, the round-one
  re-analysis, three helper records, the scene and image hashes.
- `screenshots/followup/` holds 16 images, listed under "Round-two frame set".

### Commands and results (round 2)

```sh
T=handoffs/0032-sprite-rendering/tools; F=artifacts/0032-sprite-rendering/followup
python3 -I $T/sprite_lookdev.py $F/c1
python3 -I $T/sprite_lookdev.py $F/c2
python3 -I $T/sprite_lookdev.py $F/p1 --binary pre-ties
python3 -I $T/analyse.py $F/c1 $F/final/analysis-c1.json $F/c2
python3 -I $T/analyse.py $F/p1 $F/final/analysis-p1.json
python3 -I $T/analyse.py artifacts/0032-sprite-rendering/engine-r1 $F/round1-reanalysis.json
python3 -I $T/compare_runs.py $F/p1 $F/c1 $F/final/compare-p1-c1.json
python3 -I $T/followup_figures.py $F/p1 $F/c1 $F/figures-1
python3 -I $T/test_reference.py                     # 10 tests OK
python3 -m unittest discover -s tools/tests          # 10 tests OK
cd editor/ui && npx tsc -b --noEmit && npx vitest run && npm run build
cd handoffs/0032-sprite-rendering && shasum -a 256 -c evidence/followup/screenshots.sha256   # 16 OK
```

Each run printed:
```
binary_unchanged: true, engine_calls: 126, authored_unchanged: true,
screenshot_repeats_equal: true, screenshots: 29, source_free_shots: 10,
play_repeats_equal: all 8 true, play_frames: 34 each
```

The UI checks are an unchanged baseline. Strict typecheck reports 0 errors, all 420
tests in 16 files pass, and the build succeeds.

Each invocation stays within budget. The largest frame is 2.5 MiB raw. A play capture
writes 34 frames, 7.5 MiB raw. A whole run writes 635 frames, 228 MiB raw, over 126
invocations. Nothing is overwritten.

Not run by me: Cargo, Clippy, the Rust and GPU suites, native captures and hosted CI.
Astra reports 325 Rust tests, Clippy, 53 GPU checks and 10 tool tests passing.

### New boards

- **Board F, orientations:** a 7 × 7 ruler in ten orientations, numbered 0 to 9:
  none, flip X, flip Y, both flips, 90°, 270°, 90° with flip X, 90° with flip Y, a
  180° transform and a mirror parent. Rows place the centred pivot on a grid point
  (ties) or a pixel centre (aligned), at 1× and 3×.
- **Board G, offset sweep:** the ruler at magnifications 1, 2, 3, 4, 5, 6 and 8,
  unflipped and flipped. Its left and top edges start on a pixel centre and move
  toward increasing screen coordinate by an offset, in 1/256 px:
  −1, 0, 0.125, 0.25, 0.625, 1, 2, 3, 4 or 6.
  - The raster snaps to 1/256 px, so 0.125 and 0.25 snap to an exact tie and 0.625
    snaps to one step.
  - An exact half step, 0.5, was replaced by 0.625. Its snapping direction is itself
    a knife edge that f32 and f64 arithmetic resolve differently.
- Both boards sit near x = 0, far from the other boards, so f32 positions resolve
  every offset.

### Offset sweep, measured from engine pixels

Each cell lists the offsets, in 1/256 px, whose texels do not all span exactly M
pixels. All other cells are uniform.

| Magnification | Pre-ties, unflipped | Pre-ties, flipped | Corrected, unflipped | Corrected, flipped |
| --- | --- | --- | --- | --- |
| 1× | none | 0, 0.125, 0.25 | none | none |
| 2× | none | 0, 0.125, 0.25 | none | none |
| 3× | 0.25 | 0.25 | none | none |
| 4× | none | 0, 0.125, 0.25 | 0.625, 1 | none |
| 5× | 0.25 | 0, 0.125, 0.25 | 0.625, 1 | 0.625, 1 |
| 6× | 0, 0.25 | 0, 0.125, 0.25 | 0.625, 1 | 0.625, 1 |
| 8× | none | 0, 0.125, 0.25 | 0.625, 1, 2 | 0.625, 1 |

The corrected build fixes all 23 tie failures and introduces 15 near-boundary ones.
Widths in the corrected failures follow one pattern:

| Case | Corrected widths, px | Pre-ties widths, px |
| --- | --- | --- |
| 4×, unflipped, 0.625 | 3, 4, 4, 4, 4, 4, 5 | 4 each |
| 5×, flipped, 1 | 4, 5, 5, 5, 5, 5, 6 | 5 each |
| 8×, unflipped, 2 | 7, 8, 8, 8, 8, 8, 9 | 8 each |

Rows behave the same way as columns.

### Request for Astra: scale the tie band (regression, low to medium)

**Why it happens.** The band is fixed at 1/1024 of a texel, so on screen it spans
M/1024 px, while the raster snaps edges in 1/256 px steps. Once M exceeds 4, the band
is wider than one raster step. An edge snapped one step past a pixel centre then puts
the next texel boundary just after that centre. Instead of rendering the earlier
texel there, the band assigns the pixel to the next one. At exactly 4×, the band edge
coincides with one step: unflipped sprites fail and flipped ones do not.

**Impact.** Pixel art drawn at 1× to 3× of the camera density is unaffected. The
regression needs nearest sprites magnified at least 4× whose edge falls within M/4
raster steps after a pixel centre. Moving magnified sprites can show a one-pixel
flicker of their first and last texel.

**Proposed fix.** In the shader's nearest branch, replace `sign(direction)*(1.0/1024.0)`
with `direction*(1.0/1024.0)`. The band then becomes 1/1024 px along the dominant
screen axis:
- at 1:1 it is identical to today;
- at M× it is 1/(1024 M) texel, a quarter of a raster step;
- under minification it stays 1/1024 px.

The reference model with this change predicts all 40 orientation cases and all 140
sweep cells uniform; the shipped rule predicts exactly the 15 failing cells the
engine shows. That is model evidence only, and Astra owns integration and
verification. Very large magnification, or regions near 8,192 texels, may need a
lower bound on the band for f32 noise.

**Regression test.** Use the board G sweep: magnifications 1 to 8, offsets of 0, 0.625,
1, 2 and 3/256 px, both orientations. Every texel must span exactly M pixels. Keep
the existing eight-orientation and resized-sheet test.

### Other checks (round 2)

- **Aligned placements:** all 20 aligned cases on board F pass on both builds. In
  the before/after comparison every changed pixel was a former tie.
- **Nine-slice:** only former ties change. In the pixel-perfect 1× frame they lie in
  the simple panel stretched to 72 × 44, the 12° rotated panel and the density-32
  border. The 3× orthographic and rolled frames add tie pixels across the other
  panels. No seams or cracks appear in any of eight checked frames.
- **Fractional and rolled views:** the 3.75× and 7° rolled frames match the model.
  Their only changes from the pre-ties build are former ties: 2 to 120 pixels per
  frame.
- **Linear and tone:** no linear-only, tone, alpha or gray-ramp pixel changes. The
  gray ramp still shows 0 to 234 exactly and 252 to 255 as 243. The straight-alpha
  counterfactual still tints 1,128 pixels while the engine tints none.
- **Live:** all 272 frames are byte-identical between builds. The Wisp's box stays at
  (98, 113) to (109, 127) in every frame.

### Appearance review (round 2)

I viewed the engine crops and the decoded views.
- **Decoded tie rows.** Before, 8 of 10 rulers at 1× and 8 of 10 at 3× show broken
  checkers, meaning texels repeat or vanish. After, all 20 are clean checkers, and
  quarter turns and diagonal reflections read correctly.
- **Nine-slice.** The panel and density-32 frames look equally clean before and after.
  The fix only changes which texel survives on tie rows of non-integer stretches.
- **Sky strip.** One gradient row shifts at a tie row and reads the same.
- **Magnified rulers.** The 4× to 8× rulers in the corrected build show the narrow
  first and wide last texel at one and two steps. This is visible when zoomed and
  subtle at 1×.
- **Unchanged scenes.** The showcase, live follow and flipbook are pixel-identical to
  the pre-ties build.

### Round-two frame set

In `screenshots/followup/`:
- `engine-board-f-orientations-after-960x540.png` and
  `engine-board-g-offset-sweep-after-1024x640.png` are unedited corrected frames.
- These `*-before-after-zoomN.png` files are nearest-neighbour enlargements, pre-ties
  on the left and corrected on the right:
  - the board A ruler station, from the 64 px and 80 px sheets;
  - board F's tie rows at 1× and 3×;
  - the nine-slice stretch and density panels;
  - the nearest sky strip.
- `board-g-4x-to-8x-before-zoom2.png` and `board-g-4x-to-8x-after-zoom2.png` enlarge
  offset columns 0 to 6.
- The `decoded-*.png` files are diagnostics derived from engine pixels, not engine
  images. Each ruler pixel is redrawn as a two-tone checker by the texel it names,
  so a repeated or dropped texel breaks the checker. They cover board F's tie rows,
  board G's 4× to 8× rows, and offsets 0, 0.625, 1 and 3 at 8× unflipped and 6×
  flipped.

### Limitations (round 2)

- **Hardware:** one GPU (Apple M5 Pro) and one OS, headless only.
- **Precision modelling:** the f32 vertices, half-up snapping and clip tolerance are
  inferred from measurements on this GPU, not from published hardware specifications.
- **Remaining ambiguity:** the 19 ambiguous mismatches lie on board G's band edges and
  on clipped triangles in rolled frames. They are reported, not judged.
- **Untested:** the pixel-camera near-tie follower defect.
- **Gates:** no phase gate is approved or claimed. The tie-band regression, the
  diagnostics request, the display option, the Inspector and native review remain
  open.

---

## Status

Engine art and rendered review are complete for this round. The Sprite Inspector is
deferred until packet 0033 is integrated, as the packet directs.

- **Engine sprite rendering: pass for aligned content, with one renderer defect.**
  Every one of 318 real engine frames matches an independent model of the documented
  contract, with zero unexplained pixels. Regions, pivots, flips, asymmetric nine-slice
  borders, painter order, alpha, depth, sampling, seams, tone, reimport and source-free
  loading all behave as documented. Fresh from-scratch runs are byte-identical.
- **Defect for Astra: nearest-sampling ties.** When texel boundaries fall exactly on
  pixel centres, the texel shown depends on floating-point rounding of normalized sheet
  UVs. A 7 px sprite whose centred pivot sits on a pixel corner shows its last texel
  twice and drops its first when flipped, rotated 180 degrees or mirrored. Reimporting a
  taller sheet with identical texels changed which texels unflipped sprites show. See
  request 1.
- **Diagnostics request.** A reimport that shrinks a texture below its sprite regions is
  accepted silently. Every later render then fails with a generic message that names no
  entity, region or size. See request 2.
- **Look-dev recommendation.** Unlit sprite colours pass through the 3D tone shoulder.
  Grays up to sRGB 234 show exactly, but 252 to 255 all show as 243. Pure primaries gain
  a 30-level floor in their zero channels. See request 3.
- **Sprite Inspector: not started.** No `editor/ui` file was edited. The dependency on
  0033 and the data the Inspector will need are recorded below.

This verdict covers headless engine rendering on one Apple GPU. It does not approve a
phase gate, native editor presentation, atlases, tilemaps, sprite animation libraries,
sprite lighting or shadows, or 2D physics.

## Model, effort and transport

- **Model:** Claude Opus 5.5, model ID `claude-opus-5-5`, as stated by the session.
- **Thinking effort:** Max, the session's configured reasoning effort.
- **Transport:** ACP through `tools/handoff/main.py` with the pinned
  `@agentclientprotocol/claude-agent-acp` 0.88.0 adapter and
  `@anthropic-ai/claude-code` 2.1.295. The prompt arrived in that runner's exact
  template. The runner sends it only after ACP confirms model "Opus 5.5" and effort
  "max" (`configure_model_and_effort`). I cannot inspect the ACP handshake from inside
  the session. No model or effort substitution occurred.

## Director feedback acknowledged

The packet's priority revision says engine art and review come first and the UI waits
for the full redesign in 0033. I applied it:
- no `editor/ui` edits, so nothing competes with the redesign;
- original art, real engine scenes, helpers, measurements and this result only;
- the Sprite Inspector dependency is recorded under "Deferred: read-only Sprite
  Inspector".

The worktree was clean at `0d29968` with only `brief.md` in this packet folder. No
files from an interrupted attempt existed.

## Provenance

| Item | Value |
| --- | --- |
| Worktree base | `0d29968` |
| Binary source commit | `39e5c38107451ba44a02ef7b80f27bef11d4141d` (`binary.json`) |
| Headless binary | `artifacts/tools/incant_headless` |
| Binary sha256 | `848b8294854add3bc672e6ab97e1c3dc1f5f39a9c5b1c234e61e5edace5820ba` |
| Binary check | Matched `binary.json` before and after every engine invocation |
| GPU adapter | Apple M5 Pro, reported by the screenshot command |
| Host | macOS 26.6 (25G72) |
| Tooling | Python 3.14.8, Node 22.23.3, pinned TypeScript 5.9.3 and @swc/core 1.16.13 |

## Changed paths

All changes are inside `handoffs/0032-sprite-rendering/`.

Tools, standard-library Python:
- `tools/sprite_art.py` generates all original art from code.
- `tools/sprite_lookdev.py` builds the project through public commands and renders it.
- `tools/reference.py` is an independent CPU model of the documented contract.
- `tools/analyse.py` measures every frame from PNG bytes.
- `tools/crops.py` copies the frame set and makes labelled zoom crops.
- `tools/test_reference.py` holds seven self-checks for the reference model.
- `tools/pngio.py` reads and writes PNG files.

Evidence:
- `evidence/analysis-r1.json`, including the cross-run comparison with run 2.
- `evidence/helper-r1.json` and `evidence/helper-r2.json`.
- `evidence/scene-r1.json`, the authored entities, cameras and regions.
- `evidence/live_e.generated.ts`, one of the eight strict-TypeScript behaviours.
- `evidence/art/`, the source images and `regions.json`.
- `evidence/screenshots.sha256`, hashes of every committed image.

Screenshots:
- `screenshots/engine/`: ten unedited engine frames and eleven zoom crops.
- `screenshots/art/`: two previews of the source art. These are not engine frames.

Ignored and not committed, under `artifacts/0032-sprite-rendering/`:
- `engine-r1` to `engine-r3`, the final from-scratch runs with all raw frames.
- `engine-p1` to `engine-p3`, superseded probes made before the art and checks were final.
- `smoke-1`, scratch tools, analysis files and logs.

No Rust, SDK, transport, Cargo, CI or account file changed. No Cargo command ran.

## Commands and results

```sh
npm ci --ignore-scripts                                   # pinned lockfile, 0 vulnerabilities
python3 -I handoffs/0032-sprite-rendering/tools/sprite_lookdev.py artifacts/0032-sprite-rendering/engine-r1
python3 -I handoffs/0032-sprite-rendering/tools/sprite_lookdev.py artifacts/0032-sprite-rendering/engine-r2
python3 -I handoffs/0032-sprite-rendering/tools/sprite_lookdev.py artifacts/0032-sprite-rendering/engine-r3
python3 -I handoffs/0032-sprite-rendering/tools/analyse.py \
  artifacts/0032-sprite-rendering/engine-r1 artifacts/0032-sprite-rendering/analysis-r1.json \
  artifacts/0032-sprite-rendering/engine-r2                 # about 150 s
python3 -I handoffs/0032-sprite-rendering/tools/crops.py \
  artifacts/0032-sprite-rendering/engine-r1 artifacts/0032-sprite-rendering/crops-r1
python3 -I handoffs/0032-sprite-rendering/tools/test_reference.py   # 7 tests OK
python3 -m unittest discover -s tools/tests                          # 7 tests OK
cd editor/ui && npx tsc -b --noEmit && npx vitest run && npm run build
cd handoffs/0032-sprite-rendering && shasum -a 256 -c evidence/screenshots.sha256   # 31 OK
```

Each helper run printed:
```
binary_unchanged: true, engine_calls: 114, authored_unchanged: true,
screenshot_repeats_equal: true, screenshots: 25, source_free_shots: 8,
play_repeats_equal: all 8 true, play_frames: 34 each
```

The UI checks are an unchanged baseline, not Inspector evidence. Strict typecheck
reports 0 errors, all 420 tests in 16 files pass, and the build succeeds.

Not run by me:
- Cargo, Clippy, the Rust and GPU test suites and portable checks;
- native editor captures, native WebKit and hosted CI;
- Windows, Linux, Android or iOS GPUs.

## Original art

All art is drawn from code in `tools/sprite_art.py` and regenerates byte for byte.
- **Wisp:** a 16 px rendition of the project's own mascot. It was rasterized from the
  vector trace kept in handoff 0004 at 50% coverage, then hand-finished with a deep
  purple outline, upper-left light and a glint. Four idle frames form the flipbook:
  rest, sway left, blink and sway right. The flame and eyes make it asymmetric on both
  axes. A teal palette twin exists for the reimport test.
- **Dialog panel:** 24 × 24 px with asymmetric borders of 3, 6, 4 and 8 px (left,
  right, top, bottom). Each edge is uniform along its stretch axis. Each corner has a
  unique mark: red, green, blue and a gold page tab.
- **Measurement art:** an orientation key, a 7 × 7 ruler whose every texel has a
  unique colour, a warm orb and a white aura with magenta RGB hidden under zero alpha,
  a crate inside an opaque magenta guard ring, and a sky strip beside a magenta guard.
- **Scenery:** two tiling grass tiles, a soil tile, stars, a crescent moon and a fog band.
- **Swatches:** 16 tone swatches, also as a 16-bit PNG twin, a 16 × 16 ramp holding
  every 8-bit gray once, and white and black alpha ramps.
- **Text:** a 3 × 5 pixel font for labels, and a gem badge in teal, orange and an
  undersized 8 × 8 variant.

## Engine scene

Everything is authored through public commands:
1. `init` creates the project.
2. Texture sources are copied into it and registered with `import --texture-usage
   color`. The occluder glTF is registered with `import`.
3. One `rpc command.execute` transaction with provenance creates all 315 entities:
   290 sprites, three parent transforms, an emissive glTF occluder and 21 cameras. The
   engine reports no diagnostic stand-in geometry for the parents.
4. `project.save` and `validate` follow.

No project or journal file is edited. Five 320 × 180 boards at 16 px per metre expose:

| Board | Content |
| --- | --- |
| A | source strip and frames, guarded crate, five pivots with origin markers, rulers on grid and aligned pivots, flips, mirror parent, rotations, tints |
| B | nine-slice native, 72 × 44 next to a simple stretch, three compressed targets, flips, densities 8 and 32, linear, rotated 12° |
| C | layer, order, ID tie, Z ignored, interleaved textures, 50% overlaps over dark, light and empty backdrop, alpha ramps, opaque-mesh depth, 0–255 gray ramp |
| D | 4× nearest and linear orbs, linear orb over a light plate, 2× Wisp, rotated crates, two 18-tile rows, tone swatches, sky strip, rotated tile row |
| E | showcase: sky gradient, stars, moon, fog, grass and soil, crates, Wisp with aura, gem, dialog with text |

Cameras:
- every board has a pixel-perfect camera at 16 px per metre and 320 × 180, captured
  at 320 × 180 and 960 × 540;
- every board has an orthographic camera at an exact 3× (vertical size 11.25 m at
  960 × 540);
- every board has an orthographic camera at a fractional 3.75× with a sub-pixel offset;
- boards B, D and E have a 7° rolled orthographic camera;
- board A also has an odd 319 × 179 camera and one offset by +0.6 and +0.2 px.

Live runs use eight strict-TypeScript behaviours with ordinary `set_component` commands
and no source files. Each moves the pixel-perfect camera and the Wisp together by
1/8 px per tick in one of eight directions, for 33 ticks. The Wisp's source region
advances to the next idle frame every 4 ticks, which makes a flipbook. Every tick is
captured, twice per direction.

## Measured results

All values come from `evidence/analysis-r1.json`, measured from PNG bytes.

**Contract model.** `tools/reference.py` predicts each output pixel from the published
contract. It snaps vertices to 1/256 px, applies the top-left fill rule and premultiplies
before filtering. It also models f16 blending, the tone curve and the backdrop. It
reproduces `incant_2d`'s unit-test vectors and the tone values stated in the shader.
Where a pixel centre lies exactly on a sprite edge, or a nearest sample lies exactly on
a boundary between different texels, it marks the pixel ambiguous instead of guessing.

| Frames compared | Count | Unexplained pixels |
| --- | --- | --- |
| Static screenshots | 25 | 0 |
| Reimport screenshots | 21 | 0 |
| Live frames, 8 directions × 34 | 272 | 0 |

Static frames hold 9,733,901 pixels. Outside ambiguous pixels the differences are:

| Sampling | Identical | 1 level | 2 levels |
| --- | --- | --- | --- |
| Nearest or uncovered | 7,181,024 | 8,905 | 0 |
| Linear | 2,210,412 | 313,136 | 4 |

20,420 static pixels are ambiguous and 1,812 of them differ from the model's choice.
All 1,812 lie on non-integer stretches or half-pixel placements:

| Sprite | Pixels |
| --- | --- |
| Simple panel stretched 24 to 72 × 44 | 772 |
| Nine-slice at density 32 | 367 |
| Rulers on a grid-point pivot | 372 |
| Sky strip stretched 32 to 38 px | 140 |
| Nine-slice 44 × 9 | 136 |
| Nine-slice 8 × 10 | 23 |
| Board edge and rotated crate | 2 |

**Repeatability.**
- Within each run, all 25 screenshot pairs and all 8 play pairs are hash-equal.
- Runs 1 and 2 were built from scratch and match byte for byte across all 623 PNGs.
- A third run with the final committed tools matches run 1 across all 631 images,
  which adds the 8 generated source images. Its analysis is identical, apart from the
  random scene ID that `init` assigns, which appears inside one rejection message.
- Rendering and playback leave the project and journal bytes unchanged.

**Pixel-perfect framing.**
- All five 960 × 540 frames are exact 3× replications of their 320 × 180 frames.
- The 319 × 179 frame equals the base frame without its first column and first row.
  57,099 of 57,101 pixels are identical, and 2 linear pixels differ by one level.
- A camera offset of +0.6 and +0.2 px snaps one pixel to the right, so the frame equals
  the base frame moved one pixel left. 57,419 of 57,420 pixels are identical, and one
  linear pixel differs by one level.

**Orientation.** Every pair below differs in 0 pixels:
- flip X against a horizontal mirror, and flip Y against a vertical mirror;
- both flips against a 180° turn, and a 180° transform against both flips;
- a mirror parent with scale −1 against flip X;
- a 90° transform against a counter-clockwise quarter turn on screen;
- the nine-slice flips against mirrors of the unflipped panel;
- the 24 × 24 nine-slice against the simple 24 × 24 panel.

**Painter order and depth.** All 13 probes pass:
- layer beats order, order beats ID, and equal values draw the larger ID last;
- a far sprite on layer 1 covers a near sprite on layer 0;
- interleaved key and badge textures keep their order;
- the opaque quad hides sprites behind it, including a layer-100 sprite;
- a sprite in front of the quad draws over it.

**Seams.** No background pixel appears between tiles or nine-slice cells. That holds in
all eight frames checked: pixel-perfect B and D, and exact 3×, fractional and rolled
orthographic B and D.

**Filtering fringe.** The 4× linear orbs hide magenta RGB under zero alpha.
- **Engine:** no pixel moves toward magenta by 3 levels or more, and the worst shift is
  1 level. No pixel over the light plate is darker than both the plate and the orb core.
- **Counterfactual:** the same frame with straight-alpha filtering would tint 1,128 pixels,
  by up to 35 levels. The premultiplied path therefore prevents a measurable fringe.

**Bleeding.** No frame contains magenta-like pixels. The model never samples outside a
region, and all guarded regions matched it. That covers the crate ring, the sky guard,
the aura between two guards and the panel guards.

**Live follow and flipbook.**
- Frame n shows the state after n scripted ticks.
- In all eight directions and all 272 frames, the Wisp's box stays at (98, 113) to
  (109, 127).
- The background steps one pixel only on camera snaps.
- The idle frame changes every 4 ticks.
- This confirms the corrected half-pixel camera ties for a followed sprite.

**Ordinary orthographic at exact 3× against pixel-perfect 3×.** Pixels differ only on
linear, rotated, half-pixel or stretched sprites:

| Board | Differing pixels |
| --- | --- |
| A | 8,054 |
| B | 17,624 |
| C | 4,764 |
| D | 57,442 |
| E | 102,286 |

The by-sprite lists in the analysis contain no aligned nearest sprite.

**Reimport through `import`.** Every accepted step keeps the asset ID.

| Step | Outcome |
| --- | --- |
| Teal sheet | New fingerprint. Frames match the model with teal texels. Only pixels drawn from Wisp regions change, and board B is unchanged. |
| Orange badge | Frames match the model. |
| 8 × 8 badge | Import exits 0. Every screenshot then exits 1 with no image, including board A, which never draws the badge. |
| Badge restored | Original fingerprint. Board C equals its earlier frame byte for byte. |
| Sheet as linear | Rejected with exit 1 and "sprite requires a color texture asset" per sprite. Project and journal unchanged. |
| Sheet restored | All five boards and orthographic E equal the original frames byte for byte. |
| Taller sheet, 128 × 80, same texels and regions | Frames match the model, but 43, 213 and 20 pixels change on boards A, B and D. All are ambiguous ties, except one linear pixel that moves by one level. |
| Restored again | Board E equals the original byte for byte. |

The 8 × 8 badge failure reads:
```
Error: "sprite source region must be nonempty and fit inside a texture of at most 8192 pixels per axis"
```

**Source-free.** Removing all four PNG sources and the glTF and bin leaves eight frames
byte-identical to the frames rendered with sources present.

**Tone.** Values come from 8-bit swatches. The 16-bit PNG twin gives identical results
for all 16 swatches.

| Authored sRGB | Shown | Shown with 0.5 tint |
| --- | --- | --- |
| 128 gray | 128 | 92 |
| 200 gray | 200 | 146 |
| 231 and 232 gray | unchanged | 170 |
| 240 gray | 238 | 176 |
| 255 white | 243 | 188 |
| 255, 0, 0 | 243, 30, 30 | 188, 0, 0 |
| 255, 214, 128 | 243, 205, 125 | 188, 157, 92 |
| 240, 200, 40 | 238, 198, 42 | 176, 146, 26 |
| 92, 60, 212 (Wisp body) | unchanged | 65, 42, 155 |
| 231, 120, 20 | unchanged | 170, 86, 11 |

The gray ramp measures every 8-bit value directly:
- grays 0 to 234 show exactly;
- the 21 inputs from 235 to 255 share only 10 outputs;
- 240 shows as 238, 246 as 240, and 252, 253, 254 and 255 all show as 243.

So near-white shades merge. The tint is a linear-light multiplier, so 0.5 turns 128
into 92, not 64.

**Alpha.** Each layer below is 50% over the board background. The engine matches the
linear-light contract at every probe. A paint program blending in sRGB space would
show darker results.

| Overlap | Engine | sRGB-space blend |
| --- | --- | --- |
| Red | 158, 48, 58 | 119, 43, 56 |
| Red and green | 124, 140, 90 | 96, 114, 84 |
| Red, green and blue | 104, 130, 173 | 86, 117, 153 |

Over 128 gray at alpha 128, white shows 205 against a paint program's 192, and black
shows 92 against 64.

**Budget.** Each engine invocation stays within the packet's limits. A screenshot writes
one frame, at most 960 × 540 or 2.0 MiB raw. A play capture writes 34 frames, or 7.5 MiB
raw. A whole helper run makes 114 invocations and writes 623 frames, 209 MiB raw in
total. Output directories and files are never overwritten.

## Appearance review

I viewed the frames themselves as well as the statistics.

- **Pixel-perfect frames are what a pixel artist expects.**
  - Edges are hard and texel blocks are uniform at 3×.
  - There is no shimmer while following.
  - Region selection is clean, with no neighbour bleed.
- **Nine-slice reads correctly.**
  - Corners stay crisp at their source density.
  - Edges stretch only along their length, so the gold tab and gems never distort.
  - The simple stretch beside it shows the corner distortion the mode avoids.
  - Compressed targets keep proportional corners.
  - Density 32 on a 16 px per metre camera drops border texels unevenly. Authors should
    keep border density at or below the camera density for pixel art.
- **Ordinary orthographic at 3.75× behaves as expected.** Nearest pixel art shows
  uneven 3 and 4 px texel widths, visible on the Wisp's 1 px outline. That is correct
  behaviour, and a reason to use pixel-perfect cameras for pixel art.
- **The 7° roll gives clean stair steps.** There are no cracks in tile rows or panels.
- **Linear sampling suits only smooth art.** On pixel art, as on the 2× Wisp and the
  linear panel, it reads soft. Nearest is the right default. Linear orbs, aura and sky
  are smooth, with no halo, dark ring or colour fringe.
- **Painter order and depth read naturally.** Overlaps never flicker between runs.
- **Highlights are slightly muted.** The tone shoulder dims full white to 243, so eye
  whites, star centres and the white aura core are slightly muted. Pure primaries lose
  saturation. This is fine for 3D look-dev but costs palette fidelity in 2D (request 3).
- **Translucency is lighter than in paint programs.** The aura, fog and 50% overlaps
  match the linear-light contract (open question 2).
- **The showcase reads as one scene.** The night sky, moon halo and fog give depth. The
  purple aura and teal gem glow are tintable from one white texture. The dialog text is
  legible at 1×.

Committed frame set in `screenshots/engine/`:
- `board-a` to `board-e`: pixel-perfect 960 × 540 frames.
- `board-e-showcase-ortho-3.75x`, `board-d-ortho-3.75x` and `board-b-nine-slice-ortho-roll7`:
  ordinary orthographic frames.
- `reimport-teal-sheet-board-e` and `reimport-orange-badge-board-e`: 320 × 180 frames.
- Zoom crops, each a nearest-neighbour enlargement with `-zoomN` in its name:
  - the ruler station, with the 64 px sheet beside the 80 px sheet;
  - nine-slice flips and small targets;
  - the linear orb fringe;
  - tone swatches, the gray ramp, alpha and depth;
  - live follow at ticks 3 to 5 and flipbook frames 0, 4, 8 and 12.
- `seams-rolled-tiles-ortho-crop`: an unscaled crop of the rolled orthographic frame.

## Requests for Astra

### 1. Nearest-sampling ties depend on rounding and sheet size (defect, medium for pixel art)

**What happens.** The ruler gives every texel a unique colour, so each rendered pixel
names its texel. Its edges lie on pixel centres when an odd-sized sprite uses a centred
pivot on a grid point.

| Placement | Columns, left to right | Rows, top to bottom |
| --- | --- | --- |
| Grid pivot, unflipped | 0 to 6, once each | 0 to 6, once each |
| Grid pivot, flip X | 6, 6, 5, 4, 3, 2, 1 | 0 to 6 |
| Grid pivot, flip Y | 0 to 6 | 6, 6, 5, 4, 3, 2, 1 |
| Grid pivot, 180° transform | 6, 6, 5, 4, 3, 2, 1 | 6, 6, 5, 4, 3, 2, 1 |
| Grid pivot, mirror parent | 6, 6, 5, 4, 3, 2, 1 | 0 to 6 |
| Pixel-centre pivot, every orientation | each texel once | each texel once |

After reimporting the same texels in a 128 × 80 sheet, the unflipped grid-pivot ruler
shows rows 0, 1, 2, 3, 4, 5, 5. Flip Y shows 6, 5, 5, 4, 3, 2, 1. The same reimport also
changed 213 pixels of nine-slice art on board B and 20 pixels of the sky strip on board D.
No texel or region changed.

**Why.** The shader samples normalized sheet UVs with the hardware nearest filter. At an
exact boundary, the chosen texel follows f32 interpolation and the sheet's dimensions.
Flips and mirrors reverse the texel direction, so an exact tie picks the texel on the
wrong side of the boundary. That repeats the outermost texel and drops the opposite one.

**Proposed fix.** This is a renderer change for Astra to integrate and verify. For
nearest sampling only:
1. Pass the region-local texel coordinate `t` after flips, in texels rather than
   normalized units.
2. In the fragment shader, pick
   `i = clamp(floor(t + e * (sign(dpdx(t)) + sign(dpdy(t)))), 0, size - 1)` per axis,
   with `e` about 1/1024 texel.
3. Fetch with `textureLoad` at the region origin plus `i`.

An exact tie then always picks the texel on the screen-right or screen-down side. That
matches the top-left fill rule, as the camera tie fix did. It is independent of flips,
mirrors, 180° turns and sheet size. Linear sampling keeps its current path.

**Regression test.** Render the 7 × 7 unique-texel ruler on a grid-point pivot in six
orientations: unflipped, flip X, flip Y, both, 180° and a mirror parent. Use sheets of
64 and 80 px height.
- Each case must show every texel exactly once, in screen order.
- Both sheet sizes must give identical frames.
- Aligned placements and integer scales must stay byte-identical to today.

A separate authoring option, pixel-grid snapping of sprite vertices on pixel-perfect
cameras, would avoid half-pixel placements altogether. It is open question 3, not part
of this fix.

### 2. Undersized reimports fail late and generically (diagnostics, low to medium)

A reimport that shrinks a texture below its sprites' regions commits without warning.
Every later render of the project then fails, including scenes that never draw that
sprite. The message names neither the entity, the texture, the region nor the actual
size, and its "at most 8192 pixels" wording points at the wrong limit. The failure is
explicit, so the contract holds.

Requests:
- name the entity, texture asset, region and cooked size in the render error;
- report, at import or `validate` time, the sprites whose regions no longer fit the new
  texture.

### 3. An exact display path for 2D palettes (look-dev recommendation)

Sprites share the 3D tone curve, which is the identity only below a 0.8 linear peak.
Measured on the gray ramp and swatches, 2D palettes above sRGB 234 therefore shift:
- 252 to 255 all become 243, so the top four grays are indistinguishable;
- 255, 0, 0 becomes 243, 30, 30;
- 240, 200, 40 becomes 238, 198, 42.

Recommendation: offer an authored display choice for 2D, such as a camera or project
setting of "neutral" (today's curve, kept as the default) or "clamp". "Clamp" would
replace the shoulder with a per-channel minimum of 1.0.

Acceptance: on a "clamp" camera, every 8-bit sRGB value of an opaque unlit sprite
round-trips exactly, and today's frames are unchanged on "neutral". The setting's level
and name are Astra's and the director's call.

## Deferred: read-only Sprite Inspector

**Dependency.** This work waits for packet 0033's Inspector redesign to be integrated.
It must then use that design's components and conventions. Nothing in `editor/ui`
changed in this round, and there are no Inspector captures or UI tests for sprites.

**Presentation notes for the next round**, drawn from this review:
- **Coordinate origins.** The component mixes two origins. The source region is in
  pixels from the top left, and the pivot is normalized from the bottom left. Each row
  must say which origin it uses, for example "Source, px from top-left" and "Pivot,
  0–1 from bottom-left".
- **Border order.** Nine-slice borders are stored as left, right, top, bottom. That is
  not CSS order, so every entry needs its own label.
- **Units.** Show border density in px/m. Show size in metres.
- **Tint.** Label tint "linear" and show a swatch. The tone and alpha results above
  explain why authors will otherwise misread it.
- **Mode rows.** Show borders and density only for nine-slice. Keep malformed values,
  engine diagnostics and the "Read-only" chip as the Camera Inspector does.

**Data binding needed from Astra.** The Inspector needs each texture asset's cooked
width and height. It can then show "Source 0, 0, 64 × 16 of 128 × 64" and flag a region
that no longer fits before a render fails. The project document does not store cooked
dimensions today.

## Limitations

- **Hardware.** One GPU and one operating system: Apple M5 Pro through the headless
  binary. No other GPU or platform was tested.
- **Native editor.** Not reviewed. Engine frames are not native Inspector acceptance.
- **Perspective cameras.** Sprites under perspective cameras were not reviewed. The
  model covers orthographic and pixel-perfect cameras only.
- **Minified sprites.** No mipmaps are used, so linear sprites drawn smaller than their
  texels will alias. Atlas packing and extruded mip levels remain open, as the spike states.
- **Not supplied, so not reviewed:**
  - transparent 3D interleaving;
  - atlas packing and tilemaps;
  - sprite animation libraries;
  - sprite lighting and shadows;
  - 2D physics.

  The flipbook here uses ordinary scripted component changes. It is not an animation
  library.
- **Model coverage.** The model's agreement covers the frames listed. Ambiguous pixels
  are reported, not judged.
- **Gate approval.** No phase gate is approved or claimed. Requests 1 and 2 and the
  Inspector are open.

## Open questions

1. Should 2D work get an exact "clamp" display path (request 3)? Should it be set per
   camera or per project?
2. Should translucent sprites stay in linear light, documented in the Inspector, or
   should 2D layers also offer display-space blending to match paint programs?
3. Is optional pixel-grid snapping of sprites on pixel-perfect cameras wanted as an
   authoring feature, beside the tie fix in request 1?
