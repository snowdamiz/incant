# Output-resolution temporal upscaling verification

The independent root review accepted producer
`0452c5f5afc11c99b7c90d111428cc23611c9590` for source and CPU correctness.
The [exact review receipt](evidence/temporal-upscaling-review.json) has SHA-256
`e40651c26d9c39cd980eb58640836d9c7a282dd00374d398fe4006dd02528f31`.
It binds the [unchanged evidence manifest](evidence/temporal-upscaling.json),
24 source inputs, 15 preserved baseline inputs, 17 raw/lossless artifacts, eight
exact copied binaries and seven completed checks. Ordinary tests passed 117 with
98 GPU cases ignored; cooked-feature tests passed 122 with 40 ignored and overlap
the ordinary suite. No GPU test was executed in this producer.

This documentation-only follow-up preserves the original frozen contract and
source bytes, including their contemporaneous review-pending wording. The root
review supersedes that source-review status; numerical GPU execution, Claude
appearance acceptance, cooked artifacts, MetalFX and device budgets remain open.
The 640 MiB + 16 KiB logical ceiling is not a mobile budget or RSS bound.

Before any later delegated numerical run, retain the exact executable permission
bits as well as the copied binary digest, and verify executable admission before
launch. No execution result is inferred from compilation or a copied artifact.
