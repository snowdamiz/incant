# Complete backend-cooked bloom

The new `cooked-bloom` renderer feature prepares all four existing bloom programs
from trusted engine-owned source: prefilter, downsample, upsample and display.
`cooked-native-shaders` includes this complete family. Partial `cooked-output`
builds continue to refuse bloom before device work; no selected program falls
back to runtime WGSL. Ordinary and web configurations retain their existing WGSL
path. Bloom remains an explicit cold renderer configuration, off by default.

The three filter programs retain the shared filter source and select their exact
entry point during host cooking. The display program shares one guarded source
assembly with the existing runtime path. Source identities and independent Naga
IR contracts bind entry names, stage visibility, textures and the 48-byte filter,
80-byte display and 16-byte gain layouts. The host cooker emits MSL, SPIR-V and
HLSL for each vertex/fragment pair. Explicit runtime layouts and private embedded
selectors validate all four families before preparation. Project files cannot
provide shader bytes, entries, layouts or artifact locations.

The trusted exporter now requires twelve complete families / 69 artifacts when
`--shaders cooked` is selected. The four bloom audits use the same compiler-selected
target output and bounded artifact checks as the ordinary catalog. The display
source replacement is checked exactly once in both build/runtime assembly and
the export audit. Missing bloom programs explicitly refuse the complete catalog.

No WGSL arithmetic, bloom gain/coverage, normalized area filtering, pass order,
target lifetime, pixel-camera bypass, resource limits or timestamp placement
changes. The cold constructors accept a pipeline-cache argument and forward it
to all seven bloom PSOs (three filters and four output formats). The separately
reviewed optional-cache implementation retains that cache on `Renderer`. The
joined `enable_bloom` supplies this device-owned cache and passes its typed
candidate through `complete_pipeline_preparation` only after the GPU validation
scope has closed.

CPU qualification used source `0067361d`, Rust 1.99.0, an explicit arm64 CI target,
locked/offline dependencies, two Cargo jobs and two Rayon threads. All 41 Python
controls (21 shader-selection controls plus 20 exporter boundary controls), 41
cooker tests, 209 renderer controls and strict all-target Clippy passed. The
partial `cooked-output` bloom filter passed 12 controls; three device controls
were ignored and 223 unrelated tests filtered out. The full renderer suite left
54 device controls ignored. The native-window fast-physics/profile player was
**compiled only**, with cache disabled to match the ordinary export feature set.
Its actual compiler-selected output passed the closed twelve-family audit:
69 shader artifacts totaling 268,438 bytes, plus 24 manifests/audits. All generated
files and four qualified binaries were copied outside the reusable build target.

Two failed attempts remain in the packet. CPU003 passed 15 tests before a bloom
test confused Naga's special constant buffer with a shader uniform at `b1`.
The correction checks exact uniform `cbuffer` declarations separately from the
special buffer for both stages of all four programs. CPU004 then passed all 41
cooker cases but failed renderer compilation because the embedded display
selector's `pub(super)` visibility excluded its sibling output caller. The
one-line `pub(crate)` correction changed no shader, ABI or rendering equation.
CPU005 completed every stage. The earlier unexecuted protocols and Python
fixture mistakes are retained as such. Target growth across all native attempts
was 221,256,083 bytes, below the original 4 GiB cumulative cap; the final free
space exceeded the 6 GiB floor. These storage observations are not GPU bounds.

[The compact packet](evidence/cooked-bloom/manifest.json) binds immutable source,
raw commands/outcomes, independent source review, all generated shader bytes and
hashes of external binaries. Replay launches no compiler, player or GPU:

```sh
python3 docs/spikes/evidence/cooked-bloom/verify.py
python3 docs/spikes/evidence/cooked-bloom/verify.py --originals --binaries --git-source 0067361d
```

`--source-root` additionally requires unchanged producing code inputs; it must
not be used to relabel later changed source as the producing build. Prior
eight-family CPU evidence remains bound to its original source in
[the shipping selection spike](shipping-cooked-shaders.md). SSAO,
motion/temporal and MetalFX remain separate source/cooked routes with their
existing refusals. No public shipping export, player execution, GPU rendering,
native driver compilation, image parity, cache hit, window/browser/capture or
shader-stall/performance/phase gate is established by this CPU-only result.
