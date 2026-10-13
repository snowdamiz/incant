# SSAO, bloom and dynamic resolution composition

The default-off SSAO and bloom producers are composed with the accepted clustered
lighting and dynamic-resolution renderer. Bloom reads the same internal scene
coordinates as the base HDR sample, including odd sizes and offset viewports;
physical UI and output bounds stay unchanged. Bloom uses the actual internal
frame size/scene rectangle. Pixel cameras bypass bloom, and zero intensity remains
an exact off control. Existing cooked-feature refusal rules remain explicit.

Independent source review accepted the composition and the numerical bloom
alignment fixture. Frozen CPU validation passes eleven commands: shader cooking
(25 cases), default renderer/host (98), all cooked families (125), Clippy, native
window compilation, both browser feature configurations, dependency guards,
formatting, conventions and diff checks. Counts overlap; 94 GPU cases in each
renderer variant were intentionally ignored. No actual rendered acceptance is
claimed. Claude0070 reviews both effects together, including dynamic-resolution
mapping and exclusion semantics.

Two earlier attempts remain in the [evidence](evidence/render-effects-composition.json).
The first exposed a stale source-string assertion, corrected to the actual mapped
coordinate. The second loaded an old shader-cook artifact from a reused target;
cleaning only that package and rebuilding it before the suite resolved the
missing-module error without a source change. The final frozen source is
`b633f92d9d20cd8ed4c328b0b670ab7935797801`.

This does not implement temporal reconstruction, MetalFX or reference-device
performance validation. GPU timings and retained motion history are separate
producers and still need coordinated integration.
