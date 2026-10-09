# Imported glTF materials: preview look-dev and pixel review

Claude Opus 5.5 through ACP owns this visual review. Astra implemented the checked
material semantics and GPU resource lifetime. Review every PNG in ignored
artifacts/material-review (copied into this worktree before your run). These are
real Apple M5 Pro readbacks, not mockups. The small quads are numerical test
fixtures; default-metal.png is a 640x360 imported model with glTF default material.

Judge legibility and neutral studio appearance, not just numerical equality.
Review base-color/emissive color, normal-map shading, MR appearance, alpha cutout,
layered transparency, sidedness, minification, and the default imported model.
The mask-discarded image intentionally contains only background. Pure red/green
emissive cases are test signals, not art direction. Do not call them final game art.
Inspect code as needed. Main tests are in crates/incant_render/tests/model_gpu.rs.

You may tune crates/incant_render/src/studio.rs LIGHT_RADIANCE and
DIFFUSE_ENVIRONMENT for a clearer neutral preview. Preserve camera EYE and light
direction for now. Do not fake metallic semantics to conceal missing specular IBL.
If production IBL/tonemapping is necessary, report it explicitly for Astra's next
implementation; do not claim it exists. No changes to geometry/alpha/color-space
math, fixtures, tests, UI layout, dependencies or project state. If you change
constants, Astra will regenerate captures for your final review. No screenshots of
native UI in this first pass; native captures will follow after correctness checks.

This renderer is a fixed preview: no authored lights, clustered render graph,
shadowing, specular environment, SSAO, bloom, tonemapping or antialiasing yet. Keep
those limitations explicit. Stable CPU/GPU lifetime, source watching and Undo
must remain intact. No account access or external publishing. Use only this
worktree. Do not share CARGO_TARGET_DIR or use another worktree's target directory.
No build is needed for initial image review; if a build is necessary use ./tools/cargo.

Return handoffs/0014-gpu-materials/result.md with exact model/transport, all
inspected captures, verdict, concrete findings, changes and limitations. Commit
with Built-by: claude. Permission mode bypassPermissions is director-authorized.
