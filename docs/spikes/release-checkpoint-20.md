# Release checkpoint20

Checkpoint20 promoted selected main205 (`7bfeecfdfaa4a52ec198d1caee0fcb4ff9399fd9`)
through [PR174](https://github.com/snowdamiz/incant/pull/174) on 2026-10-13 at
08:06:59 UTC. Candidate `99ba87ed82b4009680eaaa5089ffbd76f6bedc58` passed all
23 required contexts and all eight exact-head push workflows. Independent review
verified the unchanged policy, raw queries, ancestry and exact selected tree before
merge. Release `065aac69289bf9aab067dbd2148624904a860d5e` has the same tested tree
`71e55928908092313a4bcaf4d799cc09f976726c`; its merge parents and `Built-by: astra`
trailer were verified afterward. Main remained at reviewed main209 during promotion.

[Machine-readable evidence](evidence/release-checkpoint-20.json) retains the freeze,
terminal results, independent review, merge receipt, all job identities and full
lossless logs for macOS general, Linux/Windows desktop and Windows optimized
conformance. [Offline verification](evidence/release-checkpoint-20/verify.py) checks
record hashes, exact successful contexts/workflows and Git trees without builds or
network. The original manifest's pending wording is the historical freeze state.

Default and opt-in cooked renderer suites passed on logged hosted adapters:
Apple Paravirtual Metal, llvmpipe LLVM20.1.2/Mesa25.2.8 Vulkan and Microsoft Basic
Render Driver Dx12. The Linux cache test passed four fresh processes, each with
31 render plus one compute pipeline: Missing → Supplied → Rejected → Supplied,
with storage succeeding each time. This establishes cache lifecycle and corruption
handling on that adapter, not a cache-hit or performance result. macOS also passed
the invisible AppKit retirement probe: 768 contexts, 256 stale-session refusals and
active-owner destruction, without GPU submission or injected input.

Observed execution durations were 75m19s for macOS general checks, 43m37s for Linux
desktop, 67m58s for Windows desktop, and 89m30s for Windows optimized conformance.
The latter succeeded near its 90-minute limit. These are run timings, not a matched
speedup comparison. All four Windows conformance children and their fail-closed
aggregate succeeded. Of 28 jobs, 27 succeeded; only the nonrequired Pages deployment
was intentionally skipped by candidate policy.

Failed checkpoints15–19 remain historical and unpromoted. The exact checkpoint19
terminal record is retained here; existing earlier packets and repository history
remain unchanged. Main206–209 work lies outside this release baseline. No signing,
distribution, reference-phone, appearance, complete-engine or phase gate is approved.
