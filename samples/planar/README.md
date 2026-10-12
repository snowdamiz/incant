# Planar Sample

A top-down XY gameplay foundation for the Phase 1 2D sample. The controller uses
native 2D sensor events, bulk velocity columns and reversible structural commands.
It collects each pickup once, keeps the exit locked until all pickups are held,
loses on a hazard and stops after either terminal outcome. Diagonal movement is
normalized. Tilemap collision remains owned by the native solver.

Author one controller with `player`, `exit`, `pickups`, optional `hazards`,
optional `camera` and `speed` props. IDs are canonical stable ULIDs with distinct
roles; there must be 1–16 pickups and at most 16 hazards. Speed defaults to 3 m/s
and must be in `(0,12]`. The player needs `RigidBody2D` with dynamic motion and
zero gravity, `Collider2D` and `Velocity`. Exit, pickups and hazards are fixed
2D sensor colliders. The camera, when supplied, follows XY while preserving its
authored offset. Physics entities and camera must be unparented. Despawned
pickups must have no children. Bind `move` as a two-axis action with positive Y
pointing up. The builder owns sprite art, tile layout and camera composition.

Check the controller with
`node node_modules/typescript/bin/tsc -p samples/planar/tsconfig.json`.
Build `incant_headless` with the repository's normal CI profile, then run
`python3 samples/planar/verify.py <new-output> --binary <incant_headless>`.
The helper imports a one-pixel numerical atlas, authors the project exclusively
through public transactions, verifies Undo/Redo and journal reopening, then plays
seven native scenarios: keyboard win, gamepad win, hazard loss, wall collision,
locked exit, normalized diagonal movement and exact replay. All seven pass locally.

`python3 samples/planar/export_verify.py <development-output> <new-export-output>`
builds an ordinary shipping player, removes its copied project/cache, private
build storage and external script maps, and compares seven fresh processes with
the development observations, logs and PCM. Source review is accepted; the first
shipping run is in progress. The optional follow camera and simultaneous
hazard/exit priority are source-reviewed but are not covered by these seven runs.

The numerical room and white atlas are test fixtures. Finished sample art,
rendered acceptance, device performance and physics save/load remain open.
Gameplay persistence declarations do not make solver checkpointing supported.
