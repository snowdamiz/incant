# Core Sample native gameplay and standalone execution

The TypeScript controller in [samples/core](../../samples/core/README.md) implements
character movement and jumping, navmesh-chasing enemies, pickups, damage cooldown,
winning and losing. One source callback uses native component columns and bounded
physics/navigation queries. Project authoring uses command transactions; gameplay
never mutates or serializes the project document. The optional camera follows the
player while preserving its authored offset and rotation.

A numerical collider room exercises seven scenarios through the real native
runtime. Public authoring, atomic Undo/Redo and reopening also pass. Seven fresh
development runs pass 53 observations, including arriving at the exit while a
pickup remains without incorrectly winning. Keyboard and gamepad produce the
same results; terminal states stop the player and both enemies. The wall scenario
checks collision, and damage events respect the invulnerability interval.

An ordinary fresh macOS ARM64 shipping export runs the same seven scenarios
after its copied authoring project, private build storage and external script
maps are removed. Logs, observations, tick counts and PCM match development
exactly. All 1,230 captured build inputs stayed unchanged through termination.
The explicit exit-proximity assertion was added afterward and passes in both
development and that same shipping player, with unchanged gameplay. Development
parses scripts at startup; the shipping player uses cooked bytecode and performs
no startup source parsing. The actual source and process evidence is retained in
[the manifest and compressed logs](evidence/core-sample-gameplay.json).

Existing comprehensive macOS, Linux and Windows checkpoint jobs now run the
development scenarios using their already-built headless tool and retain their
results. Hosted results remain pending. The standalone export helper remains
available separately so these added CI steps do not repeat a fresh Rust build.

This fixture proves gameplay behavior, not the finished sample. Claude owns the
terrain, original art, UI presentation and native visual review. Dynamic HUD
bindings, music and reference-device measurements are still being assembled.
Physics checkpoints and Phase 1 acceptance are not claimed.
