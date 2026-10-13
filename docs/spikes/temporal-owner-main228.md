# Integer owner attachment on main228

This curated integration adds the independently reviewed R32Uint motion-owner
attachment to main228. It selects the source delta from `613ee59a` and retains
producer CPU evidence from `4c431087`; it does not import their private upscaler
or MetalFX ancestry. The existing placement provenance foundation is already
main226. Checkpoint24 documentation in the same branch records a separate
completed release promotion and does not validate this new renderer source.

## Current source boundary

The exact producer owner WGSL, integer vertex ABI, four startup pipelines and
owner/Motion/Depth drawing behavior are preserved. Location 12 reads Uint32 at
offset 68 of the existing 80-byte instance row. The second MRT is R32Uint, with
no blending. Clear and excluded surfaces write zero. The original Motion/Depth,
fixed resolve/copy, bloom, output and cluster shader sources are unchanged.
The existing four-program temporal cooker adds only MotionOwner; all old source
hashes/contracts remain unchanged. Qualification will compare actual old native
payload bytes after the explicit recook, rather than infer equality from hashes
of WGSL alone. Default runtime behavior and Cargo feature definitions are unchanged.

Main has no upscaler or MetalFX history owners. Admission therefore accounts only
for the actual accepted and candidate motion/depth plus integer target sets. It
preserves the producer 671,105,024-byte aggregate ceiling, the 4,194,304-pixel cap,
24P motion/depth and 4P owner per-allocation limits, two maximum 64MiB instance
sets and 16KiB packet allowance. Same extents reuse the exact retained Arc;
distinct extents count both even if their pixel counts match. Fixed temporal
history explicitly refuses while owner attachment is enabled. Toggling the
attachment allocates/adopts a fresh motion owner only after successful admission,
then resets all present accepted color history. Idempotent toggles preserve it.
No absent private cache is assumed, no allocation ceiling is raised, and queued
GPU residency is not claimed to fit this logical owner bound.

The producer device test remains twelve renders plus twelve batched readback
submissions, maximum32x32, with no image output. This composition adds only
non-submitting assertions for idempotent toggles and precise fixed-history
refusal/unchanged accepted sequence. It is compiled by the CPU suite; no current
GPU execution is inferred from compilation.

## Evidence and limits

The producer's CPU/offline packet remains byte-exact under
`evidence/temporal-owner-attachment.json`; its historical pending language is
superseded only by the separate retained review receipts. Root accepted producer
CPU evidence `d232a224` and the exact original 0073 Metal numerical execution
`ad7cde4b`. The latter ran twelve render and twelve readback submissions using
binary `fc3cd756`; compiling native-MetalFX features did not enable that backend
in the test. It proves only the tested attachment substrate, not appearance,
history consumption, new native-backend behavior or performance.

Current-main source review is independently accepted (`fcfc04e6`). Scoped
CPU/recook checks are pending. The ignored
GPU test has not run on this composition. There is no color-history consumer,
reactivity repair, default enablement, physical GPU residency bound or phone gate
claim. The full original producer documents and raw evidence remain historical.
