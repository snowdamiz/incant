# Native Wasm navigation path queries

This slice connects the public Rust module SDK to the same committed native
navmesh and prepared-grid solvers used by TypeScript. It adds copied observations;
movement, off-mesh traversal behavior and converting grid cells to world space
remain gameplay responsibilities. It adds no authoring, navigation builder,
debug drawing or alternate pathfinding algorithm.

## Contract

Existing opcodes 0–35 and their records remain unchanged. The existing
`incant_navigation_v1` namespace adds `find_path` (36), `find_grid_path` (37) and
`read_path` (38). The first two accept a copied fixed-size versioned request and
return the latest result's required byte length. `read_path` copies that result
into initialized caller-owned scratch. Mesh and grid frames have distinct magic;
a new query replaces the previous reply. A failed authorized query clears it.

Mesh requests are 88 bytes, including stable owner ID, original f64 endpoints,
snap distance and requested visit limit. Coordinates must be finite and within
±100,000, snap distance within 0.001–10, and visits within 1–32,768. Validation
precedes narrowing to the existing f32 solver. Grid requests are 48 bytes, with
u32 cell fields checked against u16 before narrowing, an explicit diagonal flag
and 1–65,536 requested expansions. Unknown owners and invalid/out-of-grid requests
are errors. Disconnected or unsnappable mesh routes and blocked grid endpoints
are successful misses, represented separately from failures.

Mesh results contain world-space points, the polygon corridor, visited count,
exact u64 generation and off-mesh traversal records. Each traversal preserves
its big-endian stable link ID, consecutive point indices and direction. Grid
results preserve adjacent cell indices, exact u64 integer cost and generation.
Numeric frame fields use explicit little-endian encoding. There are at most
4,096 points, corridor entries, traversal records or grid cells, with fixed
advertised upper scratch bounds of 245,800 mesh bytes and 32,800 grid bytes.
No native pointer or cache handle crosses the boundary.

At callback start the host captures at most eight navmesh and eight grid Arc
handles. Geometry and checked grids are shared; no world or geometry clone,
project document, JSON serialization, rebuild or authoring validation runs per
query. Queued structural commands and pending copied numeric writes remain
invisible to queries until native commit. Cache generations then follow the
ordinary native rebuild/restore rules.

Navmesh, grid and steering queries share one 32-call callback budget. Mesh/grid
requests also share 65,536 requested search steps, charged before search even for
misses. Existing global query units, host-call, memory and transfer quotas apply.
Result reads validate the complete destination before copying. Authorized errors
remain sticky even when a host-side caller catches them; all numeric writes,
structural commands and logs reject together. A stale epoch cannot poison a
different current epoch. A real Wasm import trap still terminates its callback.

## Verification

Frozen source002 passes 369 tests/doctests in 64 groups through the actual
Wasmtime 49.0.2 and WABT 1.0.42 wasm2c conformance harness, all-target Clippy and
three fresh development processes. The harness checks 125 prepared modules and
626 artifact hashes before removing its private temporary build; 16 navigation
guests and their 80 Wasm/C artifacts remain in the local proof directory with
recorded hashes. An authored Rust guest observes detours, forward/reverse
off-mesh routes, valid misses and rebuild generations after committed source
motion, matching an independent direct-native oracle. Failure guests stage mixed
writes before SDK, raw-frame, native-query, quota, callback and trap refusals.

Seven shared-boundary controls additionally cover caught-error transaction
poisoning, stale epochs preserving current authority/reply, complete destination
validation, shared steering/path quotas, call/memory/transfer exhaustion and
failure clearing the prior reply. Raw and SDK guests cover read-before-query,
cross-kind reply replacement, original f64 boundaries, u32-to-u16 grid ingress
and ignored short-scratch errors after a staged prefix. Native runtime tests
pass 190 tests/doctests in 29 groups; scoped native Clippy, supported runtime/SDK
wasm32 checks, 232 Python tests (four existing skips), 22 Node tests, generated
contracts, formatting and the authoring-free shipping dependency check pass.
See [durable evidence](evidence/native-wasm-navigation-paths.json) for source
identities, commands, logs, review receipts and preserved earlier failed attempts.
Independent root review accepted the frozen source and final evidence for scoped
integration. It rechecked retained artifacts and logs; the 626 prepared-file
validation remains the recorded pre-cleanup probe check. Current-main integration is recorded below; its publication requires the exact fast gate.

Collider fixtures intentionally compare complete entity snapshots and retained
native routes instead of claiming unsupported physics checkpoint support. This
is local CPU capability conformance. It does not establish a fresh ordinary
source-deleted shipping package, full guest checkpoint continuation, reference
phone performance, rendered debug-draw acceptance or a phase gate.


## Actual main137 integration

The reviewed implementation and final receipt are integrated on actual main137
`98e972c4bf2239279e937091c835890389b77a51`. Only STATUS needed an additive
conflict resolution; all 28 accepted implementation/test/probe inputs are exact.
The actual-main ancestry merge was made after all checks terminated, preserving
the tested tree exactly.

[Integration evidence](evidence/native-wasm-navigation-main137.json) binds 1,463
frozen inputs, 16 compressed logs and eight receipts. The current union repeats
369 actual-host cases across 64 groups and 190 native cases across 29 groups,
with zero failed or ignored cases, both all-target Clippy suites and three fresh
development processes. Supported wasm32, 232 Python tests (four existing skips),
22 Node tests, generators, dependency guards and formatting pass. All 626
prepared files for 125 modules were verified before cleanup; 80 navigation
artifacts and three guest source copies remain hashed and independently checked.
[Independent integration review](evidence/native-wasm-navigation-main137-root-review.json)
accepted the unchanged source and exact evidence. No ordinary navigation shipping
package, native window, GPU, provider or device run is added by this integration.
