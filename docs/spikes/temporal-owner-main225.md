# Temporal placement provenance on main225

The accepted producer `af7efed1a89f51e33bfc2fa75f2da046b428dcf5` is integrated as
an isolated eight-file Rust patch on actual main224. Its private temporal
upscaling/MetalFX ancestry is absent. A temporary Git index independently reapplies
the exact producer patch to main224 and compares all eight resulting files.
Shader bytes, render targets, configuration defaults and memory limits are unchanged.

Source `9e9abec425033c59b9691503448d14d190a95780` passes 143 default and 172
cooked CPU cases in ten groups each, with 103 GPU cases ignored in each suite.
The suites overlap. Both all-target strict Clippy commands, wasm32 compilation,
native-window consumer compilation, normal/runtime dependency guards, formatting,
conventions and full-base diff checks pass: twelve final command receipts.
Four copied executable identities are retained externally; no GPU was executed.

The first attempt failed because the shared warm target reused old `incant_text`
and `incant_render_data` products without the current HUD halo APIs. Cargo marked
them fresh although the frozen source contains the missing fields and export.
Their hashes and the original compiler errors are retained. Cleaning only the
owned `incant_text`, `incant_render_data` and `incant_ui` package products recovered
the full check chain without changing source. No broad dependency-cache deletion
or production workaround was used.

Ancestry commit `11ce57b5` joins actual main225
`d62976cc5adc740b5b0541abcbfd59d4069b4a45`. Its only executable-scope difference
from the tested source is the separately accepted command-journal test isolation.
The verifier binds both versions explicitly; it does not claim another renderer
build after this disjoint join. All other 3,144 frozen inputs match exactly.

The producer's 56 retained artifact records, 54 payloads and eight copied
executable identities are rehashed in addition to current integration evidence.
The original root acceptance remains byte-identical. Current-main final review
is independently accepted in the [integration manifest](evidence/temporal-owner-main225.json).

This establishes stable accepted placement/change metadata only. Current float
shaders do not transport integer ownership. Per-pixel provenance, contributor
tracking and temporal/native reconstruction repair remain separate work. There
is no new appearance, GPU, phone, performance or phase-gate claim.
