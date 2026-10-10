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
