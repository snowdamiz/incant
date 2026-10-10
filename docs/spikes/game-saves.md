# Versioned logical game saves

The host can persist an isolated `PlaySession` with `save_text()` and restore a
new session with `from_save(authored, compiled_source, text)`. This implements
Phase 1's initial versioned game-state format. There is no new project mutation
path: runtime edits continue through the simulation command bus, and saved
projects are fully validated before a new engine is constructed.

## Format and boundaries

`GameSave.schema.json` and the generated SDK type describe the JSON envelope.
Version 1 has an explicit `incant-game-save` marker, authored-project and compiled
script SHA-256 revisions, tick/time, the runtime project and JSON behavior state.
It preserves stable entity IDs, hierarchy, component values, spawns/deletions,
velocities and gameplay data such as inventory or quest flags. The project hash
uses canonical text. A successful hot reload changes the saved script revision;
failed hot reloads leave it unchanged.

Loading rejects a different authored project/script revision, unknown format or
version, unknown envelope fields, malformed documents, invalid clocks and changed
resource manifests. Project identity/settings/assets/scripts must match the
authored baseline. Save input is at most 18 MiB, its project at most 16 MiB and
behavior state at most 1 MiB. Restore builds a separate session; failed loading
cannot damage an existing session or authored files. Saves carry data only; the
host supplies the compiled behavior and project-bound assets.

Physics is reconstructed from logical body state. Contact warm starts, sleeping,
overlap history and exact rollback are not preserved. Existing sensor overlaps
can emit fresh enter events on the first restored tick. Persistent pickup flags
or deleted pickup entities belong in saved gameplay data. VM module globals,
closures and continuations are rebuilt; durable values belong in the behavior's
explicit `state`. Prior log records are not replayed. Cross-revision migrations,
native save controls and mobile/device storage APIs remain open.

## Host file behavior

`play --save-output NEW_FILE` stages a same-directory temporary file and publishes
it atomically with no overwrite after successful playback/log completion. A
concurrently created destination wins and is preserved. Existing files, symlinks
and frame/report/log collisions are rejected. Failed ticks cannot be exported or
retried as committed play state, since physics may have advanced before the
script failed. An earlier valid save remains loadable.

`play --load-save FILE` restores before ticking. `--ticks N` means N additional
ticks; `start_tick`, state timestamps, logs and optional frame filenames use the
absolute game clock. Neither operation requires a provider account, network,
editor or GPU. Save files and captures have no filesystem capability in the
script sandbox. The host owns file IO. Publication is atomic, with a synced file;
no stronger power-loss or filesystem durability guarantee is claimed.

## Verification

At `d6106c7`, 172 Rust tests (nine new save tests), 315 UI tests, five tool tests,
workspace Clippy, generated contracts, strict TypeScript and the native release
build/package pass. Save tests exercise nested data and exact clocks, runtime
spawn/delete/hierarchy, physical body pose/velocity restoration and live raycasts,
hot reload identities, invalid/oversized data, failed ticks, output collisions,
concurrent destination creation and separate-process restart.

The initial tests exposed serde_json's default approximate float decoding:
0.39999999999999997 seconds became 0.4 and one body quaternion changed in its last
bits. Enabling its `float_roundtrip` feature consistently across the workspace
fixes this without changing the JSON wire format. The same tests now preserve
serialized values exactly. Free-flight physics continuation is checked within
1e-5 m, without claiming contact/rollback equivalence.

The public probe creates a project and durable history through RPC, checks a
TypeScript behavior before compiling it, then runs 19 ticks and saves. A new
process resumes another 41 ticks, saves again, and a third reopens at tick 60.
Inventory, runtime spawn/deletion and final scene/script state equal uninterrupted
play exactly; continued logs equal its tick-20–60 suffix. Authored project/journal
hashes are unchanged. Primitive-physics and character CLI regressions also pass
under the precise decoder. Run this without rendering:

```sh
tools/cargo build -p incant_headless --release --locked
python3 tools/probes/game-saves.py artifacts/game-save-example
```

Three alternating local 1,000-entity trials (120 ticks and 120,000 commands each)
measure median p95 of 8.471 ms on the main baseline and 8.360 ms on the save tree.
The paired samples show no observed tick-cost regression from precise JSON
decoding in this workload. They include concurrent development load and exclude
rendering; they are not a complete-game or physical-device performance gate.

The existing hosted GPU playback test now resumes at tick 37 and compares the
actual tick-37 and tick-60 PNGs, final state and logs to uninterrupted playback.
It also checks that a failed captured run cannot publish a save. This extension
was compiled locally but intentionally not executed during the capture pause;
its hosted result is pending.

The director's computer-use/capture pause remains active. No new local captures
or UI operations were used for this increment. Hosted desktop workflows include
the same save probe and existing build/GPU tests; their results remain pending.
Machine-readable evidence is in
[game-saves-2026-10-09.json](evidence/game-saves-2026-10-09.json).
