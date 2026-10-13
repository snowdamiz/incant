# Fixed-resolution temporal accumulation: curated main217 integration

This private integration adds explicit, opt-in fixed-internal-resolution temporal
accumulation to main217 (`1da6cab9778913999a745803445f209d4da0f90d`). It selects
the accepted fixed implementation from
`e23776df275fa1de81b2027110d9c7b6916999ac`; it does not merge that producer's
portable upscaler or native MetalFX ancestry. Neither the renderer nor any
authored project enables accumulation by default. Bloom and SSAO are separate.

## Ownership and rendering contract

`Renderer::enable_temporal_accumulation` prepares the fixed pipelines, and
`RetainedScene::enable_temporal_history` accepts a checked explicit policy.
Opaque and MASK models contribute color, motion and depth under the same retained
owner. Sequence, epoch, geometry, extent and jitter phase advance only after the
owning command buffer is submitted. Preparation/refusal leaves the last accepted
history intact. Pixel cameras bypass temporal history; viewport/DRS size changes
reset the fixed history. This is accumulation at the current internal resolution,
not reconstruction to a larger output resolution.

The accepted coverage shader keeps premultiplied coverage at depth and MASK
boundaries. Clear alpha is canonical zero, expected previous depth comes from a
bounded current 3x3 neighborhood, and positive valid history support is normalized
before clamp/reaction. The three fixed WGSL files are byte-identical to e237:
motion/depth share `motion/shader.wgsl`, resolve uses `temporal/resolve.wgsl`,
and presentation copy uses `temporal/copy.wgsl`. No shader arithmetic is changed
by this integration.

Transparent models, sprites and text remain current-only and unjittered. A
separate unjittered opaque/MASK depth pass protects their depth test. Existing
physical UI and output mapping remain main217's implementation. The fixed
projection uses the accepted conservative light-grid and shadow-cascade margins.
History resources retain the accepted checked size/transition caps; these bounds
exclude ordinary renderer targets and driver/in-flight allocations as documented
in the source.

The Models timestamp interval now begins at its first actual motion/model pass
and ends at its last actual fixed resolve/copy/overlay-depth/transparent pass.
Interior passes do not write duplicate timestamps. Main217's bounded min/max
frame-envelope decoder, optional-adapter admission and other six timing groups
are preserved. The frame envelope still measures instrumented extrema and gaps,
not complete GPU frame or presentation cost.

## Native cooking and admission

The new opt-in `cooked-temporal` feature closes over the existing cooked material,
geometry, output, sprite, UI, text and cluster families. The private program
catalog is narrowed to exactly four kinds: Motion, Depth, Resolve and Copy.
Each emits Metal, Vulkan and DX12 vertex/fragment artifacts with exact source,
producer and contract hashes. The accepted 144-byte motion frame, 64-byte material
and fixed 64-byte resolve/copy frame layouts, output formats and backend binding
maps are preserved. The jitter-aware fixed cluster variant is cooked explicitly.

Native partial cooked configurations continue to refuse motion/temporal admission
with a typed error; they never silently compile missing WGSL programs. The default
and browser paths retain their WGSL behavior. All narrowed artifacts must be
regenerated from this exact cooker: earlier producer artifacts are historical
evidence, not artifacts built from this composition. No new lock/dependency is
introduced. No upscaling, native command-buffer, MetalFX interop or extra-output
program enters this catalog.

The three existing release-only cooked CI feature selections also enable
`cooked-temporal`, retaining default runs, actual backend assertions, the isolated
Vulkan cache lifecycle and all required contexts. This makes the fixed temporal
device tests execute their supported path in the cooked configuration. It does
not alter the currently frozen checkpoint22.

## Source composition and evidence limits

The temporal family, its numerical oracle, coverage controls and retained owner
tests come from exact e237 source. Main217's renderer constructor, cache support,
HUD/font changes and timing decoder remain the base. The graph and retained owner
are narrowed to the one ordinary command-buffer path. Motion's fixed jitter owner
comes from `e73afe43ffaf78e73b725876411128d092791f5a` because e237's later additions
to that file only support the excluded output-resolution upscaler. The cooker,
build catalog, host validator and their tests omit only the excluded programs.
An integration regression covers the extra overlay-depth timing interval.

Claude's accepted producer fixed-mode review (`9927598c`) includes 150 numeric
resolves and 79 scene submissions. It found reduced static shimmer without stale
ghosts, silhouette erosion or overlay contamination in those fixtures. Softness
and MASK detail of approximately 18% of the reference remain explicit quality limitations;
acceptance is for opt-in fixed mode only. It does not approve default quality,
upscaling, MetalFX, phone performance or a phase gate.

Current composed source review, CPU/consumer checks and explicit offline recook
are pending. No GPU execution or current-composition appearance acceptance is
claimed. Original producer execution remains tied to its own source and binary.
