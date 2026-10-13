# Compound and tile checkpoints through the native world

The default-off opaque checkpoint path now retains supported flat 3D compounds
and fixed 2D tile-box compounds. It captures exact ordered children, poses,
cached bounds, native BVH storage and composite contact workspaces. Restore
constructs fresh owners under one cumulative allocation permit; it does not
rebuild compound acceleration or clone a trusted physical owner.

The inert geometry format keeps primitive/declaration-only v1 bytes unchanged.
Its v2 extension joins bounded child bounds and BVH records to the same immutable
geometry identity. The native layer separately checks actual shape-derived
bounds and reproduces native two-pass mass arithmetic without temporary storage.
Metadata visits are charged before scanning; child walks are separately charged
before native checks. Refusal retains previously consumed budget. These are
logical construction/validation bounds, not whole-step CPU-time promises.

Deterministic workspaces retain actual insertion order. SIMD8 workspaces retain
canonical logical keys; physical hash buckets, capacity and tombstones are not
claimed equivalent. Paired native and query continuations exercise the actual
ordered BVH traversal and contact bookkeeping.

Outer physics checkpoints preserve stable IDs, ordered configurations, overlap
events and pending inputs. Their joins compare exact native compound parts and
tile rectangles against the retained declarations. ECS restore requires the
actual planar owner for colliding tile maps; component declarations cannot
silently reconstruct a missing solver. Fresh whole-world candidates remain
separate until all owners and bindings validate.

## Observed CPU qualification

On source `19d42811`, each numerical mode passed 538 tests across the snapshot,
2D physics, 3D physics and ECS runtime packages, with no failures or ignored
cases. Each run retains 67 immediately copied executables. The lower compound
controls capture warm contacts, compare 72 continuation steps, remove/reuse
handles, query rays and restore again mid-run. Outer controls additionally
exercise sleep/wake, triggers, corrupt late joins and pending ECS values.

The candidate then composed actual reviewed main250 (`646aeb4f`). All previously
tested physics/schema/ECS input bytes remain exact; eleven prior script/game
inputs change to their independently reviewed main versions, with their new
checkpoint modules also included in the current freeze. Current script/game
library and integration suites pass 238 tests per mode (39 game and 199 script),
with 17 immediate executable copies each. This command excludes doc tests;
producer counts remain in their own packets.

Eighteen current checks pass: strict all-target Clippy with the feature in both
modes and ordinary physics/ECS Clippy, both wasm library builds, selected graph
audits, ordinary feature exclusion, manifest/runtime-dependency guards, tooling
tests, formatting and conventions. An extra vendor formatting invocation first
used Rust 2024 for the Rust 2021 Parry sources; that failed command is retained.
The corrected checks use each vendor manifest's edition and pass without source
changes. The earlier native observer failure from decoding the private envelope
as inert payload is also retained; it is not passing coverage.

Separate accepted packets retain the full
[Parry unit qualification](parry-compound-checkpoints.md) (201 tests per mode)
and [Rapier unit qualification](rapier-compound-regressions.md) (336 per mode).
These are separately compiled native suites, not extra cases in the 538 count.
Original producer commits are retained in ancestry so their source identities
remain resolvable after publication. The ancestry-only merges change no tree.

The [union manifest](evidence/native-compound-checkpoints/manifest.json) retains
lossless command/source records, immediate executable identities, and the
independent source reviews. Verify without native execution:

```sh
python3 tools/probes/native-compound-checkpoints/collect.py --verify-only
```

On this producer machine, `--originals --local-binaries` also checks retained
original records and all 174 immediately copied binaries, including historical
generations. Independent final union evidence review is accepted (e235399e). Required fast CI
remains separate from these local results.

## Current main251 composition

The integration preserves the complete producer ancestry and all 79 owned source
files. Of its 3,155 frozen build inputs, only the authoring source-store test
changes: main251's independently reviewed exact-child lock isolation. No runtime,
native, manifest, lock or build input changes. All three selected dependency
graphs match after checkout-path normalization, with eight read-only checks
passing. No compiler or native test was rerun for this disjoint join.

The [current composition verifier](evidence/native-compound-main251/verify.py)
checks the actual new test bytes against main251, then replays the original
immutable assertions with only that explicitly bound historical test read.
Every other source, log and executable remains checked unchanged. The original
538/238 per-mode executions retain their source generations. Final current-main
composition review is independently accepted; publication awaits the required
exact-head fast check. Public physics bytes remain refused.

## Remaining boundary

This is an opt-in in-memory capability. Public persisted physics bytes remain
refused. Nested/custom/mesh shapes, nondefault compound flags, unsupported joint,
soft-body and character-cache state retain their explicit refusals. Build hashes
are compatibility metadata; cross-build or cross-mode reuse is not claimed.
The tests use the CI profile on this Mac. No GPU appearance, exported-player,
phone, shipping-profile performance or phase-gate result follows from them.
