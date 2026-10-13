# SSAO repair: CPU proposal experiment

This is a test-only experiment on `d2d2294e`. Production shaders, prepass
normals, attachments, default-off behavior and native cooked-material refusal
are unchanged. It does not approve a repair, GPU result or appearance.

The six `ambient_occlusion` inputs add independent analytic pixel-center depth
and normal samples, exact f32 direction/rotation tables evaluated in f64,
rotation and step jitter, and the proposed separable five-tap plane filter.
The original fixed kernel remains a negative control. The native shader is not
called by this model; camera geometry and nominal bias are f64, without an
assertion of f32 GPU decision equivalence. R8 round-to-nearest is modeled after
raw AO and each filter pass. GPU tie behavior and the full decision/tone error
interval still need an explicit observer contract.

The modeled mathematical inputs are the committed Claude design
`eeeac1d4f633a1777db746ecda49df4901e54fe7` and root's explicit resource decision:
keep four megapixels, add one R8 target for 14 bytes/pixel and a 56 MiB logical
one-size idle-cache payload. One retained old maximum plus a new candidate is
108 MiB logical; three candidate maxima are 168 MiB. Neither number measures
driver allocation, nor bounds caller-owned in-flight queue depth. The proposed
rotation array is 16 vec4 values, making the frame uniform 736 bytes.

The first frozen draft experiment (source001) used the then-proposed geometric
normals and a 32×16 sphere over a floor. It ran seven passing controls and one
failing control: its 20 top-cap samples were unoccluded with both geometric and
old shading normals, so the demanded shading-normal mutation did not fail.
The committed design withdrew geometric normals and changed N5. Source002
follows that explicit design change, preserving the failed draft and its log.
The reversed authored normal remains unoccluded; the withdrawn viewer-facing
geometric variant is retained as a counterexample that would violate that gate.

Source002 has nine passing CPU tests and strict all-target renderer Clippy.
Those test passes establish model invariants and measurements; the proposed
quality gates below have **not** all passed. Counts overlap with the historical
run and must not be added. No GPU tests were run.

| Observation | Result |
|---|---|
| N6, original perspective receiver row y97, x100–171 | Maximum repaired adjacent difference is 7/255 = 0.02745098 at x161→162 and x162→163, exceeding 1/64 = 0.015625. Jitter-off has the same maximum, so this metric does not discriminate jitter. |
| N7, declared 2-pixel Chebyshev silhouette exclusion | Old raw counts are 1382 / 1767 / 2027 for white perspective / white orthographic / edge perspective. Proposed filter yields zero in each. The design's 1397 / 1782 / 2046 counts require clarification of the exact mask. |
| Filter-off mutation | Counts 5196 / 5272 / 7830, which fail the one-tenth gate and distinguish filtering. |
| Rotation-off mutation | Counts 0 / 51 / 0 also satisfy N7. The final design assigns this mutation to N8 instead. |
| N8 diagnostic stripes | The draft omits exact stripe origin, vertical extent and receiver-row ROI. Explicit rows with mixed receiver/occluder coverage fail its 25% reduction criterion; these are reported as diagnostics, not fabricated matches to an authored acceptance fixture. |
| Plane-weight-off and asymmetric-window mutations | The first causes actual slab-edge bleed; the second changes thousands of samples. |
| N5 isolated 24×48 radius-0.6 sphere | Both projections stay fully open with original and proposed kernels. A continuous conservative rise upper bound of 0.002561075714 is below bias 0.01. An actual rise witness of 0.001144977283 exceeds bias 0.001, proving refusal of that reduced-bias precondition. |

The N5 upper bound is independent of sampled barycentric density. For this
analytic-vertex mesh, interpolated shading normal is parallel to receiver p
relative to the sphere center. Every surface S lies in the radius-R ball, so
`dot(S-p,n) <= R-|p|`. Each triangle's plane distance d gives `|p| >= d`.
Thus `R-min(d)` bounds every receiver point continuously. It is conservative,
not the exact supremum; finite native vertex and GPU errors are not included.

A separately retained stdlib Python implementation uses the prior independent
reference's camera and geometry, then implements the proposed sampling/filter
algebra. It reproduces all 144 candidate and jitter-off N6 scalar records
exactly. This independently confirms the N6 failure without images or GPU work.

The [lossless packet](evidence/ssao-quality-oracle.json) binds both source
generations, the original red log, final CPU/Clippy logs, per-field observations,
Python cross-check and exact final test binary. Reproduce only the CPU model:

```sh
CARGO_TARGET_DIR=/path/to/owned-target ./tools/cargo test --locked --offline \
  --profile ci -p incant_render --lib ambient_occlusion::quality -j2 -- --nocapture
```

Production admission remains pending a corrected, explicit design decision on
the failed or ambiguous metrics. No threshold, radius, bias, fixture or art
setting was tuned to obtain acceptance. Native GPU/raster/DRS correspondence,
observer uncertainty intervals, actual resource lifetime validation and Claude
appearance review remain later work.
