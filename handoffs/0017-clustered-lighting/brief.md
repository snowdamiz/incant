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
