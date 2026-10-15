# Explicit cooked shaders in native-window exports

`tools/export_native.py PROJECT OUTPUT --mode window --shaders cooked` selects
existing fixed backend-cooked shaders through the trusted exporter. Omission
retains the ordinary configuration. Unknown modes, headless selection and
unsupported native targets refuse explicitly. Local CPU qualification passed on
source `42b24f78dfcaf34a0bfce84252abc13429acbd87`; a complete public export and
runtime rendering with this selection have not run.

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
performance, cache-hit, appearance or phase-gate claim follows from this CPU
qualification. Screen/window capture, CUA and native-window launches remain
prohibited.

## Local qualification and retained failures

The [portable packet](evidence/shipping-cooked-shaders/manifest.json) retains the
frozen source, exact commands, actual Cargo messages, raw logs and generated
shader bytes. Rust 1.99.0 ran arm64 CI-profile, locked/offline, jobs 2/Rayon 2.
The successful checks were 39 Python controls, 36 cooker tests (including two
new audit controls), 188 cooked renderer CPU tests with 54 ignored GPU tests,
strict all-target Clippy for cooker/renderer/player, the fast-mode cooked
native-window player build, and the ordinary feature-off player check. The
renderer fixture suite uses deterministic physics; the actual player/family
audit separately uses fast physics. The compiled player was never executed.

The actual compiler-selected target output admitted all eight families and 45
artifacts totaling 198,123 bytes. The copied player SHA-256 is
`90a85262ccc8fe0c70d3503739c6b626fe44c563a4725213107d1651ef24c5b8`.
CPU 001 retained three renderer failures caused by deterministic fixtures in a
fast-mode test build (185 passed); CPU 002 corrected only that command and passed
188 tests, then stopped the player build at the declared cumulative 2 GiB target
growth limit. Root separately granted a 4 GiB cumulative limit for CPU 003.
Its four remaining stages passed without source changes. Final target growth
was 2,856,601,093 bytes from the original baseline; the supervisor and owned
children were reaped before the target was released. The first CLI expectation
failure is also retained as a transcript-derived observation, not a raw log.

Replay performs data validation only and never launches a player or compiler:

```sh
python3 docs/spikes/evidence/shipping-cooked-shaders/verify.py
# On the producing machine, also verify original files and retained executables:
python3 docs/spikes/evidence/shipping-cooked-shaders/verify.py --originals --binaries \
  --git-source 42b24f78dfcaf34a0bfce84252abc13429acbd87
```

`--source-root` additionally requires unchanged producing code/build inputs and
refuses later compositions that differ; the updated prose is excluded from that
code-input comparison. Neither the CI profile nor the data replay establishes a
shipping-profile runtime, target-driver pipeline creation or rendered shader-stall
result. Native MSL/SPIR-V/HLSL emission here is host-cooker evidence; only this Mac
was compiled, and no other backend execution is implied.
