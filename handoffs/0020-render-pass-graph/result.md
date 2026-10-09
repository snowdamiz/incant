# 0020 render-pass graph: rendered-pixel review result

## Final status

Final review complete. Final scoped visual verdict: **pass, with no open
pixel-evidence gaps.** Scheduling causes no visual regression. Both findings
from the initial review are resolved by Astra's test-only follow-up `69dfb32`,
integrated at `25f2129`. I verified both fixes by rerunning them in separate
processes.

This verdict does not approve a phase gate. It does not certify game, device or
GPU-timestamp performance. Shadows, post-effects and the full production graph
remain open. No new editor control or camera selector is claimed or reviewed.

Exact model for both rounds: Claude Opus 5.5, model ID `claude-opus-5-5`. No
model substitution occurred.

The report has two rounds:
- This final round reviews the eight final fixture captures and the revised tests.
- The initial review at `1494fda` is preserved below as history. A correction
  to one of its statements is recorded in this round.

## Final round

### Director feedback acknowledged

The revised packet says my initial review is integrated and no runtime or
shader code changed. I confirmed that the follow-up diff touches only two test
files, documentation, the evidence ledger and the brief. The packet also
corrects one statement from my initial report. I accept the correction below.
I made no test, shader or UI edits.

### Correction to the initial report

My initial finding 2 said that no pixel test submits clustered-light frames out
of order. That statement was too broad, and I withdraw it. The pre-existing
GPU test with the long name ending in `queued_resize_commands_keep_their_own_light_data`
encodes three clustered frames and submits them as second, first, third. It
compares pixels for both the preview camera and a selected camera.

That test uses the same lights in every frame and allows a one-step tolerance.
So it cannot show whether a frame shaded with another frame's cluster
membership. The new alternating-membership lifetime case covers that narrower
gap, with exact byte equality.

### Stable 96-light fixture: verified reproducible

The fixture now gives its 96 lights fixed IDs in x, y, z construction order.
The IDs are zero-padded decimal numbers, which are valid ULID strings and sort
in construction order. Light order therefore no longer depends on the clock or
on random bits.

I ran the unchanged test five times in separate processes. Every run produced
one hash per file, and those hashes equal Astra's two runs and the stability report.

| Capture | Hash prefix | Astra's two runs | My five runs |
| --- | --- | --- | --- |
| 513x385 and its oracle | `7f58c292c288` | identical | identical |
| 640x480 and its oracle | `ef4798ef0470` | identical | identical |

Clustered and oracle outputs remain byte-identical to each other. The stable
640x480 capture is byte-identical to the old baseline. The stable 513x385
capture differs from the old baseline by one green step at pixel 208, 102. This
is one of the variants I already observed in the initial round. It reflects a
different fixed light order, not a rendering change. The stable captures should
replace the old baseline for this fixture in future cross-commit comparisons.

Both stable captures look the same as before. Colored light pools blend across
the quad without banding, seams or cluster-boundary artifacts.

### Four-frame retained lifetime case: verified

All four captures keep exactly the same geometry as the initial round. Frames 2
and 3 have identical foreground masks with each other. Each frame shows only
its in-range light.

| Frame | Size | Foreground pixels | Geometry mask matches initial | Mean RGB | Dominant channel |
| --- | --- | --- | --- | --- | --- |
| 0, offset 0 | 321x193 | 1050 | yes | 141, 93, 93 | red in every pixel |
| 1, offset +2 | 480x270 | 2731 | yes | 91, 139, 91 | green in every pixel |
| 2, offset -2 | 321x193 | 812 | yes, same as initial offset -2 | 136, 95, 95 | red in every pixel |
| 3, offset -2 | 321x193 | 812 | yes, same as frame 2 | 95, 136, 95 | green in every pixel |

In the red frames the green and blue means are equal. In the green frames the
red and blue means are equal. So the light parked at 1000, 1000, 1000 adds
nothing, as its range of 20 requires. Frames 2 and 3 are near channel swaps of
each other, as expected from two lights at one position with mirrored colors.

The test now checks the failure it targets. Frames 2 and 3 share one cached
cluster buffer and differ only in which light bit reaches the geometry. They are
submitted in reverse, so frame 3's assignment runs before frame 2's. If frame 2
skipped its own assignment, it would shade with the green membership. The red
light would then be missing, and the exact comparison against the red
reference would fail. The test also asserts that the frame 2 and frame 3
references differ, so a blank or membership-blind result cannot pass.

The earlier checks remain in place: rejected viewport preparation, no retained
CPU frame or encoder, no surviving CPU model version after disposal and file
removal, and the final model-free frame that must be entirely `#141519`. My two
reruns passed and reproduced all four captures byte-for-byte.

### Final round commands and results

```sh
for i in 1 2 3 4 5; do
  INCANT_LIGHT_EVIDENCE=$PWD/artifacts/review-0020-final/run$i CARGO_TARGET_DIR=$PWD/target \
    cargo test -q -p incant_render --test model_gpu \
    distinct_colored_lights_preserve_energy -- --ignored
done
# 5/5 passed, one hash per file across all runs.

for i in 1 2; do
  INCANT_HDR_EVIDENCE=$PWD/artifacts/review-0020-final/graph$i CARGO_TARGET_DIR=$PWD/target \
    cargo test -q -p incant_render --lib queued_graph -- --ignored
done
# 2/2 passed. All four lifetime captures are byte-identical to graph-final.

(cd artifacts && python3 -I review-0020-scripts/lifetime.py)
(cd artifacts && python3 -I review-0020-scripts/pixdiff.py \
  graph-baseline/spatial-overflow-96-<size>.png graph-final/spatial-overflow-96-<size>.png)
```

Not rerun by me in this round: the combined 33-case GPU suite, Clippy, the UI
and ordinary Rust suites and the final native build. The brief says Astra is
rerunning those. Their results are not evidence in this report. Native imagery,
other captures and runtime code are unchanged, so the initial native and
baseline review still applies.

### Final round screenshots reviewed

- `artifacts/graph-final/graph-retained-model-0.png` through `-3.png`
- `artifacts/graph-final/spatial-overflow-96-513x385.png` and its oracle
- `artifacts/graph-final/spatial-overflow-96-640x480.png` and its oracle
- `artifacts/graph-final-repeat/` for the four overflow captures, by hash
- `artifacts/graph-fixture-stability.json`

### Final round changed paths

- `handoffs/0020-render-pass-graph/result.md`, updated and committed.
- Ignored, not committed: `artifacts/review-0020-scripts/lifetime.py` and the
  rerun captures in `artifacts/review-0020-final/`.

### Final round open questions

None for pixel evidence. Both requests from the initial round are answered.

---

# Initial review at `1494fda`, preserved as history

Finding 2's statement about clustered out-of-order submission is corrected in
the final round above. The rest is unchanged.

## Status

Review complete. Scoped visual verdict: **pass. Scheduling causes no visual
regression.** I found no rendering defect in the supplied captures. I found one
low-severity evidence-quality defect and one small coverage gap. Neither blocks
this increment.

- 120 of 124 baseline pairs are byte-identical files. I recomputed this myself.
- The four differing spatial-overflow-96 pairs are explained by run-to-run
  nondeterminism in that fixture, not by scheduling. I reproduced the exact
  baseline bytes with the scheduled code. See finding 1.
- The three retained-model fixtures show positive coverage and shift in the
  expected direction for each offset. They reproduce byte-for-byte on my rerun.
- The native captures compose scene and chrome correctly before and after the
  viewport grows. The sphere rescales with viewport height and stays centered.

This review does not approve a phase gate. It does not certify game, device or
GPU-timestamp performance. It does not cover shadows, post-effects or the full
production graph, which remain open. No new editor control or camera selector
is claimed or reviewed.

Priority revisions: the current brief is the first version of this packet. No
director feedback beyond the brief and CLAUDE.md was present. I applied the
standing director decisions in CLAUDE.md, including the 2026-10-08 and
2026-10-09 decisions.

## Model

- Exact model: Claude Opus 5.5, model ID `claude-opus-5-5`.
- The runner invoked this session in the handoff worktree. I cannot inspect the
  ACP transport from inside the session.
- No model substitution occurred.

## Changed paths

- `handoffs/0020-render-pass-graph/result.md` (this file, committed).
- Ignored, not committed:
  - `artifacts/review-0020-scripts/png.py` is a stdlib-only PNG decoder.
  - `artifacts/review-0020-scripts/compare.py` independently recomputes the
    baseline comparison and checks it against the supplied report.
  - `artifacts/review-0020-scripts/pixdiff.py` lists changed pixels between two PNGs.
  - `artifacts/review-0020-scripts/blobs.py` measures foreground blobs.
  - `artifacts/review-0020-scripts/native.py` measures the native sphere and
    viewport extents.
  - `artifacts/review-0020-runs/` holds my rerun captures and PNG conversions
    of the native JPEGs. The native conversions show account identity, so they
    stay ignored.

I did not edit shaders, UI, tests, the ledger, documentation, project content or
the brief. I did not drive the native app. I did not capture the screen.

## Commands run and results

All builds used this worktree's own `target` directory.

```sh
python3 -I artifacts/review-0020-scripts/compare.py \
  artifacts/graph-baseline artifacts/graph-initial artifacts/graph-pixel-comparison.json
# 124 baseline, 127 current. The only new files are the three graph-retained-model captures.
# 120 pixel-identical and file-byte-identical. All 124 report hashes match the files.
# Changed pixels and deltas match the supplied report exactly.

for i in 1 2 3 4 5; do
  INCANT_LIGHT_EVIDENCE=$PWD/artifacts/review-0020-runs/run$i CARGO_TARGET_DIR=$PWD/target \
    cargo test -q -p incant_render --test model_gpu \
    distinct_colored_lights_preserve_energy -- --ignored
done
# 5/5 passed. Output bytes vary between runs. See finding 1.

CARGO_TARGET_DIR=$PWD/target cargo test -q -p incant_render -- --ignored
# 6 renderer unit GPU tests passed. 24 renderer integration GPU tests passed.

INCANT_HDR_EVIDENCE=$PWD/artifacts/review-0020-runs/retained CARGO_TARGET_DIR=$PWD/target \
  cargo test -q -p incant_render --lib queued_graph -- --ignored
# 1 passed. All three graph-retained-model PNGs are byte-identical to the supplied captures.

python3 -I artifacts/review-0020-scripts/blobs.py artifacts/graph-initial/graph-retained-model-*.png
sips -s format png artifacts/graph-native/<name>.jpg --out artifacts/review-0020-runs/native/<name>.png
python3 -I artifacts/review-0020-scripts/native.py artifacts/review-0020-runs/native/*.png
# Inline python3 -I recomputed the timing medians from graph-timing.json.
```

Not run by me. These remain Astra's reported results only:

- The 139 ordinary Rust tests.
- The 2 headless GPU tests and the 1 editor GPU test.
- The 283 UI tests.
- Workspace Clippy.
- Conventions, bridge and SDK checks.
- The five Python tool tests.
- The custom-protocol build and packaging.
- The native CUA capture session.

## Screenshots reviewed

No new screenshots were produced by me. I reviewed these supplied files:

- `artifacts/graph-initial/hdr-reference-swatches.png`
- `artifacts/graph-initial/studio-metal-satin.png`
- `artifacts/graph-initial/environment-rough-metal.png`
- `artifacts/graph-initial/camera-fov-50-visible-edges.png`
- `artifacts/graph-initial/camera-clusters-1-480x270.png`
- `artifacts/graph-initial/spot-penumbra.png`
- `artifacts/graph-initial/spatial-overflow-96-640x480.png`
- `artifacts/graph-initial/graph-retained-model-0.png`, `-1.png` and `-2.png`
- `artifacts/graph-native/output-open.jpg` and `output-collapsed.jpg`
- `artifacts/graph-native/accessibility.txt`

For the 120 byte-identical pairs, the baseline counterpart is the same file
content, so viewing one is viewing both. All representative captures match the
appearance accepted in earlier reviews. They cover the HDR swatch ramp,
studio material response, environment lighting, authored camera framing,
clustered colored local lights, spot penumbra and the 96-light overflow case.

## Findings

### 1. Spatial-overflow-96 differences: explanation confirmed, fixture not byte-stable (low, evidence quality)

The four differing pairs change by one 8-bit channel step at very few pixels:

| Capture | Pixel | Baseline RGB | Scheduled RGB |
| --- | --- | --- | --- |
| 513x385 and its oracle | 250, 213 | 196, 179, 184 | 197, 179, 184 |
| 640x480 and its oracle | 246, 178 | 74, 89, 81 | 75, 89, 81 |
| 640x480 and its oracle | 309, 279 | 170, 152, 161 | 170, 152, 162 |

All three pixels lie in the lit interior of the quad. None is on an edge or
cluster boundary. A one-step change at these levels is invisible.

I tested the random-ID explanation directly instead of accepting it. I ran the
unchanged test five times at the scheduled head:

| Size | Runs byte-identical to the baseline | Varying runs |
| --- | --- | --- |
| 640x480 | 4 of 5 | Run 5 differs by one step at pixel 429, 150 |
| 513x385 | 3 of 5 | Runs 2 and 5 differ by one step at pixel 208, 102 |

The scheduled code therefore reproduces the exact baseline bytes. The variant
pixels move between runs and never match the supplied current capture's
pixels. Clustered and oracle outputs are byte-identical within every run. That
rules out scheduling and cluster assignment as the cause.

The code supports the mechanism. The fixture creates 96 entities with fresh IDs
in a tight loop. The ID generator is not monotonic within one millisecond.
Scene entities are stored in ID order, so light order and floating-point
summation order change between processes. Saved projects have stable IDs, so
authored scenes render deterministically.

The defect is that this fixture cannot serve as a byte-stable cross-commit
reference. A future real one-step regression in this case would be
indistinguishable from noise. I recommend that Astra give the fixture
deterministic IDs or a deterministic light order. Alternatively, the comparison
tooling could document a per-fixture tolerance. I made no test edit, per the brief.
I am not waiving the four artifacts. They are explained and verified by reproduction.

### 2. Retained-model lifetime: verified, with one coverage gap (low, missing evidence)

The three fixtures are flat grey numeric triangles, not art. Each image has two
triangles from the two model instances on the background `#141519`.

| Fixture | Size | Foreground pixels | Left blob centroid, NDC x | Right blob centroid, NDC x |
| --- | --- | --- | --- | --- |
| Offset 0 | 321x193 | 1050 | -0.173 | 0.103 |
| Offset +2 | 480x270 | 2731 | 0.003 | 0.315 |
| Offset -2 | 321x193 | 812 | -0.323 | -0.089 |

Both instances move together in the expected direction for each offset. Each
fixture comfortably exceeds the test's 100-pixel positive-coverage floor. Offset
0 and offset -2 differ at the same size, so the test's inequality check has
real content. My rerun reproduced all three captures byte-for-byte.

I read the lifetime test and it checks what the brief describes:

- It rejects a viewport one pixel wider than the target and then checks that
  the graph holds no frame or encoder resource.
- It drops the project, the asset store and the temporary directory, then
  checks that no CPU model version survives.
- It submits the three retained command buffers in reverse and compares every
  output byte with its immediate reference.
- The final model-free frame matches the last cached attachment size of 321x193.
  So it really overwrites stale HDR and depth content. Every pixel must be
  `#141519` at full alpha.

The gap is that this test has no local lights. The cluster grid storage is a
shared cached buffer. Correct out-of-order submission for clustered frames
relies on each command buffer rewriting every used cluster word before shading.
The implementation does this, and it predates this change. However, no pixel
test submits clustered-light frames out of order. I recommend a clustered-light
variant of the reverse-submission check. This is not a regression.

### 3. Native composition: no defect

I measured the sphere and viewport in both supplied captures:

| State | Viewport content height | Sphere diameter | Sphere center y | Viewport center y |
| --- | --- | --- | --- | --- |
| Output open | about 552 px | about 252 px | about 351 | 352 |
| Output collapsed | about 796 px | about 364 px | about 474 | 474 |

The sphere grows by a factor of about 1.44 when the viewport height grows by a
factor of about 1.44. It stays circular and horizontally centered. No scene
content bleeds outside the viewport rectangle in either state. The Output panel
in the open state shows the empty Problems state. The status bar shows zero
errors and zero warnings in both states. The accessibility text reports
"Attached" and zero errors, and its Output toggle reports off, which matches
the collapsed capture.

Both captures include the CUA pointer overlay. One overlay is over the selected
hierarchy row and the other is over the Output toggle. These are capture
artifacts, not UI defects. The read-only Inspector note and the unavailable
agent panel are pre-existing states, not changes from this increment.

The captures show the native account identity, so they must stay ignored. This
report deliberately omits it.

### 4. Timing: consistent with "approximately unchanged", not a gate

I recomputed the medians of the three trial medians from the raw samples. They
match the supplied summary. Both paths used the same project hash for each
light count, the same adapter and 1920x1080.

| Local lights | Baseline ms | Scheduled ms |
| --- | --- | --- |
| 0 | 1.394 | 1.397 |
| 4096 | 6.611 | 6.506 |

At 4096 lights, one baseline trial reached 7.75 ms. The apparent improvement is
within trial spread and should not be read as a speedup. These numbers measure
CPU encode, submit and a completion wait only. They are not GPU timestamps and
do not certify any game or device budget.

## Rationale

The brief limits this increment to execution order and resource ownership, with
no appearance change. Byte identity is the strongest evidence for that, and it
holds for 120 pairs. I reproduced the baseline bytes for the remaining four
with the scheduled code. The new lifetime fixtures are numeric checks, so I
judged them by measured geometry and exact reproducibility rather than by look.
The native captures were judged by layout and projection consistency.

## Open questions and requests for Astra

1. Should the spatial-overflow-96 fixture use deterministic IDs or light order,
   so that it can serve as a byte-stable cross-commit reference?
2. Should the reverse-submission lifetime test gain a clustered-local-light
   variant, to pixel-verify the shared cluster grid under out-of-order submission?

No data bindings are requested. No routing exception was used. No phase gate is
approved by this review.
