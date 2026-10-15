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
to all seven bloom PSOs (three filters and four output formats). The separately reviewed optional-cache implementation now retains that cache on
`Renderer`. The joined `enable_bloom` supplies this device-owned cache and passes
its typed candidate through `complete_pipeline_preparation` only after the GPU
validation scope has closed.

The 21 Python metadata/selection controls pass. Rust host emission, SPIR-V
validation, embedded-selector and existing bloom CPU/device controls are prepared
but unrun at this source stage. No compiler, native player, GPU, window or capture
has qualified this extension. Prior eight-family CPU evidence remains bound to
its original source in [the shipping selection spike](shipping-cooked-shaders.md).
SSAO, motion/temporal and MetalFX cooking/refusal remain separate. Native driver
compilation, shipping runtime parity and the full shader-stall gate remain open;
source cooking alone establishes none of them.
