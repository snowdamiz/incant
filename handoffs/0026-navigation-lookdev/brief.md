## Priority revision 5: rounded portal boundaries (2026-10-10)

Source 020c640 fixes both v8 query directions without a fallback that hides invalid
geometry. Projecting onto the sloped triangle puts the start on a shared edge;
f32 storage rounds it just outside. Portal intersection, its segment parameter,
funnel orientation, visibility intervals and height barycentrics now use f64.
Boundary tolerance is measured in world-space coordinate precision (two f32 ULPs),
not a fixed fraction of a centimetre segment. A nearly collinear segment tests
its actual endpoint distance to the portal before dividing a tiny determinant.
The exact three polygons now pass 7,056 nearby queries at four heights and four
directions. The room fixture passes 34,932 additional short queries. Existing
radius, narrow-doorway, disconnected/stacked-floor and Dijkstra tests still pass.
The public saved-game navigation probe passes on the supplied binary.

Run a v9 final rendered review and exact repeat using the new immutable binary
and metadata. Preserve the current scene/cameras; re-run query probes, walkability,
clearance, grounding and arrival. Keep previous results as history and accurately
report any remaining exceptions rather than masking them. Check actual frames.

Height scope clarification: inspection of the reported [1.084,.081,-2.354]
point traces its undershoot to the detail triangle interpolating between a low
sample at x=.97 and high samples at x=1.06 and x=1.518. No further height-detail
algorithm change is included. This quantized, piecewise-linear surface smooths
true step discontinuities and can fall below source geometry near a step. The
older packet's suggestion that it never goes below walkable spans was too strong;
the API documents approximate heights and requires physics for grounding. Retain
the measured 6.9 cm undershoot / 28 cm segment from v8 as historical evidence,
measure v9 honestly, and verify that actual character grounding remains correct.
Do not label this exact source geometry or a passed full-engine gate. Source-
conforming height detail remains an explicit navigation quality follow-up.

The generic script deadline failures remain a separate engine robustness issue;
no deadline change is in this binary. Use temporary sleep assertions for long
runs, record failed attempts, and do not bypass the Mac lock. No native capture
is required. Claude Opus 5.5 over ACP remains the visual reviewer.

## Priority revision 4: phantom raster spans and endpoint precision (2026-10-10)

Both v7 failures now have reproduced, fixed regression cases. The missing floor
came from rerecast 0.4.0's triangle clipping: when input vertices reached zero,
old output counts were reused, inventing solid spans more than a metre outside
capsule facets. A pinned local source patch clears those counts. The open 4x4
floor-query grid around the pillar now passes. See third_party/rerecast/INCANT_PATCH.md.
The centimetre query chose the wrong skinny triangle because the f32 squared
snap distances lost their ordering. Closest-point arithmetic and distance ranking
now use f64. Both directions of the exact 4.7/4.71 query pass a targeted test.

Review the new verified binary as v8 with an exact repeat, including the hole
map, query-error probe, live motion/clearance and doorway sweep. Preserve earlier
results as history. Quantify remaining bends and height artifacts; the raster fix
may alter the route and arrival timing, so use actual final checkpoints. If a
camera needs an adjustment to inspect the changed route clearly, you may make it,
but retain the existing overview/plan views for comparison and disclose changes.
Use temporary sleep assertions for long command runs if the locked Mac suspends;
never bypass the lock. No native interaction is needed for this packet.

## Priority revision 3: portal-entry search (2026-10-10)

The v6 review exposed a detour the shortcut pass could not remove. Astra has now
replaced centroid-cost A* with directed portal-entry A*: distance is charged
between portal midpoints from the actual start, including the final end segment.
Funnel and connected visibility repair remain. An independent Dijkstra oracle
checks the new graph cost, and a core regression recreates your room.

Use the new immutable binary and source metadata in artifacts/tools/binary.json
for final v7 captures and an exact repeat. The direct fixed-start diagnostic from
[-3.44, 0, 0] to [6.6, 0, 1.2], after moving the barrier, now measured 13.634 m
instead of 15.27 m and no (-3.3, -1.8) tile kink in Astra's preliminary run. Verify
this separately from the live course: the new initial corridor goes around the
other side of the pillar, so the character's tick-105 replan start changes and
whole-route lengths are not directly comparable. Review motion, wall clearance,
grounding, marker coverage and arrival on the actual new course, without hiding
issues with camera changes. Report residual kinks honestly. This is still portal-
graph optimality, not a global continuous shortest-path claim.

The resolution/clearance tradeoff and quantized height limitations from revision
2 remain. Preserve prior results as history and rewrite result.md around the
final tested binary. No routine engineering decision needs director approval.

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
