# Priority: integrated final lighting review

All corrections are integrated. Runtime source is 2ee02d0 / 00dfbd1 (the latter
adds only an account-dialog test race fix). Full local validation passes 135 Rust
behavior tests, 24 real GPU checks, 283 UI tests, workspace Clippy, generated
files, SDK typecheck and the native release build. Your range-edge fix passes the
new observable regression that failed on the original shader.

Review freshly regenerated `artifacts/cluster-final/` and CUA captures in
`artifacts/lighting-native-final/`. Initial images remain historical only.
The final CLI captures reverify intensity edits, exact Undo/Redo image hashes,
atomic invalid-range rejection, journal reopen and source-free loading after the
shader change. Verify the final oracle pairs directly.

Native final: 01 point dim at 60 cd; 02 Undo to 300 cd; 03 Redo to 60 cd;
04 spot at wide size; 05 spot at 1000x650 before scrolling; 06 after scrolling
inside Inspector; 07 directional at minimum size. The native schema now emits
RGB widget hints and cd/lx/m/degree units. Spot/directional probes now include
explicit identity Transforms so their aim is inspectable. Their intensity is
zero intentionally; the point lights the sphere. Account restoration still
requires no prompt; do not put its email in the report.

Handoff 0016 is complete and its generic two-line label fix is integrated here.
Its final native captures are approved. You may now make a focused Inspector
scroll-affordance fix if the final 05/06 pair still demonstrates a material UX
problem. Field access does work through scrolling; do not call the field
unreachable. Own any necessary presentation correction, and keep command
behavior unchanged. If no correction is required, give the precise final scoped
verdict. Do not expand into broad editor redesign or production lighting approval.

Update result.md, closing resolved defects and listing any remaining material
issue or limit. Commit current brief and review result with Built-by: claude.
Images stay git-ignored; no account-bearing native image is to be published.

---

# Priority: resolve the visible range rim and review native captures

Your expanded review found a real range-edge crease. Please own the focused
look-development correction to the range window in
`crates/incant_render/src/lighting/shade.wgsl`. Preserve the authored units,
inverse-square behavior away from the cutoff, strict zero outside the range,
and finite behavior at the source. A squared quartic window is the established
C1-continuous candidate: `(max(1-(d/r)^4,0))^2 / max(d^2,1e-4)`.
Primary reference: https://google.github.io/filament/Filament.html, attenuation
function / equation 65. Implement the independently expressed arithmetic, cite
the equation rather than copying a shader listing. Choose and review the visual
correction; do not change cluster assignment, light units, tone mapping or other
BRDF math. This narrow shader change overrides the earlier no-shader-edit rule.
Astra will run all numerical tests and regenerate final evidence after return.

The near-patch plateau is intended: the denominator uses a 1 cm minimum distance
to keep coincident punctual sources finite, matching the reference above. The
first two patches put sources 5-7 mm from the patch. Document this confirmation;
it is not a cluster omission. New `punctual-low-dielectric-*` captures at intensity
50 provide the lower-intensity companion you requested.

The Mac is now unlocked. `artifacts/lighting-native/` has real CUA captures:
01 point intensity 60; 02 Undo restores 300; 03 Redo restores 60; 04 spot Inspector
at 1440x900; 05 spot Inspector at 1000x650. The disabled spot/directional probes
have zero intensity solely to inspect schema-generated fields; the sphere is
illuminated by the authored point. Saved account restored without interaction.
Do not transcribe its email. Review these for viewport and field presentation.

Handoff 0016 is concurrently fixing generic Inspector label wrapping; do not
edit that CSS in this handoff. If Color using X/Y/Z instead of R/G/B is materially
confusing, you may make the focused generic FieldView label presentation fix
without changing command behavior. Identify any other concrete issue.

To render in this isolated worktree, use its own target directory and the root
cargo helper; never share CARGO_TARGET_DIR. All dependencies are local/cached.
`INCANT_LIGHT_EVIDENCE=<absolute output directory> ./tools/cargo test -p
incant_render --release --locked --test model_gpu lights::coverage -- --ignored`
regenerates the requested range/material/oracle images. Native final captures
will be refreshed after integrating the shader and label changes. Preserve the
old verdict/history and append the final changes and remaining verification.

---

# Priority follow-up: expanded real GPU coverage

Review the additional captures now in `artifacts/cluster-initial/` and update
result.md. Preserve the initial verdicts and clearly distinguish added evidence.
No shader or runtime appearance change was needed after your initial review.

- `spatial-overflow-96-*`: 96 spatially distinct colored point lights overlap,
  crossing the 64-light list bound. Paired `-oracle` images are supplied now.
- `depth-boundary-patches-*`: 24 camera-facing numerical patches in four columns
  and six rows, ordered left-to-right/top-to-bottom. Their view depths alternate
  just before/after each logarithmic boundary from 0.1 to about 681 world units.
  Perspective-scaled patches make distant slices visibly inspectable. Paired
  `-oracle` images supplied. Unlike a receding floor, far slices are not subpixel.
  The separate real GPU readback test verifies one tiny light in every one of
  the 24 slices and verifies stale counts clear; a slice-debug shader was not
  added to the shipping path. The patch grid is a numerical diagnostic fixture.
- `visible-range-edge-*`: finite point range crosses a surface, with a visible
  smooth cutoff and tiles on the path. Odd and even dimensions included.
- `punctual-hdr-*`: analytic sphere, authored high-intensity point light, metal
  and dielectric at roughness 0.045/0.3/0.7/1. Includes grazing angles and HDR.
- `spot-centered-reference` / `spot-penumbra`: odd dimensions put the exact
  sampled center at the world origin. The numerical 15-degree falloff check now
  passes (even-dimension center samples were slightly off-axis).
- `cli-*point`: real public CLI sphere fixture with authored point, intensity
  edit, exact Undo/Redo restoration, durable journal reopen, atomic rejection of
  invalid range, and source-free rendering. No account or provider was needed.

Visually review all added images and verify supplied oracle identity directly.
Report any defect or missing evidence materially relevant to this increment.
The Mac remains locked, so native work remains explicitly pending.

Transport is verified by Astra's runner: `tools/handoff/main.py` starts
`node_modules/.bin/claude-agent-acp`, exchanges JSON-RPC initialize with ACP
protocol 1, creates/resumes the worktree session, selects `Opus 5.5`, and sends
`session/prompt`. Record Claude Code through ACP 1; no director clarification
or separate transport identifier is needed. Keep throwaway review scripts in
this worktree's ignored artifacts directory. Do not edit outside this worktree.

---

# Authored punctual and clustered lighting: pixel review

Claude Opus 5.5 via ACP owns rendered-pixel review and any look-development
recommendations. Astra implemented correctness: DirectionalLight, PointLight and
SpotLight typed components, physical light units, inherited world transforms,
GPU clustered assignment (64-pixel tiles, 24 logarithmic depth slices), and a
bounded list with an exact all-local-lights overflow path. No new art direction
from the director: this is the next Phase 1 engine increment in PLAN.md.

Review actual PNGs in `artifacts/cluster-initial/` (copied into this worktree).
These are numerical quad fixtures, not a designed game or art. Point-near/far
checks inverse square distance; range and away-facing spot images deliberately
contain black geometry; directional-red is intentionally pure red; 64/65/128
fixtures share total intensity and must look the same; spatial-cluster captures
contain a regular distribution of small colored lights and match a brute-force
GPU oracle byte for byte. The default studio and tone map are unchanged.

Judge discontinuities, tile/depth boundaries, unintended seams, cone edges,
unexplained tint or clipped highlights. Do not mistake the test quad boundary
or intentionally absent lighting for a scene-design defect. You may suggest
additional actual rendered evidence for visual coverage. Do not implement
shader math, change tests, tune authored test lights, edit project content, or
alter the UI. Do not approve production lighting: no shadows, antialiasing,
exposure controls, mobile tiers or complete render graph exist yet.

The Mac is currently locked, so no native captures are available. Never claim
native approval. Do not use shell-native capture or read credentials. Write
result.md with exact model, transport, image names and verdicts, open defects or
additional evidence needed. Commit with Built-by: claude in the final trailer
paragraph. The handoff runner's bypassPermissions mode is director-authorized.
