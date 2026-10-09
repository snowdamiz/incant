# HDR scene output and display composition

Phase 1 increment, 2026-10-09. Geometry and transparent material layers now render
into RGBA16Float with a shared depth attachment. A subsequent fullscreen pass
transforms scene radiance into display color, composes the display-referred
viewport backdrop and native rounded mask, and applies exactly one sRGB transfer.
The native editor, CLI screenshots and simulated frame captures share this path.
Output remains SDR; this does not add an HDR-monitor presentation mode.

Claude Opus 5.5 over ACP owns the preview curve and pixel review. Its final curve
uses the hue-preserving shoulder derived from Khronos PBR Neutral, with identity
below a peak channel of 0.8 and fixed exposure 1.0. It deliberately omits the
reference's Fresnel offset. Actual capture review found that offset made muted
colors too saturated and crushed dark fill-lit surfaces in this lighting setup.
The derived curve is not Khronos PBR Neutral conformance or an ACES pipeline.
The pinned source, modifications and Apache license are recorded in
[third-party notices](../../THIRD_PARTY_NOTICES.md), included in development app
bundles and hosted desktop binary artifacts.

Transparent scene layers blend in linear HDR before the transform. Scene alpha
represents coverage over transparent black. The output pass unpremultiplies the
combined scene, transforms it, and composites the display backdrop by coverage.
This keeps editor colors outside the scene exposure while opaque and translucent
scene layers still share the same HDR lighting arithmetic. The output has alpha
one. Geometry radiance saturates only at finite half-float maximum 65504 before
storage. Four supported display formats are RGBA8/BGRA8, sRGB and non-sRGB; other
formats fail with a typed error rather than silently choosing a transfer function.

The renderer retains one color/depth attachment pair for the current dimensions.
Same-size frames reuse it. Encoded commands retain replaced attachments across
resize. Dimensions are limited by the device and 8192 per axis, with a total
16,777,216-pixel budget (192 MiB color/depth). Outstanding command buffers can
retain older sizes until released; this is not a global application memory cap.
Scene resources and material versions retain their previous ownership behavior.

## Verification

- 124 workspace Rust behavior tests, workspace Clippy and formatting pass.
- 282 UI/bridge tests, UI build and generated convention/bridge/SDK checks pass.
- Eleven explicit native GPU tests pass on Apple M5 Pro: two display-pass probes,
  seven geometry/material probes, editor revision recovery and headless playback.
- Known HDR samples from zero through 65504 match independent expected values.
  RGBA/BGRA sRGB/non-sRGB targets match within one output code value, including
  the native rounded mask. The #141519 backdrop and supplied chrome colors retain
  their display values. Repeated and queued resizes preserve output.
- A glossy glTF surface produces radiance above one and a controlled shoulder;
  a white half-alpha layer over black proves blending precedes tone mapping.
  Fill-only gray materials with albedos 0.1, 0.18 and 0.5 remain distinct at sRGB
  48, 66 and 108. Color-space, sampler, reimport, Undo/revision retention and source
  removal tests also pass through the new output path.
- Final headless GPU captures and native CUA captures are retained under ignored
  artifacts directories. Native reopening restores the saved project, history and
  account, with the viewport attached. Final Claude visual review is pending.

This is a fixed ordered renderer, not the complete production render graph.
Clustered forward+ lights, specular IBL, cascaded shadows, SSAO, bloom, TAA,
feature tiers, authored render settings, automatic exposure and performance gates
remain open. Metallic surfaces still lack environment reflections. Transparency
uses the existing primitive-center order, which does not solve intersecting
transparent geometry. No phase gate is approved by this increment.
