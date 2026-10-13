# Core Sample

Work in progress toward the Phase 1 sample in PLAN.md. The native TypeScript
gameplay controller has passed development and source-free shipping checks;
the finished level, presentation, audio and reference-device results are still
being assembled.

`gameplay.ts` updates the scene through native bulk component columns. Its one
controller instance references a player, a navigation mesh, an exit, enemies
and pickups by stable ID. Movement uses native character sweeps; enemies refresh
bounded navmesh paths on staggered ticks. Pickups are consumed once, damage has
a cooldown, and winning or losing stops all actors. An optional unparented
camera follows the player while preserving its authored offset and rotation.
Configured actors must be unparented. Off-mesh links need an explicit traversal
controller and are refused here.

Check the source with `node node_modules/typescript/bin/tsc -p samples/core/tsconfig.json`.
Build the headless tool using `./tools/cargo build --locked --profile ci -p incant_headless`.
Then run `python3 samples/core/verify.py <new-output-directory> --binary <incant_headless>`.

The helper authors a collider-only numerical room through public command
transactions, verifies Undo/Redo/reopen, and executes seven actual native runs:
keyboard win, gamepad win, jumping, losing, identical replay, missing pickup,
and collision against a wall. It checks exact movement stopping, unique pickups,
damage spacing and the game's state transitions. It also verifies that Play
does not modify the authored project or script. This development command parses
source at startup. These scenarios also run in the existing comprehensive desktop
checkpoint jobs, using their already-built headless tool.

Run `python3 samples/core/export_verify.py <successful-development-output> <new-export-output>`
to build an ordinary shipping player and replay the same scenarios with cooked
bytecode. The helper removes only its new copied project, private build storage
and external script maps; original development evidence remains unchanged. It
checks exact logs, state observations and PCM against development. The missing
pickup check verifies arrival within the exit's radius without granting a win.
[Recorded local verification](../../docs/spikes/core-sample-gameplay.md) distinguishes
these numerical checks from the ongoing visual and device work.

The room is a test fixture, not the finished level or a performance benchmark.
No device, full-save or Phase 1 gate is claimed by these checks.

The authored HUD may bind one optional label as `hud.prompt`, using the localized
`hud.prompt` string-table key. It joins the existing result title only after a
win or loss and returns to the inactive container on restart. Winning clears the
gameplay objective. The original HUD remains supported; missing or malformed
roles fail explicitly. No prompt node, wording or styling is supplied by the
controller. See [the binding contract and CPU proof](../../docs/spikes/core-result-hud.md).
