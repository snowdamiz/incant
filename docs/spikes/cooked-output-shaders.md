# Cooked output-pass shaders (opt-in)

The `incant_render/cooked-output` feature connects actual offline artifacts to the
existing display-output pass. It is off by default. The initial implementation
was CPU-validated; later Metal numerical checks are recorded in
[the geometry integration evidence](cooked-geometry-shaders.md#actual-main109-integration-and-metal-numerical-checks).
No Vulkan/DX12, reference-device, performance or release gate is claimed.

## Production path

A build-only `incant_shader_cook` dependency reads the same engine-owned
`output.wgsl` and `tone_map.wgsl`. It uses exact Naga 29.0.4 to validate the fixed
contract and emit two MSL, two SPIR-V and two HLSL stages. A generated private
manifest binds source and producer hashes, contract/version, emitted entry names
and payload hashes. Cargo's build fingerprint/OUT_DIR manages these artifacts;
there is no new persistent driver-cache system.

The native feature requests wgpu 29.0.4's `PASSTHROUGH_SHADERS` capability and
selects embedded MSL, SPIR-V or HLSL by actual backend. Unsupported capability or
backend fails explicitly. The private unsafe API boundary accepts only the
build-produced constants; it is not an artifact loader exposed to projects.
Checksums detect damage, not trust in attacker-supplied shader bytes. The default
path and the wasm cfg route still use the existing WGSL source. Other shader
families and their runtime translation remain unchanged.

The host cooker is forbidden in normal runtime dependencies. Naga still exists
through wgpu for the remaining shader families, so this is not a whole-renderer
parser/compiler exclusion claim. The only new external lock entry is petgraph
0.8.3, needed by the host test's SPIR-V reader; existing resolved versions remain
unchanged.

## Exact narrow ABI

The fullscreen draw remains three vertices and one instance. Both original WGSL
files, tone-map/composition arithmetic, the fixed 80-byte uniform and all four
output formats are unchanged. The cooked path supplies an explicit fragment-only
layout: group0/binding0 float 2D texture without filtering/multisampling;
group0/binding1 uniform, no dynamic offset, minimum 80 bytes. The vertex input is
only `vertex_index`; there are no vertex buffers, samplers, dynamic storage
lengths, workgroup memory, overrides or auxiliary shader metadata.

This boundary is pinned to wgpu-hal source revision
`e99f5305ded96ff7006f0714d043a7f735bd45c2`:

- Metal `create_pipeline_layout` allocates texture and buffer counters separately:
  the fragment resources are texture0 and buffer0. The output vertex has no
  vertex-buffer pulling or dynamic size argument. The passthrough API's missing
  dynamic-length/workgroup reflection does not apply to this bounded contract.
- Vulkan assigns the dense bindings0/1 in set0. Its command encoder already uses
  a negative-height viewport. The cooker deliberately omits Naga's default Y
  adjustment; the real emitted vertex has no extra `OpFNegate`. SPIR-V1.0 permits
  only Shader/ImageQuery capabilities, with explicit flags and bounds policies.
- DX12 allocates t0/b0 in space0. Its mandatory FIRST_VERTEX_INSTANCE root layout
  adds special constants at b1/space0, used by the generated vertex. There are no
  sampler heaps/index buffers referenced by this pass. HLSL uses shader model6.0.

Bounds restriction and loop bounding remain enabled in the emitter. The cooker
rejects contract changes rather than pretending to supply metadata for other
shader families. Public `ShaderSource::SpirV` is not used because it would reparse
and translate the source; the embedded artifact uses the actual passthrough API.

MSL and HLSL still undergo driver compilation. The independently compiled local
Metal libraries are validation artifacts, not what the runtime embeds. DXIL
compilation, Vulkan/Metal/DX12 driver acceptance and persistent pipeline caches
remain outstanding. CPU source validation cannot establish numerical or visual
equivalence; actual baseline comparisons and Claude acceptance are required
before enabling this feature by default.

## CPU checks and limitations

The [CPU freeze receipt](evidence/cooked-output-shaders.json) was captured before
independent review, so its pending-review field is historical. The subsequent
[accepted source review](evidence/cooked-output-shaders-review.json) verifies that
exact receipt and all17 changed source identities; no actionable finding was
reported within the opt-in CPU scope.

The CPU receipt binds source, logs, generated
artifacts and the unchanged baseline. Five cooker tests cover the real source,
strict contract refusals, changed-source identity, actual register mappings,
SPIR-V parse/validation and its coordinate convention. Four private runtime tests
cover native descriptor selection without WGSL, damaged/truncated/wrong-version
artifacts, unavailable capabilities, uniform offsets and the unchanged format
catalog. Two existing numerical GPU cases remained ignored. Both native default
and feature all-target Clippy checks pass, as do seven dependency-guard tests and
the actual normal-graph guard.

Independent ci/dev host compiler invocations produced byte-identical six payloads
and manifest. Apple Metal32023.921 compiled and linked both emitted MSL stages;
no driver or window was obtained. HLSL has not been compiled with DXC here.

A direct default wasm32 `incant_render` check failed with163 errors, starting with
Bevy Resource Send/Sync requirements on WebGPU buffers. The same command against
an isolated archive of unchanged main100 failed with the identical ordered163
error codes/messages. This is an existing native-renderer web compile limitation,
not a passing web check. This slice does not repair that separate threading
boundary.

Initial local compiler attempts exposed a host HashSet construction mismatch,
a missing explicitly allowed SPIR-V ImageQuery capability, and the Rust1.99
Clippy preference for fixed-size array chunks. They were corrected and their
logs retained before the final passing checks. No failed run is recorded as a
pass.

Reproduce CPU cooking without an adapter:

```sh
./tools/cargo run --locked -p incant_shader_cook --example output_bundle --profile ci -- /tmp/incant-output-bundle
./tools/cargo test --locked -p incant_shader_cook --profile ci
./tools/cargo test --locked -p incant_render --features cooked-output --profile ci --lib output::
./tools/cargo clippy --locked -p incant_shader_cook -p incant_render --features cooked-output --all-targets --profile ci -- -D warnings
```

## Prepared numerical comparison (GPU still pending)

The existing HDR tests now optionally write normalized raw RGBA readbacks with
`INCANT_OUTPUT_NUMERICAL_EVIDENCE`. This is separate from PNG capture. The twelve
HDR swatches run in all four output formats; the existing three-step resize test
also runs all four formats. Sixteen case receipts identify the dimensions, format,
actual adapter/backend/driver, shader source and raw-byte hashes. Fresh output
directories and create-new files prevent stale cases from being overwritten.
The original numerical assertions and tolerance remain unchanged.

Both default and cooked test binaries were built with `--no-run`, then copied to
separate immutable paths before the next integration; cooked all-target Clippy
passed. No GPU or screenshot was produced. The later serial comparison must
require complete case coverage, matching adapter/source identity and successful
test exits, then compare the raw bytes exactly before diagnosing any differences.
Claude retains rendered-pixel acceptance.
[Build/source receipt](evidence/cooked-output-readback-preparation.json).

The test extension is frozen at `550b563` and independently accepted for this
preparation-only scope. Its compiled source is unchanged;
[review receipt](evidence/cooked-output-readback-review.json) and
[exact commit, toolchain, feature/profile and copied-binary pairing](evidence/cooked-output-readback-binaries.json)
are retained. No GPU grant, default enablement or equivalence claim follows from
these CPU results.

## Actual Metal numerical comparison

The preserved default/cooked executables from `550b563` later ran the two existing
HDR tests serially under an explicit GPU grant. Both passed and all 16 normalized
readbacks were exactly byte-identical across all four formats and resize cases
on Apple M5 Pro/Metal. [Source/binary, adapter, case and process receipt](evidence/cooked-output-metal-numerical.json).
Both processes were reaped and independently confirmed absent before release.
This supersedes the earlier pending-Metal-numerical status only. No PNG or pixel
review occurred; Claude appearance, Vulkan/DX12 and reference-device acceptance
remain pending. The feature is still off by default and there is no timing claim.
