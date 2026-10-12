# Public weighted-pose Rust game package

This proof authors a real nested 1D/2D animation tree, a legacy two-pose graph
and a timed clip fade through the public command bus. A captured glTF supplies
eight distinct clip durations. The weighted graph also has a nonlooping rest
state; no state requires more than eight active samples. A public imported Rust
module uses the v4 SDK and ordinary atomic graph/animation controls.

Authoring is one reversible transaction with the module selector. Undo/Redo,
save and the editor-default journal path reopened through public RPC with
`--journal` are checked. Model, buffer, guest Wasm and private Rust/SDK build
inputs are deleted before development play or export. Source paths remain
registered, so successful play depends on the captured cooked resources.

The independent oracle cooks the authored document with those captured animation
resources, then runs only the typed native world and native controls. It neither
loads guest code nor decodes an ABI frame. Its exact cooked scene must match the
ordinary export. Every tick produces 48 sample vectors (including explicit
unused-slot sentinels) and nine metadata vectors covering every contribution,
representative clock, interpolation kind, destination playback flags and
timed-transition elapsed/duration. Legacy outgoing clip/time/weight are copied
contributions; outgoing playback flags are not separately asserted by this probe.

The guest writes those fields into native numeric entities and publishes three
per-entity records per tick, each within the existing 1024-byte message limit.
Twenty ticks yield 60 complete records and 3420 scalar observations, checked
against the native oracle. Public vector checks remain bounded to 228; the full
records cover every tick. Pause/resume, reverse speed, parameter boundaries,
manual state selection, rest completion and legacy SLERP behavior are exercised.
The gameplay counter is a native ECS component; no module global is treated as
checkpointed game state.

The authored/source-deleted development preflight, one fresh ordinary wasm2c
shipping export and three fresh packaged processes pass. The package route uses
`--package`, never a static fixture-selection flag. Full records, numeric checks,
accepted clocks and 128,000 PCM bytes match development play after private
project/cache/build/tool/map removal. The exported scene exactly matches the
independently cooked oracle scene. The unsigned aarch64-apple-darwin player is
9,653,488 bytes; this is an artifact size, not a reference-device measurement.

The negative controls request old v1/v2/v3 snapshots, undersized scratch and a
late command targeting an absent entity. Expected diagnostics are exact; the
same input-replay routes first pass positively. Full static guest checkpoints
return the existing explicit unsupported diagnostic. All eleven exact
development/package/checkpoint refusals pass. Package refusals
establish no stdout/PCM/save publication, while the separately accepted actual
host suite establishes staged-prefix rollback.

Runs001–003 remain preserved: a completion assertion used the prior tick, a
full-tick log exceeded the unchanged message bound, and the intended absent ID
was actually a marker. Run004 passed author/development preflight; run005 corrects
only per-record versus per-tick metadata labels and repeats the successful
preflight. These were proof-fixture corrections, not engine behavior changes.

This is local desktop CPU evidence. Rendered appearance, GPU execution, hosted
reference phones, performance/phase gates, blended tree-state transitions,
composite pose operands, complete guest checkpoints and the full SDK remain
outside the proof. Repository engine/SDK sources and shared engine build storage
are retained; cleanup is limited to newly created private proof inputs.

The oracle's scoped Clippy check, actual wasm32 guest compilation, all 235 Python
tooling tests (four existing skips), formatting and generated registry/convention
checks pass. The producer's post-run audit rehashed five frozen sources, all 14
retained package files and 60 command streams, and checked absence of 17 removed
private files. Independent final source/evidence review accepts the immutable proof without
findings. Current-main union validation and integration remain publisher-owned
and pending.

[Evidence inventory](evidence/native-wasm-animation-pose-package.json) binds 75
compressed logs and 43 original receipts plus the independent final review, including literal command history, authoring
transactions, full oracle/process records, refusal diagnostics and all earlier
attempts. The actual package and raw evidence remain at
`~/.codex/incant-evidence/animation-pose-package-005`; they are not transient
`/tmp` artifacts. The separately reviewed exporter environment change is recorded
in [its own evidence](export-startup-environment.md).

To reproduce, build `incant_headless` with `wasmtime-host` and the
`incant_cook` example `animation_pose_package_oracle`, then run
`tools/probes/native-wasm-animation-pose-export.py` with a new private output
directory, `--binary` and `--oracle`. Use `--stage author` to run all public
authoring/development controls before spending a fresh shipping build, followed
by `--stage export` against the same frozen output. The script verifies source,
binary and document identities before export. The evidence inventory records
the exact pinned command/environment used here.
