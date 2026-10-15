# Gameplay punctual-light controls

The disposable native world now stages color and intensity controls for existing
directional, point and spot lights. TypeScript exposes `setLightColor(id, rgb)`
and `setLightIntensity(id, value)`; the Rust guest SDK exposes
`Tick::light_commands` with caller-owned scratch. Linear RGB channels must each
be finite in `[0, 1]`; intensity must be finite in `[0, 1_000_000]`, using lux for
directional and candela for point/spot lights. Environment radiance, light kind,
range, cones and shadow configuration remain structural bindings.

Controls compose in submission order against the batch's final `SetSpatial` and
entity state. A final nonlight, removed declaration or despawn refuses the entire
batch, including otherwise valid scalar/numeric prefixes. Scalar-only and mixed
component-only batches stage touched values rather than clone the native world
or parent graph, and leave binding/topology generations unchanged. The guest
failure latch rejects caught malformed controls together with numeric writes;
expired capabilities remain refused. Native commands retain their existing
65,536-command frame bound; Wasm shares its existing 1,024-command transaction
bound and transfer/call limits. The added import is append-only opcode 62 in
`incant_light_v1`; existing imports and layouts are unchanged.

Current values live in the existing `CookedSpatial`, so native saves retain them
without a format extension. A checkpointed day/night phase should use the
existing opted-in TypeScript state contract, `persistence: {version: 1}`, plus
`context.state`; ordinary saves do not snapshot arbitrary module globals or a
Wasm heap. There is no project document or authoring command call in the tick.

The native render adapter copies light scalars through its preallocated numeric
undo storage. Consumer refusal or unwind restores its last accepted values, and
a prepared asset refresh copies the most recent light values at publication.
The retained GPU consumer checks structural light bindings separately from
numeric values. Changed scalars cut the proposed motion/temporal provenance only
when that frame is accepted; a refused candidate does not invalidate accepted
history. An unchanged following frame resumes history normally.

## Allocation and qualification limits

This change does not make complete renderer frames allocation-free. The existing
directional shadow planner filters zero-intensity or all-zero-color lights, and
the shadow atlas cache is keyed by exact active count. Crossing zero can create
a different atlas; no new shadow capacity policy is introduced. More than four
simultaneously active shadowed directional lights still refuses the rendered
candidate explicitly. CPU render-input controls observe its scalar copy/undo
allocation count separately from that GPU resource behavior.

Qualification passed 32 distinct Rust CPU controls, 20 Python controls, strict
TypeScript checking, generated SDK verification and two scoped strict Clippy
commands. The CPU controls exercise native mixed/final-state rollback and save
restoration, QuickJS caught errors/stale setters/checkpointed phase, strict Wasm
batches and quotas, real Rust SDK execution through both wasm2c and Wasmtime,
native render-input allocation/rollback, staged asset publication, structural
binding comparisons and candidate history. Four real Rust guests cover a valid
batch and caught scratch, range and owner errors. The production source is
`730928381f86f751dc5d8ff0b69a39245d0d300f`; final qualification source
`11c1856dbcca0078c3fc8bb080e8a40d4aa37967` adds only fixture corrections.

One actual numerical Metal control passed on Apple M5 Pro. It compares retained
frames with freshly prepared pixels for directional, point and spot scalar
updates, including directional zero crossings. A zero-sized candidate and a
changed shadow binding retain accepted pixels/history; accepted scalar changes
start a new motion epoch and unchanged following frames resume history. The
copied test binary has only `default` and `physics-deterministic` features, so
the fixture's cooked-mode refusal branch cannot bypass these assertions. Its
owned process group exited successfully and cleanup was confirmed. This is a
typed renderer fixture; the CPU controls establish gameplay/VM/SDK behavior
separately. No integrated day/night app, appearance, performance, full-renderer
allocation freedom or phase-gate acceptance is claimed. Native MetalFX is
compile/lint-qualified here, without executing a vendor temporal pass.

Three failed qualification attempts remain in the packet. They exposed fixture
assumptions about Bevy table order, the last scene node being a light, and native
32-hex labels versus TypeScript ULID spelling. Successors select complete stable
owner identities and preserve all scalar, rollback and allocation assertions;
no production change was needed. Across all CPU attempts, 17 managed process
groups were cleaned up, cumulative sampled/final logical target growth was
1,269,138,025 bytes (under the 4 GiB allowance), and the CPU target was released.
The later GPU child was cleaned up separately. These are local correctness
checks, not shipping-profile performance measurements.

The compact [manifest](evidence/gameplay-light-controls/manifest.json) retains
source snapshots, original failures, actual commands/logs, guest provenance,
review and executable hashes. Replay without starting a compiler or device:

```sh
python3 docs/spikes/evidence/gameplay-light-controls/verify.py
```

`--originals --binaries --git-source 11c1856dbcca0078c3fc8bb080e8a40d4aa37967`
also checks retained external evidence and the producing Git source.
`--source-root .` checks exact qualified code inputs; later source compositions
must establish their own relevance rather than relabel these executions.

## Main341 integration

The producing packet remains unchanged. Of its 65 code inputs, 63 are identical
in the main341 composition. Only `incant_script/Cargo.toml` and `src/lib.rs` differ:
the reviewed main340 change makes the separately prepared stress diagnostic
explicitly opt-in. Light modules and all runtime, SDK and renderer inputs remain
identical. The strict live-source replay correctly refused this drift; historical
source/artifact replay still passes. The integrated script crate then passed all
three light controls and strict all-target Clippy. Both managed processes exited
successfully and were reaped. [Composition and local checks](evidence/gameplay-light-controls/integration-main341.json).
