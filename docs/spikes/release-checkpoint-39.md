# Release checkpoint39 — exact main302 promoted

[PR299](https://github.com/snowdamiz/incant/pull/299) merged on 2026-10-15 UTC after all 23 required contexts and eight exact-candidate workflows succeeded. The [promotion record](evidence/release-checkpoint-39.json) retains the successful checks, workflow identities, review and post-merge verification.

- Selected main: `6dd277685096595ea4f9b7c5a8834dc1c8f28544`.
- Frozen candidate: `155ef905ca54db6e1f8bce9b29fa8c51ea26162c`.
- Promoted release: `aa481bea41434dd938ab560144321a59e14f5da2`.
- Shared tested tree: `456feb1a6e01bad67a53bc29b94d08e3e8dc69ce`.

The first attempt's routing check failure and cancelled work remain historical. The narrow routing assertion repair was reviewed and locally checked on main302; the same candidate branch then advanced without rewriting history. Its second head stayed frozen while main development continued. Every required result was actual success; missing, skipped, pending and neutral results did not pass. The final general check took 94m48s from its recorded start to completion. This is one observed duration, not a matched CI speed comparison.

This batch includes ordinary editor 4x antialiasing, native-window replay and timing foundations, compatible animation refresh, and the rejected repeated-navigation-start optimization hypothesis. It does not include later main303+ audio-overrun handling, complete presentation-observer replay, the 2D native proof, timestamp-decoder correction or compilation observer. Private reconstruction and localized-window experiments remain outside this tested release tree.

The verified release tree equals the tested tree and its merge commit ends in `Built-by: astra`. This is an integration baseline, not approval of rendering quality, performance or a phase gate. The script-time requirement remains unmet and may be revisited through distinct bounded investigations. No distribution was signed or published.
