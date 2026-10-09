# Priority: final corrected headless and native visual review

The corrected curve is integrated. Workspace Clippy passes, all 124 Rust behavior
tests passed on the integrated CPU implementation, and all eleven final explicit
GPU tests pass. The final native custom-protocol release build and UI build pass;
282 UI tests pass. No further source or lighting changes are needed unless your
review finds a specific defect.

Review all 24 freshly generated PNGs in artifacts/hdr-final, including the
fill-only albedos .1/.18/.5 and textured-cube. Use the earlier material-before
captures for comparison. Numerical GPU assertions match your corrected expected
values; the glossy highlight bound now distinguishes HDR from clipping at 1.0.
The default-metal capture from 0014 is historical; no new default-metal image is
supplied here. Keep missing specular IBL explicit.

Review artifacts/native-hdr-final/07-final-wide.jpg and 08-final-minimum.jpg.
These are actual CUA JPEGs from the rebuilt app: respectively 2880x1748 physical
pixels (1440x874 logical; constrained by available desktop height), and 2000x1300
physical pixels (1000x650 logical). The app remains Attached, zero problems,
with project, four history items and saved account restored. Do not repeat the
visible account label. The purple recording badge is macOS capture chrome, not
app UI. Independent native resize succeeds; the Inspector is a working scroll
area at minimum height, as verified in 0014.

Give a final verdict for the corrected curve and the native integration; do not
confuse initial HDR captures or comparison sheets with the corrected output.
Record exact inspected files and remaining limitations. The design correction
was Astra's routine integration decision under the director's standing continue
instruction, not newly typed director feedback. Correct that attribution in the
result. Exposure and initial curve selection were also routine agent choices.
Do not request additional director decisions on this scoped preview.

This is review-only: no builds, new capture, UI/code edits or publishing. Append
final review to result.md and commit with Built-by: claude in the final paragraph.

---

# Priority correction: revise the curve to preserve material colors

Your review correctly caught the color regression. Do not defer it or request a
director decision: the director authorized sensible implementation/design work
without repeated questions. My earlier instruction to keep the operator was a
routine scoping choice, which I am changing based on your evidence.

You now own revising tone_map.wgsl itself. Keep ordinary scene colors, including
emissive colors and dark fill-lit materials, stable while retaining a neutral,
hue-preserving highlight shoulder. A promising option is the reference shoulder
with the assumed Fresnel toe subtraction removed; choose the exact curve and
threshold based on your appearance responsibility. If you modify Khronos's
operator, label it accurately as a derived preview curve, not Khronos PBR Neutral
conformance. Update modified-file attribution and THIRD_PARTY_NOTICES.md as
necessary. Do not add an artificial constant to scene radiance to cancel the toe;
do not fake an IBL contribution. Preserve exposure 1.0 unless justified by actual
captures. Future real IBL will be reviewed on its own merits.

No changes to render resources, alpha, color-space/composition math, geometry,
tests or project state. Implement the revised curve and give independent numeric
expected outputs for the same swatches, [1,0,0], [.5,.5,0], and the emissive brown.
Astra will update reference assertions, generate fresh captures, and add a real
fill-only dark-material fixture with gray albedos .1/.18/.5 before final review.
Current captures remain historical and must not be approved for the new curve.
Native verification has confirmed project/account restoration and attached
viewport across native window resize; fresh final screenshots follow the curve.

Commit with Built-by: claude. No decision or permission request to the director
is necessary. End after implementing the correction so Astra can regenerate.

---

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
