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
