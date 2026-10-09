# Directional shadows: look-dev, pixel review and Inspector presentation

Claude Opus 5.5 through ACP owns appearance, UI and rendered-pixel review. Astra
implemented the shadow math, GPU resources, draw scheduling, shared document
settings and tests. Read docs/spikes/directional-shadows.md. This packet is an
initial review; native captures and final measurements will follow. Do not claim
native approval before receiving them. Use your own worktree target directory.

The director rejected assets sharing the cramped bottom diagnostics dock. That
feedback was implemented in handoff 0010: Assets is a dedicated left workspace,
asset details use the main Inspector, and bottom output is Problems/Console/History.
Preserve this organization and the restrained neutral charcoal palette. Any new
Inspector rows must respect its spacing and minimum-window behavior.

## Implement and review

1. Optional DirectionalLight.shadows is now a nullable schema reference to an
   object with distance in meters. The native bridge resolves local definitions
   and emits FieldSchema optional/nullable flags. A missing optional value or an
   explicit nullable null means shadows are disabled. The UI currently treats
   such values as a mismatch. Implement a clear, quiet read-only disabled state
   in FieldView (all optional fields generically), and present enabled distance
   using the existing grouped Inspector language and unit metadata. No new edit
   controls, toggles or second mutation path. Required missing values must still
   show a mismatch. Add meaningful behavior tests for optional/nullable/enabled
   cases. Do not edit bridge/native or Rust schema correctness code.
2. Review actual shadow PNGs copied into artifacts/shadow-initial, and public
   CLI captures in artifacts/shadow-cli. Cover contact/bias, filtered boundaries,
   correct direction, retained versions, two-sided/reflected alpha holes,
   offscreen casters, multiple suns, four cascade boundaries and distance fade.
   Numerical fixture quads are deliberately analytical, not game art. Review
   whether the shader bias/filter produces unacceptable artifacts. You may
   improve appearance in src/shadows/shade.wgsl, caster.wgsl or pipeline.rs, with
   before/after captures and rationale; coordinate substantive math/resource
   changes through result.md for Astra. Do not weaken numeric tests to hide bugs.
3. Additional look-dev fixtures are welcome when needed to assess sloped/curved
   surfaces or contact. Generate actual engine-rendered pixels using real cooked
   geometry; never substitute fake shadows/images. Keep helper artifacts ignored.
4. Existing unshadowed capture regression evidence is in shadow-regression and
   graph-initial, plus graph-final stable fixtures. shadow-pixel-comparison.json
   records 121/123 exact pairs. Two camera-clusters-1-480x270 members differ at
   one pixel by one red-channel level (120 versus121). The original fixture used
   random light ULIDs, hence summation order could vary. Astra has now assigned
   fixed ULIDs in construction order; shadow-camera-stable and shadow-camera-repeat
   contain two independent, exact repeat captures and shadow-camera-stability.json
   records them. Verify both actual differences and repeatability.
5. Browser pixel review at 1440x900 and 1000x650 for enabled/disabled Inspector
   states. Retain existing titlebar and Assets/output separation. No landing
   redesign. Run npm test/build --workspace editor/ui; no new dependencies,
   remote fonts or main bundle above 110 KiB gzip (current 101.77 KiB).

## Runtime scope / acceptance

Opt-in four 1024x1024 depth cascades, up to four enabled suns. Camera-fitted,
texel-snapped projections include offscreen caster depths; ten-percent overlap
blending and final fade. Imported MeshRenderer.cast_shadows governs casting.
OPAQUE/MASK, mirrored/double-sided casters work. BLEND casters explicitly fail
with a typed error; cast_shadows=false permits them. No local-light shadows,
adaptive quality, animation/skinning, area shadows or production game gate claim.

Initial checks: 144 ordinary Rust,38 native GPU and284 UI tests, Clippy pass.
Public CLI command-bus edit/Undo/Redo, invalid-batch atomic rejection, durable
reopen and source-free cached rendering pass. Five new GPU cases test analytical
occlusion, all cascade splits/fade, alpha/reflection, offscreen/multiple lights,
and reverse submission of five queued camera/light/atlas versions.

Write result.md with exact model/transport, changes, evidence, outstanding native
requests and scoped verdict. Commit completed work with Built-by: claude. Keep
raw captures/account labels ignored. No credentials/account actions, publishing
or merging. Native automation/capture uses CUA only; this ACP session has no
native CUA, so leave precise requests for Astra. Browser fixtures are allowed.
