# Prepare the current renderer pipeline catalog before drawing

Renderer initialization now creates every currently supported pipeline before
returning a usable renderer: twelve HDR material variants (three alpha modes,
two culling choices and two reflection choices), eight shadow variants (opaque
or masked, with both culling and reflection choices), four output formats, one
diagnostic geometry pipeline and one sprite pipeline. The existing clustered
lighting compute pipeline is also prepared there. Shader modules are shared
while each family is built. Frame encoding looks up immutable handles; it no
longer creates material, shadow, output or diagnostic pipelines on first use.

Initialization reports captured backend validation errors through a typed
preparation error. An absent prepared key returns an explicit error. Existing
unsupported output formats and blended shadow casters still fail explicitly.
All WGSL source files, material math, vertex layouts and attachment behavior are
unchanged by this slice.

## Verification

Production source is `21a2aa2fb9afcd5903453ca4c1cca32ebf72c6fb`, based on
PR79 `74238785fe92c50daea1d8dd056492359def61f2`. Later packet, documentation
and standalone compiler-probe commits do not change these production sources.
Independent Astra review found no omitted reachable key or changed binding
ownership; source hashes are retained in the machine evidence.

On macOS 26.6 (25G72), Apple M5 Pro, Metal:

- All 601 ordinary workspace tests/doctests and all 67 ignored GPU tests pass.
- Workspace all-target release Clippy, optional GPU-player Clippy, 88 Python
  tool tests and generated-convention checks pass.
- The new GPU regression imports all twelve material combinations and submits
  each to RGBA8/BGRA8, linear and sRGB output targets. After every actual
  submission, the backend's render-pipeline, compute-pipeline and shader-module
  allocation reports equal their post-initialization reports. Reimports do not
  add a pipeline or shader module during drawing.
- A fresh shipping build passes the existing source-free GPU player proof:
  eight model/sprite and static/streaming-audio cases, 33 explicit refusals,
  CPU fallback controls, bounded logs, typed observations and unchanged-output
  checks. Original project, asset sources, script source and fresh Cargo storage
  are removed before player execution. Actual offscreen Metal frames and PCM
  are produced; this proof does not present a native window.

The shipping executable is 7,603,360 bytes, SHA-256
`c1a160d8a92501bf3aa416f8edd085495a349d56cd9fa6c07daa5be29d8c615e`, with
matching arm64 dSYM `aa24c02e-190a-38e9-ae20-d190326a2487`.
[Machine evidence](evidence/prewarmed-render-pipelines-local.json) contains
complete shipping results, source and log hashes, and exact validation counts.
The preparation duration printed by the test is a single local diagnostic with
uncontrolled driver caches; it is not a benchmark or a cold-load claim.

Claude handoff 0041 accepted the scoped appearance after an ACP session with
Opus 5.5 and explicitly verified Max thinking. Its 11 baseline/candidate pairs
are byte-identical; Claude also inspected their rendered appearance, including
all material and shadow variants, lighting, sprites, reimport, source deletion
and the diagnostic pipeline. All 26 guarded captures finished and the shared
GPU slot was released. [Claude report](../../handoffs/0041-prewarmed-pipelines/result.md)
and its original `Built-by: claude` commit `23fe9ca` are preserved.

The review also records two existing shared defects: generated-tangent normal-map
Y inversion (correction under review in PR77), and alpha-weighted colour mips
that darken alpha-zero texels used by an OPAQUE material. Neither is a regression
introduced by preparing pipelines. The opaque mip issue is being investigated
separately, including sources shared by opaque and transparent consumers.

The local combined renderer/flipbook source `faf12e8` additionally passes 666
ordinary workspace tests, two bounded-capture helper tests, full-workspace and
optional native-window release Clippy, 90 tools tests, conventions and the
15-root authoring-free dependency guard. Its 72 GPU cases and fresh integrated
shipping proof remain pending; the earlier separate results above do not stand
in for those checks. [Combined local evidence](evidence/native-render-followup-local.json).
Hosted checks and publication remain pending behind the coordinated runtime queue.

## Remaining shader and performance work

This prepares the fixed current catalog, not a recorded per-level permutation
list. Native runtime still parses WGSL and invokes the backend compiler while
initializing. Offline backend shader cooking, runtime shader-parser exclusion,
persistent Vulkan/Metal/DX12 caches, asynchronous preparation of future unknown
variants, fallback materials, and the reference-phone hitch/performance gates
remain open. No phase gate is approved here.

The separate [shader-cook investigation](shader-cooking-interop.md) translates
all current entry points and compiles their generated Metal source, but its
compact binding ABI is deliberately not loaded into the renderer. The pinned
wgpu Metal passthrough path omits metadata needed for dynamic buffer bounds and
threadgroup memory. That integration needs a checked backend contract before
the offline output can replace runtime translation.
