# Inert integration parameters and persistent CCD cache

This extends the checked, std-only `incant_physics_snapshot` records toward a
complete physics owner. It does not construct a native solver, install a cache,
change save/session formats, or remove `PhysicsUnsupported`. No normal solver,
serde, runtime or document dependency is added. The shared decoded records remain
inert and the crate continues to forbid unsafe Rust.

## Integration configuration

`IntegrationConfig` preserves the pinned Rapier 0.36.0 rigid integration fields:
dt, minimum CCD interval, both frequency/damping spring pairs, warmstart scale,
length scale, five normalized tolerances/velocity limits, four usize iteration
counts represented exactly as u64, clustering/recycling and both friction-pass
switches, plus the spatial friction enum. Planar absence is a distinct enum tag,
not a substituted spatial friction model. Unknown flags, enum tags, precision,
dimension, build identity and reserved words refuse.

Every float is its original f32 bit pattern. Signed zero remains exact. Finite
nonpositive dt is admitted as native collision maintenance; it does not advance
bodies or require a nonzero solver count. For positive dt, zero solver iterations
and a nonfinite reciprocal refuse. Native base zero can nevertheless be viable
when every active group has positive body-added iterations; the actual native
control proves this, while the scalar format deliberately refuses it until that
owner-dependent arithmetic is admitted. Zero PGS/stabilization counts and zero CCD
substeps remain meaningful switches. `f32::MAX` is the native disabled-cap sentinel
for maximum corrective and linear velocities: multiplication by length scale is
intentionally bypassed for those two fields only.

The supported rigid domain requires finite nonnegative frequency/damping,
minimum CCD interval and normalized coefficients, a positive length scale,
warmstart in [0,1], finite scaled coefficients, and no active zero-frequency /
zero-damping spring. These are explicit stricter admission policies, not a claim
that every value assignable to a public native field is safe. Values are never
clamped, normalized, cast through JSON, or replaced with defaults by the codec.
Iteration counts retain all u64 values without allocating or iterating by their
value. **They do not authorize unbounded native work.** Complete-owner admission
must check destination usize width, sums with body additional iterations, total
step/CCD work, actual substep reciprocal/spring arithmetic, and derived world
mass/geometry arithmetic before construction. This scalar section does not prove
those remaining numerical or execution-budget obligations.

Soft-body configuration is an explicit pinned-default-only tag for this rigid
subset. Trusted capture checks the entire pinned soft settings serialization,
including the sign of zero, against its default before setting that tag. A changed
setting produces `UnsupportedIntegration`; it is never silently discarded. Any
actual soft owner remains excluded by the existing body and owner witnesses.
The tag and matching build identity do not authenticate a hostile producer.

`ICINT001` is exactly 144 little-endian bytes: the shared 44-byte identity header,
thirteen float words, four u64 counts, four flag bits in one word, friction and
soft-settings tags, and a reserved zero word. Decode uses fixed stack storage.
Encode charges all 144 output bytes to the shared allocation budget.

## Persistent CCD state and native observer

The pinned `CCDSolver` contains one private field that serde deliberately skips:
`Option<(Real, Option<Vec<(ColliderHandle, Aabb)>>)>`. Its actual states are:

- Outer `None`: uninitialized or invalidated.
- Inner `Some(list)`: cached fixed targets, including a distinct populated empty list.
- Inner `None`: more than 512 fixed targets, so sweeps use the full broad-phase BVH.

Isolated commit `d45b6c0e` adds a default-off `incant-checkpoint` borrowed getter to
both vendored solvers. It neither allocates nor mutates, restores, or installs
anything. The exact prediction value and borrowed ordered entries are exposed.
Original upstream inventories stay unchanged; each `INCANT_CCD_CAPTURE.json`
pins the original/current CCD source hash. Solver algorithms, serde layouts,
Cargo versions, dependency locks and normal features are unchanged. Existing
script runtime identity already hashes the actual vendored source tree, so this
new source delta is included rather than relabeled as the old build.

`CcdState` joins checked integration, collider, body, attachment and arena
witnesses at the completed-step boundary. It accepts only enabled standalone or
fixed-parent colliders, including sensors, and preserves the native ascending
arena-slot order and exact handle generation. A populated list has every eligible
collider exactly once and at most 512 entries; a full-BVH cache requires more
than 512. Finite ordered AABBs and positive-zero unused planar Z lanes are checked.
Nonempty owner mutation journals already refuse in the checked collider context.
A separately supplied empty journal is still a partial witness, not authenticated
pipeline phase provenance.

Cache prediction need not match current integration settings. A parameter-only
change with CCD disabled can leave an earlier cache intact. Native cache-key
comparison uses numerical equality, so replacing current -0 with +0 can retain
stored -0. Scene changes invalidate the cache. Neither situation authorizes
rebuilding or normalizing the stored cache during decoding.

`ICCCD001` has a 60-byte header and 32 bytes per target, capped at 512 targets.
Headers, exact lengths, tags and counts are checked before allocation. Decoded
Vec storage and encoded output each charge the shared aggregate `Budget`;
validation uses bounded scans and no extra allocation. Truncation, trailing
bytes, noncanonical inactive payloads, stale handles, duplicate/reversed lists,
wrong membership and bad AABBs fail through typed errors.

**Shape-derived cache AABB coherence remains unproven by the inert validator.**
The actual native tests compare captured boxes with native shape bounds loosened
by the historical prediction, but that observation is not a hostile geometry
reconstruction algorithm. Complete broad-phase/shape ownership, captured-cache
provenance, source/build authentication and whole-owner constructor ordering are
still required before any live restore. This record does not imply those proofs.

## Focused evidence and remaining work

Source003 passes 21 focused CPU tests in three groups: 19 new cases and two reused
ordered-capture controls. Both actual native dimensions demonstrate negative/zero
dt collision-only behavior, disabled optional passes, sentinel and signed-zero
retention, exact unchanged outer settings, populated-empty vs uninitialized CCD,
cache retention/invalidation, sensors/disabled/fixed/kinematic membership,
512-to-513 fallback, removal and generation reuse. Native CCD serde is observed
to emit `{}` while the getter retains a populated cache; omission is not treated
as equivalence or permission to reset it.

The first focused compile failed only because the test dereferenced the native
velocity getter, which returns a value. Its source and raw failure remain retained;
source002 corrects that test after the failed process was reaped. Source003 adds
only a comment qualification and two actual native zero-base/body-extra controls;
no admission behavior changes. Both have independent source acceptance.

The complete source003 suite passes 156 tests in each deterministic and SIMD8 mode
(23 summary groups, 21 nonempty, zero failed/ignored). Strict Clippy then found
redundant planar constructor closures in two tests. Source004 changes only those
two expressions to direct function pointers. Its focused suites pass 21 cases in
each mode; strict all-target Clippy, wasm32 library compilation, std-only normal
graph, runtime dependency policy, fmt, generated conventions and diff checks pass.
One policy-runner attempt omitted its required root argument and exited2; the
corrected explicit snapshot-root check passes. Both failures remain retained and
are not represented as successful checks. Counts overlap across repeated runs.

The [evidence packet](evidence/physics-integration-config.json) retains exact
source generations, lossless logs/receipts, independent reviews and six copied
final test-binary hashes. The collector rechecks its archive, originals, frozen
source identities, command outcomes and parsed test totals without executing a
solver. All owned processes are terminal and reaped. Original evidence remains at
`~/.codex/incant-evidence/physics-integration-config-001`; final root audit and
publisher integration remain pending.

Broad/narrow owner composition, full pending-journal/phase provenance, exact
shape/cache arithmetic, total allocation/work admission, joints/soft bodies,
trusted construction ordering and complete native continuation remain outstanding.
There is no shipping-game, GPU, phone, performance or exact-save-completion claim.
