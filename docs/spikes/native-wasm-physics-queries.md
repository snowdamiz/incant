# Native Wasm physics queries

This slice connects the public Rust module SDK to the retained native Rapier
query implementations already used by TypeScript. It adds read-only query
capabilities and copied observations. Applying character displacement remains
ordinary gameplay through native numeric columns; the query never moves a body,
steps physics, applies an impulse or changes a project document.

## Contract

Published operations 0–38 and all previous record layouts remain unchanged.
The additive `incant_physics_v1` namespace has `query_batch` (39),
`trigger_events` (40) and `read_result` (41). Typed Rust `query_physics` accepts
up to 64 heterogeneous 3D rays, 2D rays and character movement requests in one
bounded frame. Results retain input order and borrow caller-owned scratch.
`physics_events` returns complete sorted, nondraining 2D or 3D sensor enter/exit
observations from the last committed solver step.

Each request/result starts with an eight-byte versioned magic and explicit
counts. Request rows are 112 bytes, query-result rows 96 bytes and event rows
40 bytes. Event frames use a dedicated 32-byte header carrying the active scene
ID and dimension once per complete frame, including empty frames; retained
SDK buffers expose both through `scene()` and `dimension()`. Stable IDs use the existing canonical big-endian 16-byte encoding;
scalar fields use little-endian encoding. Union tags, boolean flags, reserved
fields, unused payloads and complete frame lengths are checked. The request
limit is 7,184 bytes. A batch result has at most 4,096 aggregate character
collision IDs and 71,696 bytes. Event results retain the native maximum of
131,072 rows and 5,242,912 bytes. Smaller initialized scratch works when the
complete result fits. Oversized results fail explicitly; no truncation or
partial destination write is allowed.

Rays preserve native normalization, solid-origin hits, masks, sensor selection,
optional excluded stable ID and deterministic closest-hit ties. Finite f64
origins/directions and distances are checked against the native ±1,000,000
coordinate and 0–1,000,000 distance bounds before native f32 conversion.
Character requests preserve the existing options for skin gap, slide, slope,
ground snap and autostep. Native validation additionally checks the requested
displacement against the callback's actual dt and the owner's collider/body
configuration. Returned collision IDs are sorted and unique. The SDK carries
explicit optional fields; unsupported or malformed variants reject.

Each batch charges one shared native query unit per row; an event observation
charges one unit. Physics, navigation and localization use the existing
1,024-unit callback budget. At most 128 character queries run per callback.
Existing host-call, memory and transfer limits apply. Failed authorized requests
or reads clear the prior reply and poison the complete native transaction,
including numeric writes, structural commands and logs. Ignoring an SDK error
also rejects the callback. A stale epoch cannot poison a different current
callback; an actual Wasm import trap still ends its caller.

## Native ownership

Both hosts run guest code inside the existing synchronous
`NativeWorld::with_numeric_physics` loan. The solvers are temporarily moved,
not cloned, rebuilt or serialized. A `NativeQueries` capability carries the exact
`WorldIdentity`; matching scene/entity IDs cannot authorize another world.
Binding is allowed once per active callback and is checked against its epoch.
The native scope guard restores solver ownership and expires retained query
capabilities on success, error and unwind. Hosts explicitly revoke the binding
before native commit. An unrevoked binding makes commit fail.

Queries see committed solver poses and the previous step's events. The Wasm
numeric columns and command queue remain copied/staged until successful commit.
There is no native pointer in guest memory, per-tick document work, command-bus
mutation, JSON serialization or alternate solver algorithm.

## Verification and limits

Production freeze002 adds scene identity to copied events after independent
review. The final actual Wasmtime/wasm2c proof passes 382 tests/doctests in 67
groups with zero failures or ignored tests, all-target Clippy and three fresh
development processes. A separate 190 native runtime tests in 29 groups and
all-target Clippy pass. The ten focused wire/shared-boundary tests, wasm32
runtime/SDK checks, 233 Python tests (four existing skips), 22 Node tests,
generated files, formatting and lean dependency checks pass. The static-player
normal dependency graph excludes Wasmtime, the Wasm cooker and authoring crates.

Independent source review accepted the exact 20 production hashes and unchanged
published ABI prefixes. Final durable evidence review accepted the scoped implementation. The subsequent
[current-main integration](evidence/native-wasm-physics-main144.json) passes the
same 382 actual-host cases plus 202 current native cases and the ordinary public
package proof. Historical producer counts below retain their original scope.
The [durable evidence](evidence/native-wasm-physics-queries.json) binds all source
and test inputs, literal commands, compressed logs, manifests and source-review
receipt. Final proof003 prepared 140 modules; all 701 files were independently
rehashed before scoped temporary-input cleanup. At proof completion, fifteen physics guests retained
75 Wasm/C artifacts and three source copies. Those private `/tmp` artifacts
were lost after a restart. The final review freshly verified the durable sources,
21 compressed/raw logs and ten original receipts; the 701-file preparation and
75/3 retained-file checks are historical producer evidence, not fresh artifact
verification after restart. The final review receipt is now retained in the
repository.

Earlier run logs and receipts remain preserved in the repository. Proof001 exposed a test assumption about numeric
chunk order after physics changed archetypes; the assertion now checks the actual
changed row. Proof002 passed 382 tests/doctests in 67 groups and all-target Clippy
but was interrupted before its final development probe returned. Its result
remains incomplete. Before the restart, recovery rehashed all 701 prepared artifacts for 140 modules,
75 retained physics artifacts, three guest sources and both completed logs.
The final proof uses the scene-bearing event contract.

The real guest compares ray, character and event observations against a separate
direct-native oracle, then applies movement through normal numeric columns.
The fixture includes solid tile cells, gaps, sensors, masks, exclusions, walls,
sloped ground and committed enter/exit steps. Failure controls exercise ignored
SDK scratch/value errors, invalid native owners/options, shared quotas, late
native commands and traps. Shared-boundary tests separately cover exact world
identity, stale versus active authority, reentrant/expired loans and unwind
restoration. Physics world snapshots/checkpoints remain outside this slice;
rollback evidence compares complete entity snapshots, solver states and events.

These are local CPU capability checks. The separate
[public physics package proof](native-wasm-physics-package.md) now exercises
ordinary source/cache/build/map-deleted shipping players against a direct native
oracle. Physical-device performance, whole guest/physics checkpoint, full SDK,
rendered physics-debug acceptance and phase gates remain outside these results.
