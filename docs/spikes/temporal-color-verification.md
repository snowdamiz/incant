# Temporal color accumulation verification

The exact [source and CPU packet](evidence/temporal-color-accumulation.json)
was independently [accepted for bounded GPU and visual review](evidence/temporal-color-accumulation-review.json).
Its manifest remains unchanged at SHA-256
`2040592a60773886f605f396121224659a4a86334f636c406672e6e4ff6d084a`.
The review verified all 42 source paths, 15 unchanged baselines, 18 lossless
artifacts, eight copied binaries and seven completed checks. The ordinary suite
passed 110 CPU tests (96 GPU tests ignored); the cooked-feature suite passed
115 overlapping CPU tests (38 GPU tests ignored). Clippy, browser compilation,
normal runtime dependency checks, formatting and generated conventions passed.
The original Clippy failure and its corrected rerun remain in the packet.

The parent motion producer `814717935462e4ee5dbdb1fdae2738d17477f932` also has
[accepted execution of two frozen GPU numeric tests](evidence/retained-motion-numeric-review.json).
Both children exited successfully and were reaped; their upper bound was nine
small frames. The initial wrong-filter preflight refused before execution and
remains historical evidence. Those tests did not print adapter identity or raw
scalar values; their asserted behavior is observed, not a retained raw-value or
appearance comparison. Their execution does not cover temporal accumulation.

The [implementation contract](temporal-color-accumulation.md) remains frozen.
Temporal resolve/retained tests are compiled but unexecuted on GPU. Claude owns
subsequent rendered acceptance. There is no default enablement, temporal
upscaling, MetalFX, reference-device performance or phase-gate claim.
