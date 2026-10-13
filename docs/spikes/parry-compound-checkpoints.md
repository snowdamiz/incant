# Checked Parry compound and contact workspaces

This default-off `incant-checked-restore` extension captures and reconstructs
supported compound geometry and the two native composite contact-workspace
families. It does not open a public world installer or a physics file-save path.
Root-owned Rapier composition and outer runtime compound/tile integration remain
separate qualifications.

`Compound::checkpoint_capture` copies primitive children in native order, exact
poses, child/root AABBs, the actual BVH and its bookkeeping. Only zero compound
flags and absent pseudo normals are supported. Nested/custom shapes, noncanonical
capsules, and unsupported planar children refuse. Planar children retain the
existing tile domain (boxes and exact identity rotation); spatial children retain
the existing bounded compound domain. Signed-zero bits are not normalized.

The root-owned immutable `ValidatedCompound` joins geometry, child bounds and a
checked BVH. `checkpoint_plan` additionally recomputes primitive AABBs with native
arithmetic and compares every bound bit, then merges the root in native order.
It allocates nothing. `from_checkpoint` uses the same non-Clone cumulative permit
for exact child vectors, new primitive Arcs, cached bounds and BVH storage. It does
not call a compound constructor, rebuild a tree or clone a source native owner.
An outer Arc around the resulting compound is the caller's separate charge.

`checkpoint_mass_properties` repeats the pinned native `MassProperties::Sum`
arithmetic in two passes, recomputing stack primitive properties instead of
allocating Sum's temporary Vec. The caller first validates with checkpoint_plan,
admits density and charges both child walks to its work budget. This helper is a
numeric calculation, not independent world/capture authentication.

`ContactManifoldsWorkspace` captures only concrete native compound/primitive and
compound/compound owners. A custom trait implementation cannot gain admission by
returning a known typed enum view. The public constructor accepts the existing
immutable `ValidatedPairState`, its pair row and expected identity, joining detector
keys and manifold indices before constructing private maps and boxed workspaces.
The test-only low-level map controls do not expose a raw production installer.

In deterministic mode, detector records preserve actual IndexMap iteration order.
In fast mode, records use canonical ascending keys. Physical HashMap buckets,
capacity and tombstones are not restored or claimed equivalent. The native query
algorithms visit compound BVH leaves to determine manifold order; their workspace
map operations are key entry/lookup and pure timestamp retain. Focused paired
queries cover population, removal, empty maps, reinsertions and persisted contact
data, comparing ordered manifold bits and canonical workspace records every step.

The cumulative allocation plan accounts for pinned hashbrown table buckets,
control bytes/padding, deterministic IndexMap entries, workspace Boxes, compound
vectors, child Arc payloads and the exact BVH. Actual allocation requests are
measured by a test-only System-delegating allocator in each dimension/mode.
Logical requests exclude allocator bookkeeping; Box/Arc process OOM retains Rust's
ordinary abort behavior. Future query scratch/growth is outside checkpoint
construction accounting.

The isolated native unit harness copies exact production Rust sources. Its test
manifests remove unrelated upstream example-only development dependencies and pin
only the in-source `oorandom` test dependency; its separate lock retains all
shared root package identities. Optional upstream packages can exist in that test
lock without being selected/compiled. This is explicitly not a claim that the
prepared test workspace has the repository's complete lock identity.

Current evidence is retained under
`~/.codex/incant-evidence/parry-compound-checkpoints-001`. Initial offline index and
unused-result test compilation failures are preserved. Source and focused native
results will be frozen before broader qualification. No GPU or device work is
part of this slice.


## Source and CPU qualification

Production is immutable `4a668973077024fa29628a4ebfc519b017697157` over
schema `377b94c44e43517139fc1616f6e19d0822defabe`. Root independently accepted the
helper source in `parry-compound-source-root-20261013.json`, SHA-256
`7835ca0d553088dc3c53b160d27b5867dee5d11949254be61b7d8253a3b82073`.
The source freeze binds 33 owned files and 58 total inputs.

Full Parry native unit suites passed 107 planar and 94 spatial tests in each
mode (201 per mode, zero failed/ignored). The 10 new focused tests per dimension
are included in these totals, not additional coverage. Each dimension/mode runs
180 paired composite contact queries through churn and persisted contact data,
24 exact BVH query comparisons and 12 bit-exact mass replay cases. All mass and
plan observations allocate zero bytes. There are 58 actual workspace allocation
measurements per mode; all stay within the charged byte/call plan. Compound
construction charges match the actual measured allocations for its fixture.

All 15 broader commands pass: both full native suites, strict library Clippy in
both modes, both selected normal dependency graphs, both wasm compile checks,
both ordinary feature-off builds, normal runtime dependency isolation, 31 guard
tests, owned-file rustfmt, generated conventions and whitespace checks. The
Clippy claim is for native libraries; unrelated upstream integration/example
harnesses are not part of that lint claim. Root's separate schema tests are not
counted as this producer's native execution.

The final packet is `docs/spikes/evidence/parry-compound-checkpoints/manifest.json`.
Run `python3 tools/probes/parry-compound-checkpoints/collect.py --verify-only` for
portable replay; add `--originals --local-binaries` on the producer machine to
rehash original records and immediately copied executables. The retained
`assertions.py` separately re-evaluates counts, source identities, selected modes,
actual allocation rows and successful receipts without native execution.
Historical offline-index and test-only unused-result failures remain retained;
no failed run is used as passing coverage. All owned process handles are reaped
and the CPU target is released. Final independent packet review remains pending.
