# Matched shipped Wasm / TypeScript per-frame experiment

This helper prepares the numerical measurement required by PLAN Phase 1: a
shipped wasm2c module's per-frame cost against the same TypeScript system on this
Mac. Source and Python controls are implemented. Public authoring, guest compile,
shipping export, actual player execution, and measurement remain pending the
coordinated CPU/timing lease. Nothing here establishes a speedup or a phase gate.

The source001 producer started from main259 `64d536d5`; source002 composes
actual main260 `64abee88` with the accepted engine caches. It changes no engine
runtime, VM limits, profiler thresholds, gameplay content or backend decision.
The other script-cache optimizations remain separately preserved and reviewed.

## Fixed experiment

Both projects derive from one public `incant_headless init`, retaining project
and scene IDs. Public `incant_cmd` transactions create 1,024 unparented Transform
entities with deterministic ULIDs and distinct initial x coordinates. Input is a
60 Hz, 3,600-tick recording with six right-arrow press/release cycles. The callback
updates every transform in bulk: x accumulates `6 * dt` while held, y records the
accepted tick, z counts press edges. There are no physics, audio, render, UI,
structural commands or per-frame logs. No ECS loan survives an update.

TypeScript attaches one source to the first existing entity; its bulk view covers
all 1,024 rows. Wasm selects one publicly imported Rust-SDK module as the scene
program. The intended authoring differences are the script/asset registration,
scene program selector, first entity's Script component and that entity's public
transaction provenance. Remaining data must compare exactly. Undo/redo/reopen
checks are recorded for entity authoring and each program-selection transaction.
The Wasm import uses the existing public importer.

The guest uses the generated Rust SDK and fixed trusted compiler helper, with
1,024 Transform scratch rows. This is the workload size, not a widened VM budget.
Preparation leaves its own source/build inputs retained for audit.

The exporter runs twice with the same clean committed engine checkout, lock,
compiler, shipping codegen, deterministic physics and summary profiler. The
normal exporter freshly cooks each project and removes private export build
storage. The helper refuses unequal cooked `world.scene`, input-action or runtime
package-data hashes. It also checks the Wasm input hash and pinned wasm2c 1.0.42
identity. The cook manifest also always contains `script.source_sha256`: for a
Wasm-only project this identifies the fixed empty QuickJS artifact, **not** the
Wasm guest. Guest identity is checked against `program.input_sha256`, while the
export retains its prepared/registry/translator identities. Each complete normal dependency graph is retained. The expected host
feature delta is `static-module` on `incant_script` and `incant_game_player`; it
necessarily brings the static Wasm runtime/C host. There is no claim the full
backend dependency graphs are identical.

Four bounded observation plans cover all rows (256 each), and each is executed
on both actual shipping players. The helper independently recomputes every
translation from the input schedule and checks unchanged columns, world poses,
logs, rational audio frames and exact silent stereo PCM. Timed runs use the same
small sentinel plan and still compare both outputs and PCM.

Three fresh process pairs alternate TS/Wasm ordering. The existing
`mac_performance.measure` records actual Mac identity, load context, compiler,
package/player/input/report hashes and raw histogram samples. Both frame and
script intervals include all successful frames; there is no warmup exclusion.
The timings include the corresponding runtime bulk bridge and commit costs,
not only the source arithmetic. Process wall duration is a diagnostic and is
**not** labeled host startup cost.

## Phase selection and comparison

`tools/mac_performance.py` retains `script.tick` as its default. Explicit
`--script-phase script.static_module` requires an actual Wasm export, while
selecting `script.tick` for Wasm refuses. Unknown program kinds/phases and missing
or incomplete histograms refuse. The selected phase is in the result and match
identity (the historical missing field means the unchanged default `script.tick`).
Static-module records cannot enter the existing TypeScript matched 5% regression
or 2 ms policy. Existing default-phase baseline files remain usable.

The paired helper reports p95 interval ratios only. Rational endpoints use
`wasm.lower / ts.upper` and `wasm.upper / ts.lower`; a denominator interval
including zero explicitly has no defined finite ratio. It does not manufacture
an equal workload identity across different backends, assign a pass threshold,
or turn a comparison into a Phase 1 gate approval.

## Execution, after source review and an explicit lease

Run these from the frozen clean repository, using new external output paths.
`incant_headless` must be a source-bound compatible public-authoring binary;
`--wabt` must point to the trusted pinned WABT tools. Preparation compiles the
trusted guest, export compiles two shipping players, and measurement executes
them, so all three require coordination with the parent before invocation.

```sh
python3 tools/probes/wasm-ts-cost/prepare.py "$INPUTS" --binary "$HEADLESS"
python3 tools/probes/wasm-ts-cost/export.py "$INPUTS" "$PACKAGES" --wabt "$WABT"
python3 tools/probes/wasm-ts-cost/measure.py "$INPUTS" "$PACKAGES" "$MEASUREMENTS"
```

Focused lifecycle/numerical Python controls (three short Python child processes; no engine):

```sh
python3 -m unittest discover -s tools/tests -p test_wasm_ts_cost.py -v
python3 -m unittest discover -s tools/tests -p test_mac_performance.py -v
```

Strict TypeScript checking is configured in `tools/probes/wasm-ts-cost/tsconfig.json`;
its pinned Node22.23.3 compiler check is reported separately from the still-pending
Rust guest and actual shipping execution.

The RPC helper retains raw requests, responses, partial malformed/EOF stdout and
stderr. Constructor/operation failures close and reap the owned process group.
Leaf command failures/timeouts retain raw logs and receipts. The exporter stays
in-process so its own child ownership cannot be bypassed by an outer timeout.
Incomplete outputs remain for diagnosis; each retry needs a fresh destination.

Limits: no startup result, GPU/FPS, Core Sample, 10,000-entity, mobile/device or
arbitrary workload result. No physical output, visual judgment or project-selected
executable is involved. The source-only and Python-control results do not replace
actual public tool, guest compiler, shipping or timing evidence.

Source001 remains preserved. A source002 oracle correction uses the actual
NativeLogs wire object (`entries: []`, `dropped: 0`) instead of an array; the
actual-shape regression failed before the correction. This was a helper mismatch
found by source inspection, not a failed native workload or engine result.
