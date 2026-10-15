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
existing refusals. This original qualification compiled the player only; the
separate public export below adds its first CPU startup. Neither result establishes
GPU rendering, native driver compilation, image parity, cache hits or the full
shader-stall/performance/phase gates.

The public exporter subsequently built one fresh shipping package at immutable
source `340acb7cfe7da8cc7c959f60730feb937b0a9d31`. It used Rust 1.99.0
(`b940084d7`), Node 22.23.3 and macOS SDK 27.0, locked/offline Cargo resolutions,
two Cargo jobs and two Rayon threads. The existing 74-file Core authoring package
was unchanged. Its project SHA-256 is
`027621691da3043e25cd30a02c76a8fb92f1a4464448c3c6c7d04e6f8b38c30b`.
The unsigned arm64 package selects fast physics and `project,native-window,cooked-native-shaders`;
profiling, pipeline caching and optional observers remain disabled. The audited
normal dependency graph excludes authoring and JIT crates. Raw target invocations
verify shipping optimization, one codegen unit, aborting panics and fat final LTO.

The actual shipping output passed all twelve family audits: 69 artifacts totaling
268,438 bytes. Its source, producer, manifest and artifact identities exactly
match the retained CPU005 shader families. The unmodified exporter deleted its
private build target before startup; raw generated bytes from that fresh target
were not copied. The earlier CPU packet supplies the separately labelled raw
shader bytes, while the fresh export receipt supplies its actual audit hashes.
The 334,036,395-byte package and executable remain outside Git. Player SHA-256:
`0e98ff61a4c046a64a5e21cc251c2078d3a2619514fe2f7d5381a62101656cd0`.

Its sole inherent startup child completed **one gameplay tick**, loading the
cooked scene with source parsing disabled. Render input reported `cpu-only` and
`gpu_submitted:false`; `frames:800` counts PCM sample frames, not gameplay ticks.
Although the package supports a native window, the startup argv omitted `--window`.
All 18 managed calls completed with observed successful cleanup. Their process
groups were independently absent afterward, and the private target was empty.
Peak sampled scratch use was 4,324,769,042 bytes under the 8 GiB cap; sampled free
space stayed above the 6 GiB floor. The 3,000-second cooperative execution bound
allows mandatory cleanup and blocked filesystem work to finish afterward.

The packet preserves export002's Cargo failure because the checkout lacked its
pinned TypeScript package; no package or player was produced by that attempt.
The authorized offline `npm ci` then refused an uncached tarball. An intermediate
setup proposal also refused before any child because it incorrectly assumed a
partial dependency directory. The corrected pinned registry install used the
unchanged lock with scripts, audit, funding and workspaces disabled. Actual imports
verified TypeScript 5.9.3, SWC 1.16.13, sourcemap-codec 1.6.0 and a native SWC
transform. Export003 then passed. The closeout rehashed all 4,490 source/copy inputs,
7,620 installed compiler entries, 74 authoring files and 38 package files unchanged.

[The public-export packet](evidence/cooked-bloom-export/manifest.json) retains
commands, raw outcomes, failures, cleanup and inventories; full binaries,
`node_modules` and large Cargo/nm stdout remain external with hashes. Replay uses
only retained data and Git blobs:

```sh
python3 docs/spikes/evidence/cooked-bloom-export/verify.py
python3 docs/spikes/evidence/cooked-bloom-export/verify.py --originals --binaries --git-source 340acb7c
```

This documentation-only intake starts from main335 `33072b90`; it does not relabel
later agent/profile or early exporter-prerequisite changes as producing inputs.
The earlier CPU source `0067361d` and its help-only exporter successor also retain
their distinct identities. `--source-root` requires the exact producing inventory
and will refuse a later changed checkout. No native window, browser, capture, GPU
or driver execution occurred during this export. Rendered parity, cache hits and
the full shader-stall gate remain open.
