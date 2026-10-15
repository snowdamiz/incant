# Release checkpoint40 — exact main309 promoted

[PR311](https://github.com/snowdamiz/incant/pull/311) merged on 2026-10-15 UTC
after all 23 required contexts and eight exact-candidate push workflows succeeded.
The [promotion record](evidence/release-checkpoint-40.json) retains the successful
checks, workflow identities, review and post-merge verification.

- Selected main: `d8dea4ed7543961eb31f57f52787268914b1391c`.
- Frozen candidate: `cf55aa691122c570819aaaa58cbd4f0805331df0`.
- Promoted release: `c0276c7b31fdb013bcfb53b09e11f8baa284fa9f`.
- Shared tested tree: `9b6ee583cb87cea192404bab21f1c6c6e1a4e4ec`.

The candidate stayed frozen while main development continued. Every required
result was actual success; missing, skipped, pending and neutral results did not
pass. The general check ran for 91m48s. That is one observed job duration, not a
matched CI speed comparison. The skipped website deployment is not a required
check; candidate validation does not publish the website.

This batch adds typed audio-overrun handling, the complete native presentation
replay, the native 2D sample proof, timestamp rejection and the default-off
compilation observer to the previous release baseline. Later logical-window,
locale, subtree-duplication, host-stage and script-budget work is outside this
tested tree. Private reconstruction experiments also remain excluded.

The verified release tree equals the tested tree and its merge commit ends in
`Built-by: astra`. This is an integration baseline. Rendering quality, script
time, full rendered-game performance and phase gates remain open. No distribution
was signed or published.
