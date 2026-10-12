# Connected native root motion

Cooked v17 now loads admitted root-only catalogs into native animation owners;
v18 composes the same bounded suffix with compound state transitions. Captured
model fingerprints, selected roots, clip durations and complete channel payloads
bind the cold catalog to the actual model. Cooking and native render admission
share the same model-to-catalog check. Gameplay retains immutable root-only data
through the lean `incant_anim` dependency, without importer/model ownership.

Each fixed step prepares copied animation candidates and independent incoming
and outgoing clock spans before changing physics or ECS state. Exact signed
cycle receipts use the accepted clock expression and checked integer comparisons;
ambiguous or out-of-range multi-cycle steps refuse. Clip, rest, spherical
Blend1d and weighted trees keep their distinct inner interpolation. Compound
interval deltas use the analytic mean of the clamped linear fade over the
accepted interval. Visible pose sampling still uses its endpoint weight.
Parameter changes, select and seek rebase provenance without replaying history.

Application policies are explicit. ExtractOnly publishes observations; Transform
moves a bodyless, velocity-free owner with positive uniform scale. Kinematic3d
uses one velocity-driven solver step and reports actual displacement.
Character3d uses the canonical collision sweep against one retained scene query
frame, with an unparented unit-scale nonsensor sphere or Y capsule and yaw-only
rotation. Dynamic bodies, planar physics and unsupported affine/root channels
refuse. The retained query frame is revoked on success and failure. Every root
candidate and the full scene hierarchy validate before the first solver write;
post-solver failures make the world terminal rather than expose a partial step.

The CPU model consumer transports the selected root, samples into unpublished
storage, strips its translation and rotation to cooked rest while preserving
sampled scale, then applies ordered IK and publishes once. Source-deleted numeric
tests cover this order, late refusal rollback and the allocation-free retained
sampler. This is numerical transport evidence, not rendered-pixel acceptance.

World-save v11 stores exact current/outgoing owner, signed cycle, phase, applying
flag, tick, requested/applied interval deltas and both accumulated transforms.
Restore validates canonical rows, catalog identity and exact playback provenance
before replacing the world. Fresh and in-place QuickJS session restoration
continues active and paused reverse compound fades exactly. Physics checkpoints
and static guest checkpoints remain explicit Unsupported boundaries.

TypeScript exposes owned `animations.rootMotion` bulk columns and atomic
`controlRootMotion` commands. Signed cycles use BigInt64Array and ticks use
BigUint64Array, including values outside the exact Number range. Applying changes
only body movement; stripping remains active. Reset clears observations without
seeking clocks. Invalid caught controls discard the complete gameplay transaction.

The final broad run passes 600 CPU tests in 69 nonempty groups (82 Cargo result
groups), with no failures or ignored cases. Nine-package all-target Clippy,
wasm32 runtime/anim compilation, lean dependency guards, strict SDK TypeScript,
formatting and diff checks pass. The initial broad failure was an obsolete
unknown-version test sentinel for now-supported world11; only that sentinel
changed to12. Independent source and retained evidence reviews accept the native connection
without findings.
[The evidence packet](evidence/native-root-motion-connected.json) retains both
source freezes, review receipts, literal commands and compressed raw logs.

Allocation measurements are scoped: 4096 prepared root-delta evaluations,
1024 retained character refresh/sweep iterations and 1024 accepted staged
strip/IK samples each allocate zero times after admission. A matched 1024-step
whole-world control measures 38,946 allocations with root motion and 38,955
without it; existing hierarchy preparation dominates and scheduling introduces
variation. The first failed whole-world-zero-allocation assertion and the
redundant-preflight correction remain in the packet. No whole-world zero-cost or
reference-device performance claim follows from these CPU measurements.

This commit closes the native/cook/TypeScript connection, not the full feature.
The copied Wasm53–55 contract, actual Wasmtime/wasm2c parity and ordinary public
source-deleted root-motion game export remain ongoing. CUBICSPLINE, selected
scale/matrix tracks, general affine root ownership, physics save continuation,
rendered interpretation and reference-device gates remain unsupported or pending.
