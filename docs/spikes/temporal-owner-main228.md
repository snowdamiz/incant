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
hashes/contracts remain unchanged. Qualification compares actual old native payload bytes after the explicit
recook, rather than inferring equality from hashes of WGSL alone. Default runtime behavior and Cargo feature definitions are unchanged.

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

Current-main source review is independently accepted (`fcfc04e6`). The exact
`a692ef6e` source passes all seventeen final commands: 33 shader-cooker cases,
146 default and 175 overlapping cooked CPU cases (104 GPU cases ignored in
each renderer suite), strict renderer/cooker Clippy, browser and native-window
consumer checks, graph/format/conventions checks and one exact partial-cooked
refusal case. Explicit recooking reproduces all 27 prior temporal/cluster native
payloads byte-for-byte, and all six owner native payloads match the reviewed
producer. Both new owner stages pass actual Metal compile/link and SPIR-V
validation; HLSL is retained without a DXC execution claim.

The first partial-cooked test filter omitted the internal `models::` module and
matched zero tests. The runner's explicit count assertion stopped qualification
instead of accepting it. Original output/controller failure are retained; only
the corrected exact filter and remaining guards ran afterward. No source change
or repetition of passing expensive checks was needed. The final packet retains
134 raw records / 97 payloads, all 3,545 frozen inputs and four immutable current
test binaries; producer evidence retains 213 records / 127 payloads and two
binary identities separately. Independent final integration and checkpoint 24 documentation review is accepted
([root receipt](evidence/temporal-owner-main228-final-root.json), `6029ae16`).
[Manifest](evidence/temporal-owner-main228.json) and
[read-only verifier](evidence/temporal-owner-main228/verify.py). The ignored
GPU test has not run on this composition. There is no color-history consumer,
reactivity repair, default enablement, physical GPU residency bound or phone gate
claim. The full original producer documents and raw evidence remain historical.
