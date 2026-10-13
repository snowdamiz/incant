# Portable output history with owner provenance

This private implementation composes the reviewed integer owner attachment with
Claude 0073's corrected output-history prescription. It remains an opt-in
experiment. The private branch includes earlier upscaling/MetalFX work that has
not received general appearance acceptance; this packet does not approve that
ancestry or close the temporal-quality, device or performance gates.

The new shader kind adds two ping-pong Rg32Uint output records. Each record holds
an anchor and one further exact owner, or a conservative 31-class overflow mask.
Positive-weight current support, individually accepted previous taps and admitted
clamp contributors all enter the record. Clear/mixed/untracked support cannot
produce a reusable record. Exact changed-owner bits reject departed contributors;
overflow records use a conservative changed-class intersection. Rejected taps
leave the normalization weight. Ordinary reprojection retains the existing
coordinates and color/depth equations.

Unclipped own-pixel persistence additionally requires the accepted exact camera
witness, less than 1/256 pixel displacement, unchanged current support, unchanged
recorded contributors, compatible depth and no global lighting/shadow change.
The light witness compares every packed byte and count; changed shadow casters
include placements culled from a batch by handedness and animated palettes.
Candidate preparation never replaces submitted witnesses. The actual queue
submission precedes acceptance of transforms, provenance, lights and targets.

The private retained baseline freezes material/environment content and has no
mutable SSAO or bloom settings. Any later current-main composition that permits
such effects to change must invalidate history before granting persistence. The
present code makes no claim that an unimplemented effect epoch already does so.

## Admission and native interface

The 12,528-byte change uniform and equally sized retained CPU upload, plus the
accepted light capacity, are charged under the existing motion-state cap. No
per-frame allocation is needed for those witnesses or the resolve binding and
attachment lists. Output records add 16 bytes per output pixel. Candidate and
accepted target overlap uses both the exact extent and owner-attachment flag;
compatible caches reuse the same allocation. The existing 671,105,024-byte
aggregate ceiling is unchanged; over-budget resizes refuse before allocation.
These are logical requested-byte limits, not driver residency or allocator RSS.

A separate UpscaleOwner kind pins the whole source and exact resource/MRT
contract. Its frame uniform occupies native buffer0, changes buffer1, and its
seven textures preserve their explicit native slots. Old shader sources and
contracts stay unchanged. Every native backend artifact must pass the cooker
checks; CPU compilation alone does not establish device execution.

## CPU and compiler qualification

The final frozen source passes ten checks: 33 shader-cooker tests, 152 default
renderer CPU tests, 167 cooked/native CPU tests, both strict all-target Clippy
configurations including the game consumer, WebAssembly compilation, runtime
dependency checks, formatting and conventions. The renderer suites leave 42 and
43 GPU cases ignored respectively. All 2,736 selected input files remain exact
through the checks. Every one of the 20 previous WGSL sources is unchanged.

The initial full cooker exposed Naga 29.0.4's restricted signed-vector-index
SPIR-V output: a signed dynamic index was passed to unsigned Min. Claude's
three explicit unsigned-index conversions preserve the proved 0–3 indices;
the full emitted-artifact roundtrip now passes. No visual threshold changed.
The earlier failure/diagnostics and one redundant host-conversion Clippy failure
are retained, rather than replaced by the passing receipts.

All 57 freshly emitted previous native payloads match their saved originals.
Nine temporal manifests change only their producer fingerprint because the
compiler implementation changed; source/interface/payload identities match.
Another 67 retained artifact records belong to a prior feature combination and
were not rebuilt by this qualification; they are identified separately.

The new numerical fixture has 12 cases at 16 by 16, with one encoder/submission
and batched readback per case, including ordinary-pipeline controls in two cases.
It covers malformed/untracked records, static persistence, exact/overflow change
rejection, zero-weight support, anchor mismatch, clamp-only contributors, four-tap
renormalization and unchanged ordinary fallback. Frozen test binaries and exact
build inputs are retained for the separately bounded default/cooked exercise.

## First device result: rejected

The first delegated WGSL run on Apple M5 Pro / Metal passed cases 1 and 2, then
failed case 3: the zero-weight-support counterexample expected zero RGB but read
0.40478516. The test reached the third submission/readback before its assertion;
the runner conservatively charged its full twelve-case allowance and stopped.
The cooked-native binary was never launched. Raw output and the released child
process record are retained in `0073-owner-history-gpu-001`. Diagnosis and a
separately reviewed correction are required before another device run. The
expectation is preserved; this code is not accepted for main. Full retained
integration, appearance, device and performance acceptance remain open.
