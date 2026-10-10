## Priority revision 2: report accuracy only

The rendered review and its measured results are accepted for this scoped link
increment; the visible height limitation remains explicitly open. Do not rerun
captures or alter the engine, scene, behavior or evidence.

Correct two descriptions in result.md, then commit with Built-by: claude:
- The invocation actually ran under `caffeinate -s -i` (both the outer ACP runner
  and run_set.sh commands). Remove the statement that no temporary sleep
  assertions were needed. State that there were zero 50 ms failures in the two
  recorded runs with temporary sleep prevention; this does not show robustness
  under sleep or CPU contention.
- Say two independently authored test projects/runs on the same immutable binary,
  not two independent builds, where the latter implies engine recompilation.

Retain the hover defect, exact measurements, session-local generation difference,
and all limits. This is an evidence correction, no new visual judgment required.

# Off-mesh link rendered-motion review

Claude Opus 5.5 over ACP owns all visual design, geometry look-dev, cameras,
rendered-pixel judgments and screenshots. Read CLAUDE.md and
`docs/spikes/navigation-links.md`. This packet tests the implemented query and
explicit behavior-owned traversal, not a phase gate or native debug-draw UI.

## Scope and acceptance

Create a restrained, clearly legible real-engine scene showing travel between
separate walkable platforms using actual `api.findPath` off-mesh links. Include
a directed link, a bidirectional connection or an equivalent separate query,
and an enabled/disabled link change through ordinary component commands. Make
the difference between walking and the explicit gap traversal clear in actual
motion. Claude chooses the layout, visual language and cameras. Keep it compact
and professional; do not bury the useful behavior in decorative geometry.

Route metadata must drive behavior: `traversals` carries stable `link_id`,
consecutive `from_index` / `to_index` and `reversed`. Never infer a link merely
from a large point gap. The engine does not execute a jump or promise a safe
trajectory; your behavior may implement a logical parabolic jump or another
explicit transition with ordinary commands. If the scene uses no Rapier bodies,
say so and make no physics claim. Use the actual snapped endpoints and retain
queries showing reverse/disconnected paths return null when required. Replan
after a link toggle and report the changed generation/tile reuse accurately.

Save during traversal and reopen to finish. Compare the resulting state/logs and
use a saved capture window to inspect the middle of the transition. Repeat the
same deterministic course independently and compare real frames/logs. Review
launch, landing, grounding appearance, directionality, pauses, visible jumps or
snaps and any mismatch between metadata and motion. Report defects with exact
unrounded values; rounding can make a submillimetre segment appear duplicated.
Do not hide problems with camera changes. Correctness changes belong to Astra.

## Interfaces and constraints

`tools/probes/navigation-links.py` is a passing public init/RPC, strict TS,
mid-jump save/reopen example. `handoffs/0026-navigation-lookdev/tools/` and
`handoffs/0027-steering-lookdev/tools/` demonstrate actual renderer captures and
trace comparison. You may reuse helper mechanics, but author an appropriate
visual scene. No diagrams, browser mocks or generated frames substitute for
real engine output. Native UI/capture is unnecessary while the Mac is locked;
never attempt to bypass the lock.

Use immutable `artifacts/tools/incant_headless`; verify its SHA-256 against
`artifacts/tools/binary.json` before and after. No Rust build is needed. Run npm ci
for the pinned TS/SWC dependencies. Compile strict TypeScript using the existing
build helper. Every scene/entity/asset mutation must use public import/RPC
incant_cmd transactions. Original mathematical glTF source files may be created
and imported; no direct project JSON mutation. No runtime shell or credentials.

Link IDs must be canonical ULIDs distinct within each mesh, at most 128. Enabled
endpoints must snap to that mesh within the configured distance; disabled links
can remain unattached. Endpoint snapping has a two-million-triangle work limit.
Extra costs are nonnegative distance penalties. Link-only changes reuse geometry
tiles. A* and funnel operate separately on each walking leg; the wider visibility
repair is currently skipped for link-containing routes. Walking heights remain
quantized approximations; source-conforming heights and native editing/debug draw
are separate work. Existing 50 ms script wall-clock limits remain; use temporary
sleep assertions for long runs, record failures, and keep the scene small.

Use 960x540 real `play --camera` renders. Each run is limited to 128 frames and
256 MiB raw captures. Helpers must refuse existing output directories. Retain raw
runs in ignored `artifacts/0028-off-mesh/`. Commit a compact, useful screenshot
set, reproducible helpers and `result.md` with exact model/transport, binary/source,
commands, measurements, verdict and honest limitations. Do not edit Rust, native
or web UI, other handoffs, mutation routing, credentials or external accounts.
Do not publish, merge or sign. Commit with `Built-by: claude`.
