# Temporal edge correction: CPU verification

The [frozen correction](temporal-edge-correctness.md) has seven passing CPU checks:
native renderer/host tests, all-target Clippy, wasm32 library compilation,
cooked-feature tests, explicit-root runtime dependency guard, formatting and
generated conventions. The [manifest](evidence/temporal-edge-correctness.json)
binds 24 source/doc identities, 11 unchanged baselines, 22 losslessly retained
records and eight copied executable test binaries.

The native run passes 120 tests in nine groups, with 98 GPU cases ignored. The
cooked-feature configuration passes 125 tests with 40 ignored; these overlap the
native suite and are not additional unique coverage. No GPU case was executed.
The wasm32 build retains five existing optional-timing warnings; native Clippy
passes with warnings denied.

Two earlier failures remain preserved: the shared MASK helper initially changed
a literal expression asserted by the canonical-material test; the final helper
preserves that expression and both fragment entrypoints call it. The final chain
then omitted mandatory package roots from the dependency-guard command. That
harness invocation was corrected separately without changing frozen sources.

The copied native library binary is executable and contains the expanded ignored
temporal and upscale fixtures (14 and 19 tiny submissions respectively). Actual
shader execution, valid-support efficacy, overlay depth/order and stationary
appearance must be reviewed by Claude. Compilation and CPU oracles do not accept
the previously rejected visual result. The [independent source review](evidence/temporal-edge-source-review.json)
accepts the frozen implementation without findings; its scope excludes execution
and appearance acceptance. Implementation commit: `b0b4dcf096b738673c4023357a82b67bdb863489`.

All owned check processes are reaped and the private target is released. Root
owns the subsequent MetalFX and cooked-temporal union, including explicit new
artifact contracts and recooking; this producer does not change those older
receipts or claim their integration is complete.
