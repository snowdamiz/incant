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
