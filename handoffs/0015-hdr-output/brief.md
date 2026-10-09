# Priority follow-up: review integrated HDR output

Astra integrated the tone mapper with RGBA16Float geometry and transparent
blending, followed by tone mapping, display backdrop and the existing rounded
native mask in one output pass. Chrome/backdrop are converted to linear display
RGB for composition, then encoded once at the final output. They do not go
through tone_map. sRGB and non-sRGB RGBA/BGRA targets match within one code value,
including rounded-mask edges. Two new output GPU tests and six material/geometry
GPU tests pass; the complete workspace checks are running separately.

Review all 20 PNGs under artifacts/hdr-review (actual GPU readbacks), comparing
against artifacts/material-before (the previous 19 material captures). The
384x128 hdr-reference-swatches.png consists of 12 equal-width numerical columns:
neutral radiance 0, .01, .18, .5, .8, 1, 2, 4, 16, 65504, then [4,.2,.2] and
[65504,0,0]. The hdr-glossy-highlight quad has a halfway-vector normal and roughness
.3, so its lighting exceeds one. hdr-transparent-over-black is white emission
at alpha .5 over opaque black; the resulting .5 radiance maps to .46 linear.
These are numerical fixtures, not game art. Other captures exercise the material
features you reviewed previously. The existing textured-cube and default-metal
will be supplied with the native follow-up; do not report reviewing missing files.

Judge actual dark-material legibility, preserved neutral background, specular
shoulder and transparency. You may tune studio.rs LIGHT_RADIANCE and
DIFFUSE_ENVIRONMENT or the exposure constant in tone_map if the images require
it. Keep the selected operator and camera; do not modify correctness code,
tests or geometry. Record any tuning as pending fresh-capture approval. Native
1440x900 and 1000x650 review will follow after build. No need for your own build.

Integration corrected the upstream revision: the old file-change commit lacked
.reuse/dep5. Current code and Apache declaration are both pinned to
180b1a7bddec33f73fe41712a2963cc3ad8e5547, verified directly from upstream. The exact
license is now in licenses/Apache-2.0.txt and THIRD_PARTY_NOTICES.md; the development
app bundle and desktop CI artifacts include both. Update the result's obsolete
licensing open question and first-pass source reference. Exposure can remain a
function-local constant until authored renderer settings exist; no clarification
from the director is needed.

Append your review to result.md and commit with Built-by: claude in the final
trailer paragraph. Use the current packet first. Do not approve unseen native
output. Only Astra captures native UI via CUA; do not use shell screen capture.

---

# HDR output: tone-mapping design and later pixel review

Claude Opus 5.5 through ACP owns tone-mapping appearance and visual approval.
Astra is implementing the linear RGBA16Float scene target, pass sequencing,
format handling, GPU lifetime and numerical tests in parallel in the root
checkout. Your files below are isolated; no shared target directories.

First pass: design and implement `crates/incant_render/src/tone_map.wgsl` as a
WGSL source snippet defining `fn tone_map(color: vec3f) -> vec3f`. Input is finite,
nonnegative linear scene RGB (up to 65504); output must be finite linear display
RGB in [0,1], black maps to black. This snippet will be appended to Astra's
postprocessing shader. Do not implement vertex/fragment entry points, bindings,
transfer functions, alpha handling or render resources. Preserve hue/color
relationships in ordinary material values and give bright neutral/specular
highlights a controlled shoulder. Choose an appropriate established neutral
operator, document its source, any license obligations, and fixed exposure.
Primary source references only. Do not pretend this is an ACES pipeline if it is
only an approximation. No new dependencies. Astra will review mathematical and
licensing correctness. You may update the commentary in studio.rs; keep existing
camera, direction, light radiance and diffuse fill for this first pass. A later
capture review may tune radiance/fill as necessary, but not geometry semantics.

Use clear full comments explaining visual intent. The native editor chrome and
neutral #141519 viewport backdrop are display-referred and must retain their
exact current colors. Astra will exclude the backdrop from tone mapping using
scene alpha and compose it after the scene pass. Alpha blending happens in HDR
before tone mapping. No specular environment exists yet; that is the next
increment, not something this output transform can conceal. No authored lighting,
bloom, AA, shadows or clustered forward+ in this scope. Do not call this the full
production render graph.

Write a first-pass handoffs/0015-hdr-output/result.md describing the chosen
operator, rationale, numerical sample expectations, source/license and pending
pixel review. Commit with Built-by: claude. No screenshots or builds required in
this first pass. Actual headless and native captures will follow after Astra's
integration. You own appearance review then; do not approve unseen output.
Do not edit any other code, UI, fixture or test. No accounts, publishing or merge.
The director authorized bypassPermissions and CUA screen capture when needed.
