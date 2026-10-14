# Release checkpoint24

Checkpoint24 promoted exact main226 (`079ee1aa5e77c4bee352cd792cd43ca3869e2b12`)
through [PR213](https://github.com/snowdamiz/incant/pull/213) on 2026-10-13.
Candidate `669bd7ae87d7d1603f2c01ac39ff0cd3e8403e1f` passed all 23 required
contexts and all eight exact-head push workflows. Independent review verified
unchanged protection and context policy, raw successful queries, preserved
release/previous-candidate/main ancestry and the exact selected tree.
Release `2a9d72eaf804a00b7752015e33ad2dfa087665f4` has that tested tree,
`632fc223feceaae69986daccbec91b872361324d`; merge parents and `Built-by: astra`
were verified afterward. Main228 remained outside the selected release tree.

[Machine-readable evidence](evidence/release-checkpoint-24.json) retains the
freeze, terminal job records, independent review and merge receipt. Its
[offline verifier](evidence/release-checkpoint-24/verify.py) checks all 46 retained
files, exact contexts/workflows, Git identities and lossless hosted logs without
builds or network. The first read-only final jobs request encountered HTTP502;
its successful partial responses and failure record are retained separately from
the complete successful retry. No workflow was rerun or branch changed by that
query retry. Original manifest pending wording records the historical freeze.

The journal replay regression repaired in PR225 passed on macOS, Linux and
Windows. Actual forced-backend renderer suites passed on Apple Paravirtual Metal,
llvmpipe LLVM20.1.2/Mesa25.2.8 Vulkan and Microsoft Basic Render Driver Dx12.
The direct timestamp diagnostic records explicit Unsupported/null measurements
on the Apple adapter while preserving real compute readback; Vulkan and Dx12
record measured positive intervals and unchanged readback controls. These are
capability-specific diagnostic results, not fabricated timestamps or performance
measurements. Default and cooked paths remain separately exercised.

The Linux Vulkan cache lifecycle again completed four fresh processes, each
with 31 render and one compute pipeline: Missing → Supplied → Rejected → Supplied,
with successful storage. It establishes bounded persistence/corruption behavior,
not cache-hit rate or improved performance.

Observed job execution was 78m07s for macOS general checks, 39m14s for Linux
desktop, 67m08s for Windows desktop and 40m36s for Windows optimized conformance.
All four Windows children and their fail-closed aggregate succeeded. These are
actual run durations without a matched speedup claim. All 28 jobs were terminal:
27 succeeded and only the nonrequired candidate Pages deployment was skipped.

Failed checkpoints21–23 remain historical and unpromoted. Their reviewed main
repairs and original failures are preserved; checkpoint23's macOS inherited-lock
cause remains inferred from its controlled reproduction. The successful current
hosted assertion is distinct evidence. Later main227+ work remains outside this
release baseline. No signing, distribution, reference-phone, appearance,
complete-engine or phase gate is approved.
