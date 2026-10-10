# Native ECS numeric column loans

`NativeWorld::with_numeric_columns` supplies the native storage boundary for
PLAN.md sections 2.6 and 6.3 typed bulk scripting. A synchronous scope exclusively
borrows the world and lends a restricted `NumericColumns` handle. Its chunk
visitors expose actual Bevy table allocations: a read-only slice of stable IDs,
a mutable f64 Transform slice, and an optional mutable f64 Velocity slice. There
is no component staging array, document projection, JSON or foreign script host.

Transform row `i` occupies `i * 10..(i + 1) * 10`: translation xyz, rotation xyzw,
scale xyz. Velocity uses stride 3 for linear xyz. IDs are separate from the
numeric buffers. A chunk covers only initialized rows; allocation capacity,
padding, internal entity handles, pointers and derived matrices are not exposed
as numbers. Transform-only entities participate even when the world has never
registered Velocity. Iteration visits tables once, including multiple table
archetypes and sparse-set archetypes sharing a table. Table/row order is stable
within a scope and unspecified between structural changes.

## Layout and borrow audit

The shared Transform/Velocity definitions use `repr(C)` and retain their existing
fields, serde attributes and schema derivations. The ECS wrappers use
`repr(transparent)`. Compile-time checks cover every numeric field offset, total
size and alignment relative to f64, plus wrapper sizes/alignments. These use the
Rust Reference's [C and transparent representation guarantees](https://doc.rust-lang.org/reference/type-layout.html#representations);
the code does not assume that f64 has the same alignment on every target.

The storage casts and changed-tick writes are confined to `columns.rs`. The
audit uses the pinned Bevy ECS 0.19.1 source in `storage/table/mod.rs` and
`storage/table/column.rs`: `get_data_slice_for<T>` requires the matching component
ID/type and returns `UnsafeCell<T>` for the table's initialized row count.
Component IDs come from that same exclusively borrowed World. Distinct
components have separate allocations. Table structure cannot change during the
scope, and the visitor cannot retain safe slices between callbacks. Empty tables
are skipped before taking a first-row pointer. The public API exposes safe
references, not raw pointer functions. No unsafe code is added outside this
module. Miri was not run; the installed stable toolchain has no Miri component.

`step`, `inspect`, `snapshot`, structural `apply`, nested world loans and other
world access cannot overlap the exclusive borrow in safe Rust. Structural
commands can be queued externally during visits and applied afterward. The next
scope reacquires current tables/rows; it caches no pointer across reallocation or
component moves. A caller using unsafe foreign pointers must revoke every foreign
view before Rust component reads/restoration, the current chunk callback returns,
or the outer scope exits. This remains a requirement for the future synchronous
JS loan adapter; the native API does not implement or certify that adapter.

## Mutation and failure semantics

This is a trusted native API, like the existing direct component callback. It
does not validate numbers or promise rollback. A callback error stops that visit
at the first error, leaving its writes and earlier writes in actual ECS storage.
Unvisited numeric columns are untouched; their derived hierarchy can still change
through a parent. The outer operation may catch the error and visit
again to restore its own bounded undo copies while still excluding world
observation. Native early returns, an outer `Err`, and Rust unwinding retain
partial writes unless the caller restored them. Abort terminates the process
without cleanup. Invalid numeric values are the trusted caller's responsibility.

On outer scope exit, every Transform and present Velocity row in visited tables
is conservatively marked changed exactly once, even for read-only/restored
visits. All derived hierarchy transforms are then refreshed once, including after
an error/unwind, so safe observations cannot see stale caches. Empty/unused
scopes do neither. The simulation tick is unchanged. This cleanup is not a
transaction commit or numeric validation. A future script adapter must detach
views, validate values and restore rejected writes inside the exclusive scope.

The native overhead is one flag per table, table scanning, O(K) changed-tick
publication for K visited rows, and one O(N) hierarchy refresh. It does not copy
component values. An adapter's undo storage/restore time is additional and must
be explicitly budgeted for the touched numeric columns; this scope permits that
policy without a second world/project rollback layer.

## Local verification, 2026-10-10

Host: arm64 macOS 26.6 (25G72), pinned Rust 1.99.0. The following commands pass:

```sh
./tools/cargo fmt --all --check
./tools/cargo clippy --locked -j2 -p incant_runtime -p incant_cook -p incant_types --all-targets -- -D warnings
./tools/cargo test --locked -j2 -p incant_runtime -p incant_cook -p incant_types
./tools/cargo check --locked -j2 -p incant_runtime --target wasm32-unknown-unknown
./tools/cargo check --locked -j2 -p incant_runtime --target aarch64-linux-android
./tools/cargo check --locked -j2 -p incant_runtime --target aarch64-apple-ios-sim
./tools/cargo run --locked -j2 -p incant_headless --release -- schema schemas
git diff --exit-code -- schemas
node tools/build_bridge.mjs --check
node tools/generate_sdk.mjs --check
node_modules/.bin/tsc -p sdk/ts/tsconfig.json
python3 tools/check_runtime_dependencies.py incant_types incant_runtime
```

All 29 focused behavior tests and five doctests pass. Eight new behavior tests
cover direct ECS-address equality (which rejects a copied-array implementation),
nontrivial Transform/Velocity writes feeding native integration and rotated/scaled
hierarchy transforms, Transform-only/empty worlds, four table chunks with shared
sparse archetypes, conservative Bevy `Changed` observers, 4,096-entity table
growth/component moves/swap removal/deletion/repopulation, callback error,
caller-owned undo restoration and unwind cleanup. Four compile-fail doctests
reject escaped slices and overlapping step/inspection/structural application;
the fifth executes the documented API. Schema regeneration is byte-for-byte
unchanged, generated bridge/SDK checks and SDK TypeScript compilation pass, and
the resolved runtime dependency boundary remains authoring-free.

These are native correctness and portable compile checks. This change does not
integrate QuickJS/TypeScript/Wasm loan hosts, migrate existing gameplay scripts,
implement other component bindings or prove a performance improvement. No
shipping-profile benchmark, physical-phone run, full-script migration, release
or phase gate is claimed. Hosted CI and independent integration review remain
pending when this local evidence is recorded.
