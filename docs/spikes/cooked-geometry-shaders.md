# Cooked diagnostic and shadow shaders (opt-in)

`incant_render/cooked-geometry` extends the optional output-pass path with actual
build-produced diagnostic and shadow stages. It implies `cooked-output` and is
off by default. The default native and Web routes keep WGSL. The initial source
review below was CPU-only; subsequent Metal numerical checks are recorded at the
end of this document. No result enables the feature by default or establishes
Vulkan/DX12, device-performance or whole-renderer completion.

## Contract and consumption

The build-only `incant_shader_cook::geometry` reads the existing engine-owned
`fixture.wgsl` and `shadows/caster.wgsl`. Neither source, cascade mathematics,
alpha cutoff nor eight opaque/mask × double-sided × reflected-winding variants
changed. Exact Naga29.0.4 IR validation checks both entry points, complete stage
IO, resource types/visibility, camera and material layout, and refuses extra
storage/workgroup resources or overrides. Stage compaction emits six payloads
per family: vertex/fragment MSL2.4, SPIR-V1.0 and HLSL6.0, with reflected entries.

A private common consumer selects only the embedded fixed-family manifest.
Contract, source/producer identity, selected stages, entries and payload hashes
are checked before either unsafe shader-module call. Geometry also checks the
actual host vertex/camera/material layouts first. No project path or arbitrary
project artifact reaches this boundary; hashes are integrity checks, not an
untrusted shader admission policy. Unsupported native capabilities/backends fail
explicitly, with no silent WGSL fallback. The diagnostic pipeline now supplies
its explicit empty layout. The cooker is absent from resolved normal runtime
dependencies; wgpu's compiler remains for other families.

The complete host layouts, not only used attributes/resources, determine pinned
wgpu-hal29.0.4 bindings:

- Diagnostic: stride28 vertex locations0/1, no resources. Metal pulls bounded
  vertices from slot15 and reads supplied lengths at buffer0; DX12 special
  vertex/instance constants use b0. Its fragment returns RGBA.
- Shadow: stride48 mesh and stride112 instance streams preserve every existing
  attribute, offset and step mode. Metal uses slots15/14, camera buffer0 and
  lengths buffer1; fragment material/texture/sampler slots are each0. Bounds and
  base vertex/instance behavior match the pinned normal shader path.
- The full material layout has five texture/sampler pairs, even when shadow
  shading uses only the first. DX12 camera/material use b0/b1, texture t0,
  sampler-index SRV **t5**, base sampler index0, standard/comparison heaps
  s0/s2048 and special constants b2, all in space0. Paired fragment reflection
  supplies the vertex IO contract; opaque variants retain the same vertex stage
  and omit the fragment stage, as before. Actual depth-only pipeline acceptance
  is still a driver/GPU obligation.
- Vulkan keeps explicit sets/bindings, bounded access/loops and SPIR-V1.0 with
  no added Y flip (hal supplies the negative-height viewport). Geometry needs
  only the Shader capability. Storage-length and compute metadata are outside
  this fixed contract.

Both shaders remain triangle programs. Pipeline blend/depth/cull/bias settings,
batching and arithmetic are unchanged. The finite eight-variant shadow catalog
is preserved; it is not per-level permutation recording.

## CPU evidence

[Exact source, logs, pinned backend files and artifact receipt](evidence/cooked-geometry-shaders.json)
bind the freeze on `df71ef8` (preserved shader branch plus actual main103).

- Ten cooker tests pass: five existing output cases and five geometry cases
  covering deterministic emission, SPIR-V parsing/validation/resource interfaces,
  host-binding/IO/refusal mutations, Metal bounded pulling and DX12 sampler
  indirection. This is emitted-source validation, not driver execution.
- 42 cooked renderer and 36 default renderer CPU tests pass; 18 GPU tests remain
  ignored in each run. Private tests select every native descriptor without WGSL,
  check host/source identity and reject cross-family manifests. Existing output
  corruption/capability controls are retained through the shared consumer.
- Default and cooked all-target Clippy, default and feature wasm32 library
  compilation, fmt/conventions and the actual runtime dependency guard pass.
  Main103's WebGPU portability repair is preserved.
- Apple Metal32023.921 compiles and links all four new stages with
  `-std=macos-metal2.4`. Their compiled libraries are validation artifacts; the
  runtime embeds MSL for normal backend driver compilation. No verified DXC was
  available, so no HLSL/DXIL compilation is claimed.
- All six original output payloads remain byte-identical. Its producer identity
  changes because the cooker module graph changed; the shader payloads did not.
  The immutable `550b563` default/cooked comparison binaries and their pairing
  manifest were rehashed unchanged. They remain the first planned GPU comparison,
  separate from this later geometry source.

Initial attempts found a missing Naga `per_primitive` pattern, temporary Rust
vertex attribute arrays and a test expecting shortened HLSL register syntax.
These were corrected before the final source freeze. The first Metal CLI used
an unsupported shorthand standard flag; rerunning with the documented platform
spelling compiled the identical sources. Failed logs remain in the receipt.

## Remaining renderer work

Sprites, light-grid compute and concatenated material shaders still translate
from WGSL at runtime. The latter two need a separate, exact storage-length and
compute contract before native passthrough. Persistent driver caches, recorded
per-level permutations, asynchronous missing-permutation fallback and hitch/
device measurements remain unfinished PLAN requirements. This slice makes no
whole-renderer translation-removal, backend/device parity or performance claim.

The subsequent checks reuse the existing geometry, shadow mask/reflection/range,
queued-atlas and pipeline-warmup numerical tests under default and cooked geometry.
Any byte mismatch requires diagnosis; Claude owns rendered pixels and appearance
acceptance. Metal success does not imply Vulkan/DX12 or reference-phone evidence.

## Independent review and subsequent output-only check

The source freeze is commit `c5268ae05236e5836f04a6098ac0eef31ca6ad43`.
[Independent review](evidence/cooked-geometry-shaders-review.json) accepted the
18 source files, complete host/unsafe boundary and CPU evidence with no finding.
That CPU review did not establish geometry driver or appearance acceptance; no
geometry fixture had run at that point.

After the native review explicitly released its grant, only the preserved
`550b563` output comparison pair ran its two existing HDR tests. Both processes
passed; all 16 normalized readbacks across four formats and resize cases were
byte-identical on Apple M5 Pro/Metal, with matching shader and adapter identities.
The process IDs were independently absent before graphics ownership was returned.
[Exact output-only receipt](evidence/cooked-output-metal-numerical.json).
No PNG, screenshot, geometry draw, provider call or performance measurement was
made. Output and geometry remain optional pending their respective remaining
backend and Claude acceptance.

## Actual main109 integration and Metal numerical checks

Source `9a13f4b87813a4b1b8b79fb96792794052b4b6d8` integrates actual main109
`94c4927abadb10d2203db2e1ab031de7853947fa`. Only STATUS and the additive runtime
dependency guard required conflict resolution; both authoring-agent and shader
cooker exclusions remain. Renderer, cooker and fixture source are unchanged from
the frozen source used for Claude0051; main's newly integrated asset readers
required new linked comparison binaries.

[Integrated receipt](evidence/cooked-shaders-main109.json) records 52 cooked
renderer/cooker and 36 default CPU tests, all-target Clippy, default and feature
Web library compilation, dependency checks, formatting and conventions. The
805-file source freeze and copied test executables bind profile, toolchain,
features and build logs. No source changed during compilation or execution.

The four serial GPU suites pass all 138 cases: 19 library and 50 model cases in
each mode. [Numerical receipt](evidence/cooked-shaders-main109-numerical.json)
records Apple M5 Pro/Metal, exact commands, child identities, logs and normalized
readbacks. All 16 output and 12 diagnostic pairs match byte-for-byte across the
four formats, resize and actual reflected-draw cases. All 56 per-mode records and
bytes also match the earlier frozen0051 numerical run. Original analytic and
model assertions remain unchanged. No PNG was exported by this integration run;
all four processes were reaped and independently absent before GPU release.

Claude0051's bounded pixel review is a separate acceptance record on its original
immutable binaries. The integrated numerical comparison does not expand that
visual review to new fixtures or backends. Both features remain off by default.

## Scoped Claude appearance acceptance

[Claude0051's report](../../handoffs/0051-cooked-shader-acceptance/result.md)
records explicitly verified Opus 5.5/Max and actual pixel inspection of 24
captures from eight serial, successful children. All 12 default/cooked PNG pairs
and decoded images are identical. The output swatch, diagnostic reflected cubes,
and shadow contact/slope/curve/mask/reflection/culling properties pass within the
fixed fixtures on Apple M5 Pro/Metal. The original output-only and geometry-v2
source/build identities remain distinct. No production visual changes were made.

The report also preserves five shared baseline observations: a thin-wall contact
light seam, grazing-light stair steps, far thin-caster breakup, sub-visible
receiver modulation and a faint grazing-edge fringe. None differs between the
WGSL and cooked paths. Their causes and temporal behavior remain follow-up work;
these results do not approve complete shadow quality. The masks fixture does not
distinguish the caster from the receiver in the main pass, so its appearance
coverage is limited to shadow semantics. No new capture or rebaseline was used
to hide those limitations.

The [main109 source/evidence review](evidence/cooked-shaders-main109-review.json)
preserves the earlier private unsafe-boundary review and accepts the additive
integration. Retained raw captures, child logs and helper manifests are in the
Claude packet. Its independently reproduced CPU analysis and identity review
are recorded in [the closeout receipt](evidence/cooked-shaders-0051-closeout.json).

Packet scope disclosure: Claude produced 18 magnified CPU crops and two difference
maps from the existing captures, despite the brief’s “no further images” wording.
They are ignored, labeled derived inspection aids with source hashes; they add no
render/capture coverage. The 24 original captures remain unchanged. This does not
authorize additional captures or change model routing.
