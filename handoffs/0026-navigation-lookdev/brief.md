## Priority revision 2: connected visibility repair (2026-10-10)

Astra has added bounded line-of-sight corridor repair at source 4aee4ce. Every
shortcut traverses real connected polygon portals and restores height detail;
it uses only the visit budget remaining after A*. An independent Dijkstra
oracle still verifies initial A* cost. Tests show detour reduction and exact
fallback when smoothing has no visit allowance. 64 rotated-wall combinations
(four radii, four cell sizes, four angles) pass the configured-radius check.

Please run a final v6 and exact repeat with the new verified binary. Inspect
whether the 1.14 m tile-boundary detour disappears; quantify any remaining one.
Review actual clearance, motion, grounding and arrival. Keep v5 as history.
Do not re-author cameras to hide defects. The same scene/marker coverage is useful.

The erosion safety margin remains, since reverting it reproduced radius clips
even with zero contour simplification. This makes configured radius a minimum
clearance, at the cost of conservative passage loss at coarse voxel resolution.
A new Rust fixture explicitly demonstrates the 1.1 m doorway fails at 0.1 m
cells and passes at 0.05 m cells with the same 0.4 m radius. Add the same finer-
resolution comparison to the doorway sweep if useful; keep actual results and
state the resolution tradeoff clearly. This is not a claim that every geometric
fit is discovered. Exact-clearance navmesh construction remains a potential
quality improvement, not a reason to weaken radius protection. No separate
director approval is required for these routine implementation decisions.

Quantized heights and remaining step smearing stay explicit. This review must
not call those exact geometry or mark the phase complete. Report any new actual
navigation failure to Astra. Source metadata and SHA are in binary.json.

## Priority revision: review clearance and detail refinement (2026-10-10)

Continue the same visual review after Astra's correctness changes at 141a835.
The new verified binary and source metadata are in artifacts/tools/binary.json.
Use new output directories, e.g. v4 and v4-repeat; preserve v3 as historical data.
The source now compensates erosion for chamfer-distance error and uses 0.25-cell
contour simplification, one-cell detail sampling and 0.25-cell height error.
Sixteen rotated-wall/voxel-size correctness cases pass configured-radius checks.
Re-run your scene, every-tick diagnostics and exact repeat, then inspect real
frames again, especially wall clearance, ridge profile, doorway traversal and
arrival. Check that the marker pools cover EVERY returned route point/segment;
raise those pools if necessary within the existing frame/memory budgets.

Astra's scope decisions: query heights deliberately describe the quantized
navigation surface, not exact source triangles. Do not subtract a universal cell
from results: on arbitrary unaligned/sloped geometry that can put points below
walkable spans. Physics owns grounding/placement. The denser detail is intended
to reduce height smearing; measure remaining error honestly. Centroid-graph A*
plus funnel is not a global Euclidean shortest-path solver, so an occasional
corridor/tile kink remains an explicit quality limitation. Last-build reports
currently describe the last update, including no-ops; retain your truncated-run
method for the edit's rebuild evidence. No director approval is needed for these
routine scoped engineering decisions. Do not mark the wider phase complete.

Please revise result.md around the FINAL tested behavior, retain a short before/
after findings table, and replace/add the compact screenshot set as appropriate.
Transport is ACP: Astra invoked tools/handoff/main.py's JSON-RPC ACP adapter;
you may state that as host-supplied transport evidence, distinct from what you
can inspect from inside the session. Report any new correctness failures.

# Navigation rendered-motion review

Claude Opus 5.5 over ACP owns visual choices and all pixel judgments. Read
CLAUDE.md and docs/spikes/navigation-runtime.md. This is the actual tiled
navigation/runtime increment, not phase approval or a claim that steering,
off-mesh links, 2D navigation or the graphical navigation tools are finished.
The director permits computer use and capture. The Mac is locked; native capture
is unavailable until the director unlocks it. This handoff can proceed using real
headless engine renders and does not require native interaction.

## Scope and acceptance

Create a clear, polished, restrained real-engine look-dev scene demonstrating a
character following an actual `api.findPath` route around static obstacles and
replanning after a scripted room edit. Include a low ridge or climbable step if
useful for checking height detail; show a disconnected/too-narrow route returning
null in logs/state rather than faking travel through it. Judge actual motion,
clearance, groundedness and legibility. Report stalls, corner cutting, vertical
artifacts and any discrepancy between the returned path and rendered movement.
Do not hide defects with camera angles; correctness repairs belong to Astra.

Use imported original mathematical geometry and matching analytic colliders.
The path generator selects explicit same-scene sources in a NavigationMesh
component. Dynamic/kinematic bodies and nonzero-velocity sources/ancestors are
rejected. Curve colliders are conservatively faceted for navigation. Cooked
models use the default glTF scene with hierarchical/mirrored transforms. Every
scene/entity/asset mutation goes through public CLI import/RPC/incant_cmd. You may
create geometry source files then import them. No project JSON mutation shortcut.

`tools/probes/game-navigation.py` is a passing strict-TypeScript end-to-end
correctness example. It has no camera/visual design; create your own readable
scene. `handoffs/0023-character-lookdev/tools/` shows the prior real renderer
workflow. Stay with the current neutral Incant color direction. Use accurate
geometry and a camera that makes the route and room change obvious. Distinguish
real rendered path marker geometry from a future native debug-draw feature.

Use the verified executable at `artifacts/tools/incant_headless` and check its
SHA-256 against `artifacts/tools/binary.json` before and after. This is copied
from Astra's successful native build. Do not compile unchanged Rust. Run npm ci
for local TypeScript/SWC dependencies if necessary. Use strict TS and
`node tools/build_script.mjs SOURCE OUTPUT`. No shell is exposed to the engine
agent or gameplay scripts; build-time authoring tools may use the terminal.

Use real `play --camera` captures at 960x540, at most 128 frames and 256 MiB raw
capture data per run. Retain every-tick state/log diagnostics and run an
independent repeat. Inspect actual frames at useful motion checkpoints and
compare the repeat. Do not substitute diagrams, browser mockups, generated
frames or script-only tests for rendered review. Source/project/run helpers must
refuse to overwrite existing output directories. Keep raw outputs in ignored
artifacts; commit only reproducible tools, a compact screenshot selection and
result.md with exact model, transport, commands, verdict and limitations.

Do not alter Rust, core algorithms, other handoff packets, UI layouts or project
mutation routing. The Navigation Inspector and native debug-draw design are a
separate next increment. No native screenshot is required for this query-only
rendered review. No publication, merge, credentials or external accounts.
Commit your reviewed work with `Built-by: claude`.
