# Explicit cooked shaders in native-window exports

`tools/export_native.py PROJECT OUTPUT --mode window --shaders cooked` selects
existing fixed backend-cooked shaders through the trusted exporter. Omission
retains the ordinary configuration. Unknown modes, headless selection and
unsupported native targets refuse explicitly. This is source implementation;
no new Rust compilation, shipping package or GPU execution has qualified it yet.

The player feature `cooked-native-shaders` propagates a finite renderer selection:
output, diagnostics, shadows, PBR materials, sprites, world text, screen-space UI
and clustered-light compute. The existing host cooker translates engine-owned
WGSL to MSL, SPIR-V and HLSL. Native renderer ABI checks and embedded artifacts
remain the only executable shader admission path. Project files, data caches and
runtime arguments cannot supply native shader bytes. The shader cooker remains
a build dependency and is explicitly forbidden in the player's normal graph.

Each selected family emits deterministic build-only `audit.json` alongside its
existing private Rust manifest. Audit identity covers the exact source, producer,
contract, embedded manifest, native entry points, file sizes and artifact hashes.
The audit emitter enters the producer identity. All resolved dependency versions
stay pinned; only already-resolved JSON and test-tempfile edges are added to the
host cooker. The exporter admits eight complete families and 45 artifacts, each
at most 1 MiB; manifests/receipts are separately bounded at 64 KiB per family.
It reconstructs the actual embedded Rust manifest from the receipt and verifies
all bytes. Missing, duplicate, changed, linked, oversized or foreign artifacts
refuse package publication. The compiler-selected private target OUT_DIR is the
sole file source; the project supplies no path. Generated audit files are build
evidence, never a runtime artifact loader.

Actual Cargo target units must include both player library/executable and the
renderer library. Build-host units are reported separately and cannot supply
runtime coverage. The exact renderer cooked feature set is closed; unexpected
optional families refuse. Fast/deterministic physics and profiling retain their
existing independent audits. The published export receipt records the accepted
family and actual target-feature inventory. The CPU one-tick startup remains a
CPU proof and does not create a GPU or qualify this shader selection.

This subset does **not** remove all renderer WGSL routes. Bloom filters and bloom
display output, SSAO prepass/shading/filter, motion/temporal programs and MetalFX
prepare/normalization remain separate source paths. Existing cooked-mode refusal
for bloom, SSAO, incomplete motion/temporal and MetalFX is preserved. None is
silently translated at runtime as a fallback for the selected ordinary catalog.
The web route still uses WGSL. The WGSL parser may remain linked through wgpu;
parser-symbol exclusion is not asserted. MSL/HLSL still undergo native driver
compilation during cold preparation; these artifacts are not Metal binary
archives or DX12 pipeline libraries.

The ordinary finite pipeline catalog already prepares its supported keys before
returning a renderer; this slice does not change pipelines, render graph,
shader arithmetic, layouts or appearance. Persistent Metal/DX12 caches, per-level
recorded permutation coverage, asynchronous future-permutation fallback, deferred
driver behavior and the whole rendered-game shader-stall gate remain open. No
performance, cache-hit, appearance or phase-gate claim follows from source or
pure Python controls. Screen/window capture and CUA remain prohibited.
