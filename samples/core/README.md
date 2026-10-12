# Core Sample

Work in progress toward the Phase 1 sample in PLAN.md. The native TypeScript
gameplay controller is implemented; the finished level, presentation, audio,
shipping validation and reference-device results are still being assembled.

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
source at startup; the separate shipping proof must use cooked bytecode.

The room is a test fixture, not the finished level or a performance benchmark.
No device, full-save or Phase 1 gate is claimed by these checks.
