# Native drawable presentation observer: CPU-qualified probe

This private, default-off probe passed its bounded CPU qualification. It has no
native window, callback-delivery or timestamp evidence.
It targets one missing observation: when the exact drawable already acquired by
wgpu actually reached the display. It changes no pacing, rendering, FIFO policy,
command buffer scheduling or workload. It is separate from the private MetalFX
quality experiments and from CPU host-cadence histograms.

## Boundary and primary contract

Apple documents [`presentedTime`](https://developer.apple.com/documentation/metal/mtldrawable/presentedtime)
as host-clock seconds for onscreen presentation; zero can mean not presented or
dropped. [`addPresentedHandler`](https://developer.apple.com/documentation/metal/mtldrawable/addpresentedhandler(_:))
supplies the actual drawable after presentation. Missing callback delivery is not
evidence of either a zero timestamp or GPU completion. These APIs and
[`drawableID`](https://developer.apple.com/documentation/metal/mtldrawable/drawableid)
require macOS 10.15.4. The ID is monotonically allocated by its owning layer and
starts at zero. Read values in the callback, not from callback arrival time.
The intervals include presentation scheduling delays; they measure displayed
cadence, not uncapped GPU throughput or GPU execution duration.

Pinned wgpu 29.0.4 exposes a borrowed HAL surface but not its current drawable.
The small patch to wgpu-hal 29.0.4 therefore records the existing successful
`nextDrawable()` result in `metal/surface.rs`, carries a ticket in its
`SurfaceTexture`, and installs the handler immediately before the existing
presentation path in `metal/mod.rs`. All new code is gated by both macOS and the
new `diagnostic-presentations` feature. Existing present/commit/transaction wait
calls are untouched. The engine's Cargo manifests, lock and player remain exact.

## Ownership, bounds and coverage

Before capture, allocate one collector for one surface lineage, at most 36,000
rows. Each row is compile-time bounded to 64 bytes: at most 2,304,000 bytes of row
storage, plus the fixed collector/mutex/Arc headers. The native callback holds
only `Arc<Collector>` and a row index. It holds no drawable, layer, window,
renderer, device or callback. Dropping the host or replacing its binding is safe:
late callbacks can only finish their original collector row and context.

Each successful acquisition freezes trusted host `(surface_generation, segment,
host_step)` plus the real drawable ID. The host must serialize binding/acquisition,
use a fresh generation for a recreated surface, and break segments on focus,
suspension, resize/reconfiguration, retry, restart or intentional capture gaps.
It must bind even startup draws explicitly (host step zero is supported), report
failed/deferred acquisitions separately, and never share a collector across live
surfaces. Initial/redraw-only presentations must be labeled separately from
accepted host steps in the later player report, not counted as accepted ticks.
The standalone probe does not yet implement that native-player wiring.

One block allocation per admitted presentation and small mutex/Arc operations
add observer overhead. The fixed row allocation does not imply zero allocation
in Apple's block/runtime machinery or unchanged performance. Once full, the
collector increments an explicit overflow counter and registers no more handlers;
it never stalls or changes rendering to preserve samples. At most one block is
registered per stored row. A callback only changes its already allocated row.

Rows distinguish acquired, presentation requested, positive finite timestamp,
reported zero, invalid timestamp, explicit discard, and abandoned texture. A
requested row with no callback stays requested at snapshot time. Raw coverage
counts, segment identity, overflow and protocol errors are retained. A snapshot
is taken outside frame work after unbinding; it makes a bounded row copy and may
hold the collector lock during that copy. Further late delivery can be observed
in a later snapshot; it is never guessed complete after a timeout.

Raw intervals use acquisition order and validate drawable/context/timestamp
ordering. Callback completion order does not reorder frames. A reported zero or
discard between two displayed endpoints remains in the full elapsed interval.
Segments never join. Missing/abandoned/invalid rows, overflow or protocol errors
refuse interval extraction; reversals refuse instead of sorting them away.
Leading/trailing zero or discard rows remain outside any positive endpoint pair
and must be disclosed. An empty interval array or partial segment is not a pass.
There is no p95, FPS, performance gate, or aggregate-to-display conversion API.

## Source and bounded CPU qualification

All implementation lives under `tools/probes/native-presentation-observer/`:
`wgpu-hal.patch`, `presentation.rs`, `presentation_native.rs`,
`presentation_tests.rs`, `host.rs`, `pins.json`, and `prepare.py`. The preparer
checks the archive SHA against Cargo's checksum, checks three exact preimages,
refuses unsafe archive members, applies a zero-fuzz patch to a create-only copy,
and inventories the exact six-file HAL delta. It never edits the registry or the
engine. The trusted text patch process has a ten-second deadline.

Ten CPU controls passed: bounded storage/overflow; invalid
contexts; out-of-order callback arrival; zero/discard gap retention; missing,
abandoned and nonfinite timestamps; late collector/context lifetime; duplicate
completion; segment and surface transitions; identity/timestamp reversals; and
leading/trailing/unfinished segment coverage. The test model compiles the exact
copied collector source. The host adapter separately compiles against the real
patched HAL API, so the pure model does not substitute for that API check.

The executed qualification used one coordinated native-builder lease:

1. Prepare a create-only external source package from the pinned archive. Resolve
   its standalone lock offline, retain the resolver output, and require every
   registry name/version/checksum to match the engine lock. Only the new root and
   local patched wgpu-hal may differ. Preserve the original engine lock copy.
2. Run observer-on library CPU tests, observer-on strict all-target Clippy, direct strict HAL library Clippy, then
   observer-off library check. Use Rust 1.99, explicit `aarch64-apple-darwin`, CI
   profile, locked/offline, jobs 2/Rayon 2, one serial command at a time. Retain
   actual compiler features and immediately copy the test binary and `--list`.
3. Bound each command to 900 seconds, stop on first failure, keep at least 6 GiB
   free, permit at most 2 GiB target growth and reserve space before artifact
   copies. Use existing managed process cleanup. No native window or GPU work is
   included in this qualification.

## Route to a product feature

If CPU and a separately reviewed actual-drawable probe succeed, adopt the small
HAL seam through a pinned, reviewed dependency patch or upstream release. Do not
leave registry mutation/manual patching as a shipping requirement. Add a
default-off `native-presentations` feature on the native player and bind through
its existing `window/surface.rs` ownership, with trusted host-step/segment tags
and final raw report output. Normal exporter feature graphs remain unchanged;
diagnostic packages identify the observer and its overhead. Integrate the
already-reviewed native replay only after its source lands. That product step,
actual callback delivery, acquisition-error coverage and display measurements
remain pending; this source package does not claim them complete.

## Actual CPU result and retained failures

Frozen source `90ef0f2e48560380e9f5bb272f031d27d1c413fb` passed all four
commands in generation 007: ten collector tests, strict host Clippy, strict
patched-HAL library Clippy, and observer-off checking. The actual dependency
feature trees confirm the diagnostic feature appears only in the observer arm.
The model and patched HAL both use the upstream HAL's Rust 2021 edition. The
engine workspace, lock and player were not changed by this qualification.

External evidence is retained at
`~/.codex/incant-evidence/native-presentation-observer-001/qualify-007/`.
Its `result.json` SHA256 is
`91cf30c2251ccb9b1215a1cc6b74f9450751239f770b99fb4f7e742ab1b8d59b`;
the immediate test executable SHA256 is
`31aee45c2039a6a40be57f0f4e1fb4ab0767dccb80dc0c20bc68925f1c162262`.
The supervisor completed and reaped all children in 52.69 seconds. Minimum
sampled free space was 125,858,603,008 bytes; sampled target growth was
18,097,416 bytes, within the unchanged 6 GiB floor and 2 GiB growth bound.

Every earlier attempt is preserved: generation 003 passed ten tests then failed
strict lint on a nested conditional; 004 exposed the upstream Rust 2021 syntax
constraint; 005 passed ten tests and host lint then Cargo refused feature flags
for a selected dependency outside the workspace; 006 refused offline resolution
after making HAL a workspace member expanded dormant platform dependencies. The
final harness restores the original dependency layout and selects the root probe
and HAL together, forwarding the feature through the root. It does not remove
production dependencies, fetch new packages, relax assertions, suppress lints or
change collector behavior. None of these attempts launched a native window, GPU
workload, listener or player. CPU success does not establish real display data.
