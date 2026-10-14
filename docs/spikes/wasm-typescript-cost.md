# Matched shipped Wasm / TypeScript per-frame experiment

Two fresh shipping exports, eight all-row functional runs and six alternating
Mac timing runs passed for the same 1,024-entity numerical workload. The measured
Wasm phase had a lower p95 interval than the TypeScript phase in each pair. This
is a bounded per-frame comparison, not a general backend result or phase gate.
All 21 focused Python controls and strict TypeScript checking also pass. Public
authoring and trusted SDK guest compilation passed in `prepare-003`; earlier
helper refusals remain retained below.

The source001 producer started from main259 `64d536d5`; source002 composes
actual main260 `64abee88` with the accepted engine caches. It changes no engine
runtime, VM limits, profiler thresholds, gameplay content or backend decision.
The other script-cache optimizations remain separately preserved and reviewed.

## Actual shipping results

Execution source was `97834ab3282e0afb8b80820caf19394c929cf821`, with the reviewed
source005 helper and identical native/compiler/SDK/vendor input closure for both
exports. The actual host reported **Mac17,9 / Apple M5 Pro**, 48 GiB memory,
15 physical CPUs, macOS **27.0.1 (26A434)**, arm64 and AC power. Low-power mode
was unavailable (`null`), not inferred as disabled. Rust was pinned 1.99.0;
shipping used opt-level 3, one codegen unit, fat final LTO and deterministic
physics. The guest was compiled by the trusted Rust SDK helper and translated
with pinned wasm2c 1.0.42.

All intervals are conservative nearest-rank p95 bounds, in **nanoseconds**,
recomputed from 10,000 ns bins. Every one of the 3,600 requested frames is
included. Ordering was TS0, Wasm0, Wasm1, TS1, TS2, Wasm2, during one coordinated
quiet window; no compiler or task GPU workload ran concurrently.

| Pair | TS `script.tick` | Wasm `script.static_module` | TS `player.frame` | Wasm `player.frame` |
| --- | --- | --- | --- | --- |
| 0 | [140001, 150000] | [80001, 90000] | [180001, 190000] | [130001, 140000] |
| 1 | [150001, 160000] | [80001, 90000] | [200001, 210000] | [120001, 130000] |
| 2 | [140001, 150000] | [80001, 90000] | [180001, 190000] | [120001, 130000] |

Descriptive Wasm/TS p95 ratio bounds were approximately [0.53334, 0.642853],
[0.500006, 0.599996], and [0.53334, 0.642853]. Exact rational endpoints are
retained. Requested Rust allocation peaks were 4,370,216 bytes for TS and
4,361,685 bytes for Wasm in every run; these are neither RSS nor GPU memory.
The two phases include their respective bulk bridges and commit work. Three
pairs do not establish a statistical confidence interval or host startup cost.

Eight separate unprofiled players observed four 256-row pages per backend, for
all 1,024 rows after 3,600 ticks each. Independent arithmetic, unchanged columns,
world matrices, logs and full comparable reports matched. All fourteen players
(functional plus timed) produced exactly 23,040,000 silent PCM bytes with SHA256
`e4532117d2f1b8f0fd4a1c6f48ec086c5df6b4829a235648b179d5c9135cce8e`.
The disposable project/script/module paths actually supplied to the exporters
were removed before these runs, as were each exporter's private build trees.
Original authored inputs remain separately retained as evidence.

Player SHA256 identities:

- TS: `2bb3d5cf620c1fb2b0e076a0d3d1a5349816ab6b8a575e1675b658978656a052`
- Wasm: `725aabc6d0b578427988ea6973c6e992292961ec2380bc4be2ae70ba23bbb9bc`

The [portable manifest](evidence/wasm-typescript-cost/manifest.json) binds all
source generations, raw public RPC traces, failed preparations, compiler logs,
features, functional reports and timing histograms. Its lossless deduplicated
archive is about 3.6 MiB; tools/players/symbols, guest artifacts, redundant
journals/cache and PCM remain explicitly hash-bound external files. Verification
does not execute a compiler or player:

```sh
python3 docs/spikes/evidence/wasm-typescript-cost/verify.py
python3 docs/spikes/evidence/wasm-typescript-cost/verify.py --external --git-source
```

The latter passed for 617 original records, 297 unique embedded payloads,
41 source005 files and 7,763 immutable Git source rows across the headless and
shipping generations. The independent verifier recomputes native row results,
undo/redo/reopen transitions, actual target compiler flags/features, every p95
interval and ratio; five malformed-report controls refuse. Source and protocol
reviews are retained; final packet review remains separate.

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
its pinned Node22.23.3 compiler check is reported separately from the completed
actual shipping execution. The trusted Rust SDK guest compiled with pinned
Rust1.99.0 during preparation; neither compiler check measures runtime cost.

The RPC helper retains raw requests, responses, partial malformed/EOF stdout and
stderr. Constructor/operation failures close and reap the owned process group.
Leaf command failures/timeouts retain raw logs and receipts. The exporter stays
in-process so its own child ownership cannot be bypassed by an outer timeout.
Incomplete outputs remain for diagnosis; each retry needs a fresh destination.

Limits: no startup result, GPU/FPS, Core Sample, 10,000-entity, mobile/device or
arbitrary workload result. No physical output, visual judgment or project-selected
executable is involved. Neither this comparison nor its small-workload intervals
satisfy the TypeScript 10k/Core 2 ms policy. The source and Python controls remain
distinct from the actual public tool, guest compiler, shipping and timing evidence.

Source001 remains preserved. A source002 oracle correction uses the actual
NativeLogs wire object (`entries: []`, `dropped: 0`) instead of an array; the
actual-shape regression failed before the correction. This was a helper mismatch
found by source inspection, not a failed native workload or engine result.

Source002 public preparation subsequently compiled the real trusted SDK guest
and imported it, then exposed a second helper mismatch: the asset's `sha256` is
the captured policy/registry/input/prepared fingerprint, not the raw guest hash.
That failed run remains retained. Source003 binds the original hash separately,
recomputes the pinned capture fingerprint, retains the actual import receipt,
and requires all four identities to match the eventual export. This changes no
guest bytes, workload, engine behavior or measurement threshold.

The next public run completed its transactions but source003's strict document
oracle omitted serialized input defaults. Source004 explicitly authors/checks
the native button threshold `0.5` and key scale `[1, 0]`; both prior actual
projects already contained those values. The failed generation is preserved.

The successful fresh `prepare-003` run used the immediately copied headless
binary from source `297439d7`, explicitly built for `aarch64-apple-darwin` with
the CI profile and deterministic physics. Native/build/SDK inputs were unchanged
by the helper corrections. It verified all 1,024 IDs and transforms in both
projects, the sole intended attachment/program delta, 11 retained input files,
and public transaction undo/redo/reopen. No native guest tick or performance
sample was executed during preparation.
