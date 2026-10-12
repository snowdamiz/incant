# Disposable0048 numerical fixture

Prepared with preliminary immutable headless source263cacc and SHA4c4cb80869e6b2ceb6c26d70217b0571dbec495701dfe39eb9e4ce09f1c5ae3e. This is fixture authoring and CPU functional evidence only. Root must supply the final editor binary/hash and exclusive graphics/provider grant before native acceptance. No native window, GPU, screenshots, provider or account operation was performed here.

Saved project: `/tmp/incant-native-0048-fixtures/saved/project.json`
Journal: `/tmp/incant-native-0048-fixtures/saved/project.journal.jsonl`
Baseline revision4; tick rate60; the actual authored token cap is1,000,000. No allowance edits or usage reset were made.

Public commands were `init --entities 0`, `import` of the existing repository Blender2.72 cube FBX, three `command.execute` transactions (one script write per transaction), `project.save`, and fresh-process public project/script/history queries. Import provenance is `import`; fixture content provenance is `user`. Exact requests/responses and CLI arguments are in `public-commands.jsonl`. No project JSON, journal or script content was manually edited.

Main scene contains `Stationary Imported Mesh` at origin and `Moving Imported Mesh` starting X=2. Both reference the actual imported model asset, whose checked cooked cache contains1 mesh/36 vertices. Numerical Movement adds0.25*dt to its own attached entity’s X; stationary geometry and both explicit cameras remain unchanged. `Stationary Camera` has identity rotation and translation[0,2,8]; actual source camera convention is local−Z forward. `Second Stationary Camera` is at[0,2,12]. Camera framing/appearance remains Claude-owned; no rendered visibility is claimed.

`Key Probe`: X counts ticks with KeyD held, Y counts ticks with left button held, Z sums vertical pixel and line wheel deltas. Scale X is2 while input.focused and1 otherwise. `Pointer Probe`: translation X/Y records the logical pointer coordinates received. Their exact existing0044 instance-filtered sources are preserved, so neither can change a camera or mesh.

The `Spatial Without Camera` scene contains a real imported mesh and no camera. Public `play.start` refused it with the explicit-camera error recorded in evidence. This is the no-camera notice fixture; use the same saved project and select that scene.

`Allowance Rename Target` is the sole clearly named rename-only entity (ID00000000000000000000000051). It has no geometry/script components. Root’s later grant may authorize one real UI rename and Undo; this script sent no agent request. Set the64,000 cap through the actual UI only when the final grant says to do so. Retain the same project ULID/journal for the account/usage reopen check; a newly initialized clone has a new project identity.

## Final-grant launcher instructions

After root publishes the final exact immutable editor path/hash and grants native graphics, set `EDITOR_0048` to that path. Include the existing bundled Node directory in the launcher PATH for the trusted startup TypeScript cook, without changing accounts or project data:

```sh
EDITOR_0048=/exact/editor/path/from/root-final-grant
PATH="/Users/andreyyurlov/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin:$PATH" "$EDITOR_0048" /tmp/incant-native-0048-fixtures/saved/project.json
```

Choose `Main` and `Stationary Camera` for numerical inspection. Select Key Probe/Pointer Probe/Moving Imported Mesh by the IDs in fixture-evidence.json. To check the saved-project prerequisite, quit the owned process and launch the same granted editor with no project argument; later reopen the exact saved path above. Do not launch the preliminary headless binary as a window or run the historical native renderer probe.

## CPU evidence and reproduction

`python3 -I /tmp/incant-native-0048-fixtures/author_fixture.py` created this fixture. The script deliberately refuses an existing saved fixture. Preserve this existing project for acceptance; adapt the output path only if an explicitly separate fresh fixture is desired.

A fresh RPC owner reopened revision4 with exact source receipts. CPU native startup loaded the retained imported model/camera input;23 ticks verified movement X=2+0.25*23/60, unfocused KeyD does not increment counters, focus scale sequence[1,2,1], one held key/button tick, wheel sum3.5 and pointer[123.25,45.5]. Every state reports gpu_submitted=false. Stop and CPU verification changed no authored or cooked bytes. Public validate passed. Both RPC processes were reaped with exit0 and their PIDs were absent afterward. No process remains owned by this preparation task.

Hashes, project/scene/asset IDs, import dependency metadata, numerical states and commands are attached in fixture-evidence.json, numerical-states.json and artifact-manifest.json. These local results do not approve a phone, performance, live provider or phase gate.
