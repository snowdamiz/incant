# Script static data and immutable metadata caches on main259

This composition combines three separately reviewed changes without changing the
trusted scripting boundary: borrowed generated binding data with one bounded
startup conversion; reused immutable attachment snapshots with fresh entity-
lifetime authorization each tick; and reused immutable bulk-query row labels.
Numeric buffers still have fresh, revocable loans. A guest retaining old objects
cannot mutate cached labels or regain a retired entity capability. Cache state is
private to one ScriptHost and is rebuilt by normal fresh-VM restoration.

The frozen source is `40433f104a35e59c2ac1017be6092fcf44c0958f`, based on main259
`64d536d5388349da07ce44f78c24b2ef3663e104`. A temporary Git index applies exactly
static source `3c81bbe6`, attachment source `7a667eae` and row-label source
`3a8a17e2` to main259. Its code tree matches the composed source. Twelve Rust paths
change; the shared `bindings.rs` and `lib.rs` changes compose without manual code
repair. Cargo manifests, dependency identities, bytecode admission and the normal
runtime graph remain unchanged. The only cherry-pick conflicts were independent
STATUS introductions. Root source review independently replayed this composition.

## Actual joined CPU qualification

All 3,885 committed implementation/build inputs were frozen before the joined
checks and verified before and after every command. Both deterministic and SIMD8
modes pass 167 cases each: 144 library, 17 project-binding and six static-binding
cases, with zero failures or ignored tests. Both strict all-target lint runs,
compiler-free native and wasm builds, runtime dependency isolation, formatting,
convention and diff checks pass. Six test executables were copied immediately
from the two actual compiler artifact streams and hashed before reuse.

Sixteen current command receipts and four independent producer/archive replays
pass. The static producer's separately compiled 10,000-row generated Rust control
remains scoped to its original source; this union repeats the unchanged static
and emitter tests, including observable VM property use, but does not relabel
that older external consumer executable. All current children are reaped and the
CPU target is released. There is no GPU, screenshot or shipping performance run
in this qualification.

The packet retains the static emitter's initial standalone-driver E0463 failure
and the attachment cache's malformed JavaScript test-fixture failure through the
unchanged producer archives. These were corrected before the relevant passing
source/driver generations; they are not suppressed or counted as passing tests.

## Original Core measurements and boundaries

The accompanying [original Core reference](mac-core-baseline.md) is separately
accepted historical evidence from `bff83af6`: 46 entities, one controller,
deterministic physics and three actual Mac shipping runs. Its script p95 bounds
are 0.82–0.89 ms; one 5% repeat comparison is inconclusive. It is not the fast-10k
workload, this cache union or a presentation/GPU measurement. Its source and
artifact hashes, machine identity, failed setup attempt and exact measurement
records are unchanged. Local replay also checks the original 4,724 Git input
hashes; portable replay retains those recorded identities when the historical
Git object is unavailable. No private producer ancestry is merged merely to make
that optional historical comparison available.

The original fast-10k shipping compilation remains separate. This composition
does not assert its completion, an equal-input build-time gain, a cache speedup,
a 60 fps/hitch result or a phase gate. The constant-data change still allocates
the bounded owned catalog once at startup. Any later shipping comparison must
hold input/settings/data constant and identify the exact executable sources.

## Evidence replay

[Manifest](evidence/script-caches-main259/manifest.json) binds the frozen input map,
three original code patches, reviewed composition seam, current logs/binaries and
all four unchanged producer evidence families. Run without building or executing:

```sh
python3 docs/spikes/evidence/script-caches-main259/verify.py --verify-only
```

`--originals --local-binaries` additionally verifies all retained current and
producer raw files and local executable copies. The grouped verifier validates
current source identity; producer verifiers preserve their own source generation
and do not infer a current performance run. Source review is accepted; final
integration evidence review and required exact-head PR checks remain pending.
