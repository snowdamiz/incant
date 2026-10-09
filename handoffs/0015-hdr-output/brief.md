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
