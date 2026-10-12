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
the development observations, logs and PCM. All seven shipping processes pass locally with exact development parity. Source
and independent final execution reviews are accepted. PCM is silent in this
fixture, so matching it verifies the sample clock rather than audible behavior. The optional follow camera and simultaneous
hazard/exit priority are source-reviewed but are not covered by these seven runs.

The numerical room and white atlas are test fixtures. `scene.py` authors the
Pearl Cove presentation of this controller with original tiles, sprites and a
localized HUD bound through the optional presentation props; see
[PRESENTATION.md](PRESENTATION.md). Native-window acceptance, restart, device
performance and physics save/load remain open.
Gameplay persistence declarations do not make solver checkpointing supported.

[Retained local evidence](../../docs/spikes/evidence/planar-sample-gameplay.json).

Optional presentation bindings use only existing native commands; omitting them
preserves the original controller contract:

```ts
flipbooks: { player: playerSpriteId, boat: exitSpriteId },
hud: { table: stringTableId, status: statusLabelId, prompt: promptLabelId },
results: { won: wonNodeId, lost: lostNodeId, active: resultContainerId, inactive: inactiveContainerId }
```

Each group is optional. Either flipbook role may be omitted. Explicitly bound
flipbooks become paused state indicators: player frame 0 while playing, 2 on win,
1 on loss; boat frame 1 once every pickup is collected, otherwise 0. Author at
least three player frames and two boat frames. These roles may identify the
existing player/exit sprites or separate authored visual entities. Other
flipbooks keep their own clocks. Initial binding and state transitions queue
seeks; unchanged ticks do not queue presentation commands.

The HUD string table must provide `hud.collecting` (`collected`, `total`),
`hud.ready`, `hud.won` (`total`) and `hud.lost`. The status uses the current native
locale, fallback and pseudo-localization settings; a locale change refreshes it.
Initial committed text appears after the first callback, before the controller's
first nonzero movement integration. Text and frame reads within a callback remain
pre-command snapshots. The optional result nodes move to `active` only for their
matching terminal phase, and to `inactive` otherwise. Container appearance and
visibility are authored by the visual owner; this controller does not style them.
HUD and result role IDs must be distinct. Unknown binding keys and invalid IDs
refuse the tick; wrong native component/parent/frame roles use normal atomic
native validation.

The prompt label is cleared on initial binding and remains empty. **Restart is
not implemented.** A real whole-session reset must reconstruct the cooked world,
script state, despawned pickups and solver/host state. Resetting a few entities
would not implement that contract. No key is advertised as restarting the game.

`verify.py --presentation` authors a separate numerical fixture using the same
white atlas plus a packaged font, public string-table/UI/flipbook commands, and a
read-only observation wrapper. It checks accepted frame changes, localized HUD
reads on the following tick, result parents, unchanged authoring and the original
seven scenarios. `export_verify.py` accepts that output through the ordinary CPU
packaged route, with the same explicit UI viewport. These helpers do not merge or
modify Claude's Pearl Cove art, builder, widget layout or appearance acceptance.
