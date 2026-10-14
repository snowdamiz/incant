# Release checkpoint36 — exact main281 promoted

[PR263](https://github.com/snowdamiz/incant/pull/263) merged on 2026-10-14 after
all 23 required contexts and eight exact-head push workflows succeeded. The
[compact evidence](evidence/release-checkpoint-36.json) retains every required
check, workflow URL, source identity and promotion verification.

- Selected main: `dfac86cb17e9a1dd71d63207d779b3836e7ad6ae`.
- Frozen candidate: `89692f6d60293221803b845e8742f455b68b0c87`.
- Promoted release: `e26c14e595f7a9aaa31225aa8ba81cd1cdc854f1`.
- Shared tested tree: `665aec1330a19be9e32c526f5768e15b36e86129`.

The merge includes both previous release and selected main history. Its body ends
in `Built-by: astra`; the resulting release tree equals the tested candidate and
selected main. Release protection and all 23 required contexts stayed unchanged.
All 28 jobs are terminal: 27 succeeded and only the nonrequired website deployment
was skipped. No missing, pending, cancelled, neutral or skipped required check
counted as success. Windows desktop was the last required job to finish.

This batch incorporates reviewed main253–281 work, including persisted state,
haptics/mobile host code, Core stress and profiling work, prefab authoring,
engine compatibility and project Save UI. The two desktop smoke failures from
checkpoint35 are retained; the reviewed revision repair passes in this successor.
Checkpoints31–35 remain historical failures, not successful qualifications.

Main282 and later work is outside this baseline. Mac performance and rendering
acceptance remain open. Mobile/other-device execution stays deferred under the
director's Mac-only gate decision. This promotion does not publish distributions,
approve a phase gate, or establish a performance or visual pass.
