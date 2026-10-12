# Runtime pose blending

`AnimationPlayer::sample_blend` evaluates two independent clip clocks, blends
their local translation/rotation/scale values, then composes the hierarchy and
skin palettes. Missing channels use the authored rest pose. Static matrix nodes
remain static. Translation and scale interpolate linearly; normalized quaternions
use shortest-arc spherical interpolation. This avoids the joint collapse caused
by interpolating final skin matrices.

Both inputs and the finite weight in `[0,1]` are checked before sampling, including
an unused endpoint input. Exact zero/one weights produce the original single-clip
result. A late numeric overflow leaves all published local/global matrices and
palettes unchanged. Sampling uses a second startup-allocated TRS scratch array;
`retained_bytes` includes it, so existing renderer admission charges the added
memory before constructing players. No dependencies or cooked formats change.

Local validation on the Apple Silicon development Mac:

- 103 asset/import/reader tests passed, including six new blending cases and a
  fresh child process that samples cooked animation after source deletion.
- 62 renderer/native-input CPU tests passed; 70 GPU tests stayed ignored.
- 1,024 accepted blended pose, mesh-local palette and vertex-deformation calls
  made zero allocations under the existing thread-local allocator instrument.
- Affected all-target Clippy, formatting, generated conventions and the resolved
  runtime dependency guard passed. Independent source review found no issue.

[Source/test evidence](evidence/runtime-pose-blending.json) and
[independent review](evidence/runtime-pose-blending-review.json) pin the checked
inputs and logs. This is a working two-input runtime primitive. Authored blend
trees, transition clocks/state machines, native frame/SDK blend controls, IK and
root motion remain outstanding. No rendered appearance, reference-phone timing,
shipping performance, or Phase 1 gate is claimed.
