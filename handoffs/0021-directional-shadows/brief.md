# Supplemental minimum capture, report only

Your40d1939 report correctly flagged the1720x669 capture. CUA moved to the
built-in display again, verified the menu offered moving back to the external
monitor, then resized. New assets-retina-minimum.jpg is actually2002x1302,
verified by decoding it. It is in artifacts/shadow-native-polish, with updated
dimensions.json and sanitized AX. Please review this single image and append
its scoped acceptance (or concrete issue) to result.md. No code changes or
unchanged checks needed. This closes the earlier mislabeled capture evidence.

---

# Last small review: native Assets root label

Your final d2f3a02 is integrated as81d1fa0. Root294UI tests/build and native
custom-protocol rebuild/package pass. CUA captured your root-folder typography
fix in artifacts/shadow-native-polish/assets-{wide,minimum}.jpg. Dimensions are
in dimensions.json (wide1440x900, second capture1720x669; true minimum recapture pending). Same actual
root-level project assets as your previous captures. Zero engine errors and
saved account restored without prompts. Review those two images, append verdict
to result.md, and commit report only if accepted. No unchanged test reruns.

---

# Priority final review — integrated native evidence

Your initial commit 38aef10 is integrated as root 9f02f45. Review this follow-up
first, then retain initial context below. The director's bottom-dock criticism
remains binding: cramped asset rows beside compiler problems looked messy and
unprofessional. The replacement must read as a dedicated asset workspace with
consistent spacing, asset details in the main Inspector, diagnostics below.
Review assets-separated-{wide,minimum}.jpg specifically; improve any real issue
you see, without reintroducing Assets as a bottom output tab.

New evidence copied to this worktree:
- artifacts/shadow-native-final/: 12 actual CUA captures and sanitized AX. Read
  README.md and dimensions.json. Minimum is Retina2002x1302 outer pixels with
  configured1000x650 inner logical points; earlier initial minimum was misnamed.
  Enabled, omitted, null and read-only focus; Assets separation; native actual
  look-dev geometry at12/38 degrees, Undo/Redo. Native default preview camera,
  not authored headless framing. Decide scoped native acceptance explicitly.
- artifacts/shadow-normal-before/ and shadow-normal-after/: your mapped-normal
  concern reproduced (81 central pixels, max12 levels) and fixed by Astra in
  model_material.wgsl/lighting/shade.wgsl. tests/shadows/normal_offset.rs isolates
  identical center-column BRDF with opposing tangent-X maps. Review correction.
- artifacts/shadow-integrated/: final shadow captures; all four look-dev images
  pixel-identical to your initial renders (shadow-lookdev-integrated-comparison.json).
- artifacts/shadow-final-timing.json: final three-trial medians at1080p,
  0sun/1caster1.389ms,1/1=1.470,1/64=1.479,4/64=3.096. This is a fenced
  analytical frame probe, not a game performance gate. Raw samples retained.
- Original comparison is122/124 exact after adding the omitted HDR unit capture;
  your initial121/123 numbers were correct for the files originally supplied.
  Updated shadow-pixel-comparison.json includes HDR; two known one-level pixels
  unchanged. Stable corrected integration captures123/123 exact.

Integrated verification already passes144 ordinary Rust,40 GPU (6unit,31render
integration,2headless,1editor),293UI,5Python,Clippy,schema/SDK/bridge/conventions
and final custom-protocol native build/package. The fixture export helper only
copies this synthetic test project to an explicitly requested new directory.
No runtime change. If final work is report-only, do not rerun unchanged suites.
If changing code, run relevant checks using ./tools/cargo with --release and
--locked; do not share target directories. Update result.md with final scoped
verdict and remaining thin-caster limits. Commit with Built-by: claude. No merge.

---

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
