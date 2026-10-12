# Native Wasm local avoidance

The additive `incant_navigation_v1` namespace exposes the same native local
avoidance solver used by TypeScript. Opcode29 `steer(epoch, ptr, len)` consumes a
bounded copied request and returns its reply length. Opcode30
`read_steering(epoch, ptr, capacity)` copies the latest reply. Existing opcodes0–28,
wire records and namespaces are unchanged. Generated Rust/C/Wasmtime dispatch
comes from `sdk/abi/native-v1.json`; runtime code never reads that registry JSON.

Callers supply explicit world-space agent samples and obstacle segments or convex
polygons. The solver returns owned velocity proposals in canonical stable-ID
order. IDs correlate inputs and need not name existing entities. No geometry is
gathered from the world, and no entity moves unless the caller applies a proposal
through ordinary numeric writes. This is not a navmesh/pathfinding binding.

`Tick::steer_agents` is allocation-free in the no_std guest. The caller owns both
initialized scratch regions; returned rows borrow only result scratch. The
[actual Rust sample](../../sdk/rust/samples/steering.rs) uses two agents and a
rectangle obstacle, checks sorted IDs and neighbors, copies proposals into native
transforms and emits committed diagnostics. Host validation/solver scratch is
bounded but not allocation-free. No performance claim follows from these checks.

## Bounds and transactions

The request uses magic INCSTR01, little-endian numeric records and canonical
big-endian16-byte IDs. It contains a32-byte header,104-byte agent records and
40-byte obstacle headers followed by16-byte vertices. The result uses INCSTV01,
an8-byte header and40-byte rows. Counts, exact frame length, finite numbers and
reserved bytes are checked before iterating. Request/result maxima are17,192 and
5,136 bytes. Encoders validate the entire frame before touching caller output.

Each update admits32 requests,128 aggregate agent samples and128 aggregate
obstacle edges. A request has at most32 obstacles, with3..32 vertices per polygon
or exactly two per segment. Every request charges one of the shared1,024 query
units; localization charges four. Existing memory, call and copied-transfer
limits still apply. Scalar and velocity domains are checked in their original
f64 precision before narrowing to the f32 solver, matching TypeScript ingress.
The callback step is1/240..1 second; horizons are step..10 seconds. Native
validation still enforces IDs, geometry and representable solver inputs.

A new request replaces the old reply. Begin, finish, abort or rejection removes
it. Neither host retains guest pointers. Short or out-of-range destination reads
publish no bytes. An authorized current-epoch failure latches whole-transaction
rejection even when a direct caller catches it; SDK-local errors similarly poison
the callback. Numeric edits, structural commands and diagnostics staged before a
failure are rejected together. Stale epochs fail without poisoning a separate
currently valid update. Actual host traps permanently fail the module instance.

## Validation and review

Wire tests cover truncation at every prefix, unaligned storage, counts, finite
values, reserved bytes, exact lengths, canonical result IDs and unchanged output
on refusal. Direct capability tests exercise native solver equivalence, no
implicit world mutation, query/agent quotas, stale epochs and transaction rollback.
Actual compiled Rust guests run in Wasmtime and wasm2c hosts; success matches an
independent native boundary result across ticks and development reload. Ignored
capacity/domain/number/duplicate/count/quota/rounding errors and callback failure
reject numeric/despawn/log prefixes. Raw imports cover pointer/length/epoch/frame
errors and forbidden start-time use.

Independent source review found that finite f64 inputs just outside a domain
could round onto its valid f32 boundary. The old code accepted22 of23 domain
controls, including negative subnormal margin, too-small radius and too-large
position/velocity/horizon. The correction validates the original numbers before
conversion; all23 controls now reject and retain the original world/log state.
Solver arithmetic is unchanged. The earlier red run is retained as causal evidence.

Final command/log/source identities are retained in
[evidence](evidence/native-wasm-steering.json). The development helper's three
fresh processes exercise its existing bulk fixture, not a new source-deleted
shipping steering package. No phone, shipping steering performance, automatic
obstacle collection, pathfinding, debug draw, full SDK or Phase1 gate is claimed.

## Main128 integration: shipping graph selection

The first integrated dual-host probe stopped before conformance execution because
its workspace metadata request also enabled the development Wasmtime host. With
the new optional script development host in main, this feature selection exposed
that compiler edge to the shipping-root traversal. The audit now requests only
the embedded shipping fixture; all six shipping roots, normal edges and the
compiler denylist remain unchanged. The actual JIT/static conformance and Clippy
commands still select the development host explicitly. Independent review
accepted the correction; three real subprocess metadata regressions pass.
The original failure, exact red/green metadata hashes and review are retained in
[evidence](evidence/wasm-steering-main128-graph-selection.json). Full integrated
conformance is rerun on the frozen corrected source.
