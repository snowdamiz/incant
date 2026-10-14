# External reconstruction comparison — private diagnostic

This compares the private reconstruction branch, **not current main or a shipped
release**, with six unchanged shaders from `pmndrs/upscaler` at commit
`96184a6812bc5fd5e5fd7898257788a1663105a9`. It is a separately implemented WebGPU
reference with documented deviations from FidelityFX, not AMD's official FSR2.
The MIT source and license remain outside Git. There is no production dependency,
algorithm adoption, visual acceptance or performance claim.

The Rust adapter loads explicit external WGSL files only in an ignored test and
checks all six hashes. An external replay verifies the pinned source archive,
82 extracted files, literal shader chunks, the luminance module's constant-field
substitution, and exact deduplicated assembly without executing upstream TS.
The source archive SHA-256 is
`d13e65696f0be4213dbb26f07dc0d54db4c645ae584f482f007691eb6c15c5aa`.

## Scope and settings

Both paths consume the same actual retained-scene HDR color, jittered motion and
depth: 480×270 internal and 960×540 output, 24 stationary and 24 moving frames.
Reference motion is unjittered current-minus-previous UV; Incant's motion is
jittered previous-minus-current UV. The adapter negates the motion and removes
the projection jitter delta. The reference current/previous jitter signs are
also reversed. Invalid input motion is zeroed in this diagnostic only; production
invalid-motion policy, camera cuts, dynamic resolution and partial-coverage
conformance are not established.

The reference uses fixed exposure 1, maximum history count 24, thin-feature locks
and shading-change detection, no sharpening, and Incant's existing eight-phase
jitter. The last choice differs from the reference's own ratio-dependent sequence
(32 phases at 2×) and must be isolated before attributing all residual artifacts
to its reconstruction algorithm. The first frame resets its history. Its six
passes plus the motion bridge run 336 dispatches over the 48 frames.

Reference texture/buffer payloads total 27,478,360 bytes at this fixed extent,
excluding allocator/driver overhead and pipelines. This is additional to the
Incant diagnostic's 24,883,200 history bytes and 9,950,208-byte readback, not a
production admission measurement. Incant auxiliary history/depth/evidence planes
remain Incant outputs; they must not be described as reference internal state.

## Retained execution

Evidence directories are `~/.codex/incant-evidence/reconstruction-reference-gpu-*`.
Each has frozen inputs, copied executable, bounded supervisor and terminal logs.

- 001: GPU validation refused a missing upstream constant-field substitution in
  my shader assembler. No scene output was produced.
- 002: first-frame raw output was finite, but 39 pixels had small negative RGB
  after inverse YCoCg. The Incant-only unsigned half-code comparator refused it.
- 003: the same signed values reached and failed a redundant display precondition.
- 004: kernel equality is inapplicable to the external path and is reported as
  absent. Signed raw samples remain untouched; per-frame ranges and counts are
  recorded. Display retains its original pre-tone RGB clamp. Build, two CPU
  controls, strict all-target Clippy and the complete Metal scene all pass.

004 freeze is `1b23b52768bd771cf0e68be8f440fe9142aa378b92f0bf17c22ff444b8461872`;
its executable SHA-256 is
`d985ec2522ed4e1364fa82e19afc85e5354f09d2d02de76e1815bb2fea4ccd94`.
All owned processes were reaped. The Apple M5 Pro/Metal test-profile run performs
96 submissions and 48 maps, preserving 384 raw planes and ten PNGs. All reference
output is finite. Across frames, 497 pixel observations have negative RGB (minimum
−0.00037956) and 10,742 have nonzero RGB at exactly zero alpha. These are explicit
premultiplied-domain incompatibilities, not hidden by the display image.

Root inspected stationary frame 24 and moving frames 8/24 against their baseline.
Large silhouettes and the horizon are smoother. Thin diagonals still break up,
and moving texture detail remains visibly aliased. This is useful comparative
evidence, not acceptance. The PNGs use a CPU transcription of Incant's display
transform and exclude current-only overlays; they are not exact final display
captures. The next controlled comparison changes only the diagnostic jitter
sequence to match the reference's expected sample coverage at this scale.

## Matched sample-sequence comparison

005 and 006 extend each stationary/moving case to 64 frames. Both execute the
same frozen test binary (`2c2b0fee2af07cc9e67a6c89a49e228590f2dd606ccccd811f6e2370a6295fad`)
and source freeze (`b37ec9f2dc550eda54e71e72cac52f2949f93624481f0e54a8f84d42fe67dedf`).
005 uses Incant's eight offsets; 006 uses the reference's 32-phase Halton(2,3)
sequence with the sign conversion required by Incant's projection. The override
is test-only, thread-local and guarded; CPU controls verify restoration, other
thread isolation and phase wrapping. Production remains on its existing sequence.

Each successful Metal run retains 128 frames, 256 submissions, 128 maps, 18 PNGs
and 1,273,626,624 raw bytes. The 005 build, CPU controls and strict lint pass;
006 explicitly reuses that copied binary without claiming another build. Both
supervisors are terminal and all owned processes are reaped. The external six
shaders, exposure, history limit and resource configuration are unchanged.

Root inspected both stationary frame 64 outputs and moving frames 24/64. Changing
sample coverage does not cure the breakup of thin geometry or the broad moving
texture aliasing. This eliminates the proposed sequence-only repair; neither
result is accepted. The 005/006 outputs respectively contain 1,073/1,169 negative
RGB pixel observations and 48,043/49,081 nonzero-RGB observations at zero alpha;
minimum RGB is -0.000691414/-0.004730225. These are counts across frames, not
unique scene pixels. Raw signed values remain retained. The external reference
still fails Incant's premultiplied-domain expectations and is not adopted.

`reconstruction-reference-gpu-006/matched-jitter-summary.json` records the matched
identities, phase counts, PNG identity checks and observations. This is private
algorithm investigation; it establishes no current-main, final-display, shipping
performance, phase or release result.

## Reference history observation

007 changes only test readback: three otherwise unused Incant auxiliary planes
are replaced by reference masks, history RGB plus sample count, and locks (life,
tracked luminance, shading change and alpha). Formats, crop extents and strides
are recorded explicitly. The six reference shaders and configuration are exact
to 005. All 640 shared raw input/output planes are byte-identical to 005; the
observer did not alter either reconstructed output on this run.

The source freeze is
`5f2277b6c93b1c4f31f316698174fb0ebc580246cf3767194e87cc6dcadaa50d`;
the immediate executable is
`88308436e6f2690675c98d0c940f9886fe2947d830abf90b9e6a0ff6d5571f59`.
Build, two CPU controls, strict lint and the 128-frame Metal run pass. All owned
processes are terminal and reaped. The run retains 256 submissions, 128 maps,
18 PNGs and 1,471,938,560 raw bytes. Its packed readback is 11,499,520 bytes.
This is diagnostic allocation, not shipping memory admission.

The bounded analysis hashes all 384 new state planes, projects four authored
chart regions into the camera and samples a three-pixel grid. The stationary
line region reaches history count 24 with no shading-change or disocclusion
mask at frames 32 and 64. During camera motion, the sampled line region has
zero disocclusion at frames 15–32, but its shading-change mean repeatedly
spikes (0.550 at frame 15, 0.717 at 19, 0.655 at 23, 0.718 at 27, 0.650 at 31).
History count falls to roughly four to eight samples. The cropped history and
mask planes no longer cover this region at frame 64; missing observations are
explicitly null. These are sampled observations, not whole-scene quality metrics.

The evidence points to repeated shading rejection as one source of unstable
accumulation. It does not yet prove that rejection is false: this moving fixture
also contains a moving shadow caster. A camera-only control with the caster held
stationary is needed before changing that logic. No threshold is tuned here and
no visual repair is claimed. `state-observations.json`, `analyze_state.py`,
`observer-equivalence.json` and `verify_observer.py` in the 007 evidence directory
retain the raw-analysis and matched-observer receipts.

## Camera-only control

009 keeps the moving cube at its initial pose and changes only the camera after
frame 14. A CPU control checks every non-camera node at five steps and confirms
the ordinary combined-motion route still moves the cube. Build, three CPU
controls, strict lint and the complete 128-frame Metal run pass; all processes
are reaped. The preceding 008 build failed because the new test attempted to
clone non-Clone scene nodes; it produced no GPU output. The repair copies only
the transform matrices being compared. The failed generation is retained.

009 freeze is
`ad497f4d39025b1ae1e64b8e60d428168779f54d07ec04c8bea2335fd6b894d9`;
its executable is
`ffc4a05871bde0ad6a500c3d27e0b7a17df0d938a2507a9a6081bd36b38f0f6b`.
The 576 raw planes covering all stationary frames and the first eight motion
frames match 007 exactly. At frames 15, 19, 23, 27 and 31, the sampled line
region's shading-change and history-count statistics also match the original
combined-motion case exactly. The moving shadow caster is therefore not needed
to produce this rejection. This control does not eliminate view-dependent
shading or camera-relative shadow-map sampling.

Root inspected camera-only frames 24/64 and the private baseline at 24. Broad
moire in the one-texel stripe chart and broken thin diagonals remain. No visual
repair or algorithm adoption is claimed. `camera-only-comparison.json` binds the
matched raw identities, observations and inspection. The output-grid texture
LOD bias admits frequencies above the internal grid's Nyquist limit; the next
input-bandwidth control will isolate that deliberate undersampling from history
rejection, without treating a blurrier input as proof of better reconstruction.

## Input-bandwidth control

010 repeats the camera-only sequence with a test-only texture LOD bias of zero,
so material color and MASK motion coverage both use the internal pixel footprint.
Production keeps its existing output-footprint bias. The thread-local override
has a non-Send scoped guard; controls check restoration and other-thread isolation.
No external shader, temporal setting, exposure or geometry changes. All 256 raw
motion/depth planes match 009 byte for byte.

The source freeze is
`cfba9b272df7773efd9aa18643f2de85b3a84d50cc8f0dc660419fb873d6bce6`;
the executable is
`d2966827e31c6eb0b3f5532352cbf030c7b02a5989bbb49ffab532324b39f11c`.
Build, four CPU controls, strict lint and the 128-frame Metal run pass. The scene
retains 256 submissions, 128 maps, 18 PNGs and 1,471,938,560 raw bytes. All owned
processes are terminal and reaped. The bounded state analysis also completed.

At frames 15, 19, 23, 27 and 31, the sampled line region's shading-change mean
falls from 0.55–0.72 to zero. History count builds steadily and reaches 24 by
frame 27. Root inspected camera-only frame 24: broad line-chart moire is greatly
reduced, while the chart loses unresolved stripe detail and the thin diagonals
remain broken. This isolates input aliasing as a cause of repeated shading
rejection in this region. It does not establish a universal texture policy or
prove that filtering away detail improves reconstruction.

`reconstruction-reference-gpu-010/bandwidth-comparison.json` binds the source,
binary, reports, sampled observations, matched motion/depth and visual review.
This is a partial diagnostic result on the private comparison branch. No
production change, complete visual acceptance, shipping performance result or
phase-gate acceptance is claimed. Geometry coverage and stable detail recovery
remain unresolved.

## Coverage-lock derivative

011 changes exactly one expression in the diagnostic reference's accumulation
shader: its existing luminance lock also relaxes coverage rectification. The six
original external sources are verified before the derivative is assembled;
immutable upstream files remain unchanged. The report identifies the modified
algorithm and hashes the actual accumulation shader. This is an experiment,
not an upstream or production result.

The source freeze is
`54adc8f796a320d1a84b4b2e59ea68c971b42bb635b82f393d4001af0d4f64b5`;
the executable is
`3e6df2c94a6a71316c0bf7c50b2133f70624cfe8af9a0b4e3efa378d982a19e6`.
Build, four CPU controls, strict lint and the 128-frame Metal run pass. The scene
has 256 submissions and 128 maps; its owned process group is cleaned and reaped.
The original and derivative share all 512 baseline/current/motion/depth planes
byte-for-byte. Every RGB channel is also exactly equal across all 128 candidate
frames; alpha differs in 50 moving frames. Thus this particular coverage change
does not repair the color loss. Root inspected both camera-only frame 24 images:
the broken thin diagonal remains. No production adoption is justified.

A bounded observation samples 200 projected positions on the authored diagonal.
At frames 14, 15, 16, 19, 24, 32 and 64, between 16 and 30 sampled positions have
no geometry in the corresponding current 3×3 depth neighborhood. Between 22 and
32 positions have strongest RGB below 0.005 across five output rows, with many
of those positions coinciding with empty depth neighborhoods. This is sampled
diagnostic evidence, not an analytic coverage oracle or proof of causality.
It directs further investigation toward the treatment of history when thin
geometry is missed by rasterization, rather than an alpha-only change.

The 011 evidence directory retains `comparison.json`, `channel-comparison.json`
and `diagonal-depth-observation.json`, in addition to the exact shader derivative,
raw outputs and execution records. No shipping memory/performance, final-display
quality or gate claim is made.
