# Integer motion-owner attachment (CPU implementation, review pending)

This additive opt-in transports the previously accepted placement ID and change
bit in an R32Uint attachment from the same opaque/MASK fragment and depth test as
motion. It is a substrate, not a history consumer or a visual-quality repair.
The original Motion/Depth WGSL and source identities are unchanged. MotionOwner
has a separate two-entry source and six native artifacts. Its stride-80 previous
row uses Float32 validity at location 11/offset 64 and Uint32 owner at location
12/offset 68; the flat uint varying and uint MRT never convert IDs through floats.
MASK sampling/derivative ordering and motion arithmetic remain the same. Invalid
previous motion still writes the current owner. Clear/excluded surfaces write 0.

Call `Renderer::enable_motion_vectors`, then `enable_motion_owner`, then enable
retained motion history and `set_motion_owner_attachment(renderer, true)`. The
startup method builds four additional pipelines only when requested. A changed
attachment setting creates a new motion owner and drops all color/native
histories atomically after instance admission. Repeating the same setting is a
no-op. `enable_temporal_upscaling` preserves the explicit attachment setting while
resetting histories, so owner-only and portable-upscaling owners are usable.
Fixed temporal and MetalFX enabling explicitly refuses with OwnerCombination
while attached: this slice does not admit their additional retained caches.
There is no attachment or additional startup pipeline on the default path.

Targets retain an optional source-size owner view with a 4P per-allocation cap,
P=4,194,304. Owner-only and owner+portable paths preflight actual accepted sizes
before motion, owner, output, or overlay allocation. Equal extents reuse the
exact existing Targets/Output/Overlay Arc; differing extents count both. Target
reuse also requires the same attachment setting. Each component has its own
old+candidate sub-cap and the aggregate remains 671,105,024 bytes, including two
64MiB maximum instance payloads and 16KiB packet metadata. The owner target is not
an increase to that ceiling. Default owner-disabled admission remains unchanged.
A 2048-square steady owner+upscale frame and a 16-to-15 DRS change fit; a full-size
2048-square to 2048x1920 resize refuses. Owner reset then retry discards accepted
history; it does not claim GPU work has finished.

The retained submission method accepts `&mut self`, owns one synchronous prepare/
encode/submit candidate, and publishes only after submission. Failed candidates
drop their views/Arcs; accepted resources are not replaced. MotionDraw holds the
same candidate Arc. Upcale Pending/Draw clone the exact prepared output/overlay
views; successful accept transfers those Arcs, without keeping a third cache.
**These are logical owned payload bounds only.** Already queued GPU work and
wgpu's deferred resource destruction may retain older textures. No queue wait,
completion drain, RSS bound, phone budget or physical-residency claim is added.

No color consumer changes, meta histories, native owner pack, style defaults or
reactivity tuning are included. GPU and appearance gates remain open. The
prescription's six on/off frame comparisons require twelve render submissions,
plus explicitly counted readback submissions and setup; a later Claude grant
must bind the actual runner count rather than label them six total frames.

Focused qualification retains the original Naga integer-IO failure: vertex uint
inputs normalize to Flat with no sampling qualifier, while the explicitly flat,
first inter-stage owner must normalize to First. The corrected contract checks
those separately. Five contract tests and two six-stage native emission tests
pass. The new source-deleted fixture tests exact MASK bytes and analytic skinned
positions on CPU. Its ignored device test has twelve renders at 100%/50% source
scale, twelve batched readback submissions, no image output, and a 32x32 maximum
extent. It compares every owner texel against analytic coverage and the accepted
ID map, including clear/BLEND zero, MASK discard, mirrored packing changes, first/
unchanged/deformed joint state, failed-attempt rollback and exact target reuse.
Motion, depth and color bytes must match owner-disabled frames. It is compiled
only; no GPU result or appearance acceptance is claimed. Original fixture compile
errors (unavailable image crate and SceneInput clone) are retained; the fixture
now embeds a fixed PNG and independently loads each scale's source-free assets.
