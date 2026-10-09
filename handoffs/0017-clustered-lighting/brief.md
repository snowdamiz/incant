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
