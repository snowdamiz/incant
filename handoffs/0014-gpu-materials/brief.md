# Priority: verify the minimum-height Inspector scroll behavior

Astra investigated the last layout concern from your final review. The Inspector
is an intentional independent scroll area. A normal wheel scroll at 1000x650
reveals the complete Reimport help and Identifiers row, with space below it. See
artifacts/native-material-review/12-minimum-inspector-scrolled.jpg. The viewport,
Agent panel and side navigation stay fixed while the Inspector scrolls. Nothing
is permanently clipped or inaccessible.

Review that supplied capture and the .panel__scroll code if needed. Amend the
final review to distinguish content below a working scroll viewport from an
actual clipping defect. If you still judge the scroll affordance to require
visual polish, state a specific follow-up; do not misstate functionality as broken.
This is review-only: no builds, code edits or new captures. Commit the correction
with Built-by: claude. The previous backdrop/framing priorities were Astra's
routine integration decision under the director's continue-without-asking
instruction, not a new explicit human design decision.

---

# Priority follow-up: final integrated captures

Astra integrated your studio tuning. The final integrated tree passes all seven
explicit GPU tests, including new lit-back-face equality and varying-normal-map
checks, plus workspace Clippy. All 119 Rust behavior tests, 282 UI tests and the
UI build pass. Native texture reimport, Undo/Redo, failure retention and repair
were verified through the command bus with exact viewport-region comparisons.
The rebuilt app restores the project and saved account.

Review-only follow-up: artifacts/material-review now contains 19 freshly generated
PNG captures with your tuning, including lit-double-sided-back.png,
varying-normal-map.png, textured-cube.png and default-metal.png. Review all 19.
The cube is a mathematical fixture with simple stripes, not game art.
artifacts/native-material-review/10-final-wide.jpg and 11-final-minimum.jpg are
actual final native CUA captures at 1440x900 and 1000x650. Review those two for
viewport material appearance, framing, and preserved asset-sidebar/Inspector
separation. Do not repeat the visible account label in the report. The purple
screen-recording badge over macOS traffic lights is an OS capture overlay; it is
not application chrome and cannot validate traffic-light spacing.

Keep the current neutral backdrop and fixed camera for this scoped increment.
Specular IBL and tone mapping are the next renderer work; automatic framing can
follow. Do not ask the director to decide these routine implementation priorities.
No new builds, code changes or screenshots are needed. Append a final review to
result.md, distinguish accepted preview limitations from a regression, and commit
with Built-by: claude. Do not treat historical captures/hashes as current output.

---

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
