# Complete backend-cooked SSAO

`cooked-ssao` prepares the existing material prepass, diffuse-occluded forward,
sampling, horizontal-filter and vertical-filter programs from trusted engine-owned
source. The public `--shaders cooked` aggregate includes all five: seventeen
families, 99 artifact-bound shaders, 427,087 bytes. This extends the historical
[twelve-family bloom result](cooked-bloom.md); it does not relabel that earlier
shipping export as a new export.

AO stays off by default. Ordinary and web configurations retain their WGSL path;
native partial `cooked-output` and `cooked-materials` builds explicitly refuse AO
before device preparation. Complete selection validates kind, exact source,
producer, host ABI and embedded artifact identities before creating any SSAO
shader module or pipeline. No selected SSAO program falls back to runtime WGSL,
and project data cannot supply native shader bytes, entries or paths. Existing
motion/temporal and MetalFX routes remain separate; this is not whole-renderer
translation removal.

All production WGSL and shader equations are unchanged. The material source
assembly is shared by runtime and host cooking. Independent Naga contracts check
stage visibility, centroid interfaces, resource bindings and the 736-byte sampling
uniform. MSL, SPIR-V and HLSL emission preserves backend-specific vertex pulling,
sampler indices, storage lengths and bounds checks. A pinned main346 material
emitter fixture proves all six ordinary-material backend artifacts and entry
names remain byte-identical. Their producer identities conservatively change
because the shared cooker source changed.

The existing cold preparation still receives the retained device cache and only
publishes the complete candidate after the validation scope closes. Pass order,
resource lifetimes, finite pipeline variants, AO settings/limits and the existing
AO-plus-4x-MSAA refusal are unchanged. AO still affects only diffuse environment
light on opaque/MASK models; other lighting/material paths retain their behavior.

Rust 1.99.0, locked/offline arm64 CI builds used two Cargo jobs and two Rayon
threads. Qualification passed 47 cooker tests, 23 default AO CPU tests, 212 full
aggregate/cache renderer CPU tests, both one-case partial-mode refusals, strict
all-target Clippy, 23 public shader-audit Python controls and four CI-profile
controls. Three harmless supervisor controls verified successful cleanup, timeout
cleanup and failure reporting. The native-window/profile player was compiled
only. Its actual compiler-selected output passed the complete 17-family audit,
including the 30 SSAO artifacts; all 133 generated files were retained.

The first cooked renderer compile failed on two host-check names: the material
module is nested under `material_pipeline`, and Rust calls the uniform field
`parameters` where WGSL calls it `values`. The correction adds a narrow internal
reexport and checks the same byte-208 offset. The earlier 47 cooker and 23 default
AO passes remain bound to their unchanged inputs; the remaining sequence passed.
The exact frozen working tree against main346 `25f64814` was subsequently recorded
byte-for-byte as source commit `1dc382a6`. The packet preserves both source
inventories, the compile failure and the correction.

Four retained-binary controls then passed on **Apple M5 Pro / Metal**. In both
default and full-aggregate configurations, the production control actually
prepares SSAO, draws imported models, preserves each mode's ordinary output at
zero strength, resizes/fractionally positions the viewport, consumes queued
commands after owners retire and retains settings after refusal. Thus the cooked
Metal modules and finite pipeline catalog were admitted by the real driver.
The separate R8/Depth32Float plane-filter oracle also passed in both binaries;
that helper explicitly constructs WGSL and is not a separate native-selector
proof. Metal deliberately has no persistent-cache support in this implementation,
so this establishes successful preparation through the cache-aware path, not a
cache hit.

A further test-only fixture closes positive-strength numerical parity. The real
importer loads an opaque receiver and a textured MASK occluder at different
depths; native inputs remain valid after the fixture source/cache is removed.
Both binaries draw the same fixed camera with the existing diffuse environment,
with AO disabled, strength zero and strength one. All six raw 128×96 RGBA buffers
were retained before assertions. Every default/cooked frame pair is byte-identical
with zero tolerance. Zero strength equals disabled output; strength one darkens
1,231 pixels by at least eight output codes, with a maximum reduction of 52.
Alpha and coverage are identical, no RGB channel brightens, and 972 chromatic
pixels establish that the imported texture actually reaches output. These are
fixture observability checks, not an AO appearance rating.

Two new CPU controls pass in each mode, as do both strict all-target lints and
four independent comparison-refusal controls. The first positive attempt passed
both CPU tests per mode, then strict Clippy rejected three fixed-size iterator
calls. The final test-only correction uses equivalent `as_chunks` iteration;
fixture bytes, thresholds and production remain unchanged. Source commit
`474e8ee6` records the exact 125 final input bytes after CPU/GPU qualification;
production remains `1dc382a6`.

Every owned process group was cleaned up and independently absent. Final target
growth across all CPU attempts was 255,667,777 logical bytes under the original
4 GiB bound; free space stayed above 6 GiB. No player, native/browser window,
screenshot or capture ran. Full-strength AO appearance/quality, actual Vulkan/Dx12
execution, a new public shipping export, cache hits and the shader-stall/
performance/phase gates remain unqualified.

[The compact packet](evidence/cooked-ssao/manifest.json) contains exact source,
commands, raw outcomes, generated shaders and hashes of external executables.
Replay starts no compiler, player or device:

```sh
python3 docs/spikes/evidence/cooked-ssao/verify.py
python3 docs/spikes/evidence/cooked-ssao/verify.py --originals --binaries --git-source 474e8ee6
```

`--source-root` additionally requires unchanged producing code inputs; later
compositions with different inputs must not be relabelled as the producing build.
