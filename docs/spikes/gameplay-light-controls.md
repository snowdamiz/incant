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

Source controls cover native mixed/final-state rollback and save restoration,
QuickJS caught errors/stale setters/checkpointed phase, strict Wasm batches and
quotas, real Rust SDK wasm2c/Wasmtime parity, native render-input allocation and
rollback, staged asset publication, binding comparisons and candidate history.
The added ignored GPU control compares retained scalar updates with freshly
prepared numerical pixels, including zero crossings and late-refusal retention.
At this source-only stage none of these new Rust controls, nor the GPU control,
has run. There is no actual rendering, performance or phase-gate claim.
