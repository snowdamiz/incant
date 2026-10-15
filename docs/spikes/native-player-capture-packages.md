# One-session packages for selected-frame evidence

Source implementation and portable controls only. No package build or native
player/GPU execution has qualified this helper yet. The ordinary public
`export_native.py` command retains its fresh-target contract and unchanged flags.

The trusted diagnostic reuses `shipping_session.FreshSession`, which is already
used by the shipping proof suite. It starts with absent ephemeral build storage,
binds the compiler, frozen engine sources, target and shipping profile, and joins
each actual Cargo target artifact to its original compiler command and file
hashes. Identical shared units may be reused only from that same session. It
does not load a persistent or project-supplied Cargo cache.

Each locale still gets a separately compiled scene-bound player library and
executable. Before the next build, `begin_next_player()` checks the previously
packaged and hashed players, then retires only the exact engine player lib/bin
entries into preserved unit history. Cached player units are no longer proven
and therefore refuse. Shared-unit entries stay immutable: changed bytes,
features, output identity or unexpected rebuilding still fail. The ledger reports
shared fresh origins once and retains every retired player origin; it does not
describe four complete fresh engine builds.

The jobs file is trusted host input, with one to four records:

```json
[
  {"name": "Core-en", "project": "/absolute/english-authoring/core.json"},
  {"name": "ar", "project": "/absolute/arabic-authoring/project/core.json"}
]
```

Only `name`, absolute `project`, and optional canonical `scene`/`camera` ULIDs are
accepted. Jobs cannot select commands, features, tool paths or executable shader
bytes. This slice uses the ordinary window-capable build graph for later offline
capture and refuses static-module scenes before building. It does not open a
window. Every job's full authoring inventory is copied without links or special
files, checked before/after compilation, and retained as hashes. The producer's
project/scene/camera, project-build receipt, program metadata and packaged
executable remain bound separately for each job.

```sh
python3 tools/probes/native-player-capture-packages.py jobs.json NEW_OUTPUT \
  --seconds 3600
```

All packages are built before any player is launched. After packaging and hashing
them, the helper removes every disposable authoring copy and the entire shared
build directory. Only then does each package run its own one-tick offline CPU
startup with a 640×480 logical UI viewport. It requires `tick == 1` and
`gpu_submitted:false`; startup PCM sample frames are not gameplay ticks. The
complete batch manifest is published last. A later failure preserves earlier
independent results, partial files and its original error, with no complete batch
manifest or retry.

Data limits are four jobs, 4,096 files and 256 MiB per authoring tree, 512 MiB
combined inputs, and 64 MiB per raw Cargo log. Managed Cargo execution uses the
existing process-tree owner, a 2,400-second child deadline and a 6 GiB free-space
floor; CPU startup is bounded to 60 seconds. The cooperative whole-session limit
is explicit, 60–7,200 seconds. A separately reviewed supervisor must still bound
aggregate scratch/package/log storage, RSS, concurrency and exact tool identities
before execution. No aggregate memory bound or performance result is implied.

Portable controls exercise exact shared-origin reuse, required fresh player
units, changed package/source/artifact refusal, raw failure retention, input and
log bounds, output collisions, separate per-job outcomes, and the global deletion
barrier. Existing shipping code-generation, UI-export and Wasm-routing controls
remain applicable. These are data/fixture tests, not actual export, compiled-script
parity, Metal, locale appearance, native-window or phase-gate evidence. Actual
selected-frame collection remains a separate, source/package-bound GPU stage.
