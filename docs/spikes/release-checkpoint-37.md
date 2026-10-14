# Release checkpoint37 — exact main285 promoted

[PR286](https://github.com/snowdamiz/incant/pull/286) merged on 2026-10-14 after
all 23 required contexts and eight frozen-candidate push workflows succeeded.
The [evidence](evidence/release-checkpoint-37.json) retains checks, workflow URLs,
source identities, the integration review and promotion verification.

- Selected main: `1eca995cb36d9f9cada732dce7d52447569e63ac`.
- Frozen candidate: `e7f3d2c1e7118bfbff23402961daa98daa689fc8`.
- Promoted release: `1034930898bb2b8a7bfa08f9d426ba71a907bf29`.
- Shared tested tree: `ebd3daeb1c49eaa289cb083e5656bd71dbdc1dcb`.

The merge preserves previous release and candidate history, ends with
`Built-by: astra`, and has exactly the tested candidate/main tree. Protection and
all required contexts are unchanged. All 28 jobs finished: 27 succeeded; only the
nonrequired website deployment was skipped. No incomplete or skipped required
result counted as success.

The first Android job failed while downloading the SDK ZIP. That original failure
is retained. One retry of the failed job succeeded on the same frozen candidate;
the other seven workflows ran once. Windows desktop was the last required check
to finish. The candidate stayed frozen throughout.

This batch includes existing-texture refresh, the default-off correlated script
diagnostic, the sparse development checkout and evidence rejecting the ordered
query/navigation-cache experiments. It does not adopt either optimization. The
later native-window texture probe and private pointer, reconstruction and MetalFX
work are outside this baseline.

Promotion establishes an integration baseline. Rendering acceptance, Mac shipping
performance and phase gates remain open; mobile/other-device execution remains
deferred. No distributions are published or phase gates approved.
