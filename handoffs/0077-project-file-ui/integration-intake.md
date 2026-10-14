# 0077 — Integration source and ownership intake

The source is frozen and released for root review and final validation. No Claude session or supervisor source writer remains active. This document records the current source ownership and bounded checks; it does not replace root’s final review or claim a phase gate.

Claude’s 16-file UI baseline is preserved byte-exact in `evidence/idle-freeze/` and committed by root as `e824206a` with `Built-by: claude`. Its design, layout, rendered review and original failed/successful evidence remain attributed to Claude Opus 5.5/Max, whose settings the ACP runner verified before prompting.

Root/Astra’s four-file shared bridge metadata correction is separately preserved in `evidence/root-bridge-overlay/`, original commit `806e467f`, replay `a5e862d6` with `Built-by: astra`. Both exact committed file sets match the handoff overlay. Root’s changing producer checkout must not be used as the source.

The separate Astra UI correctness delta against the Claude baseline is `evidence/root-ui-corrections/root-ui-corrections.patch`, with its exact ownership/hash manifest beside it. It includes the shared `outcomeUnknown` consumer and preserved structured errors, post-dispatch UI expectations, and revocable project-bound status announcements. The current 16-file UI fingerprint is `d3cbe982842bf93fc5c1f25226f6ddcff0aa1c1b8277cf43d541dd7ae53fc554`.

The announcement fix’s independently reviewed pre-source, frozen post-source, separate patch and retained first-failure/successor logs are in `evidence/announcement-fix/`. Only its four authorized source/test paths were touched. Focused tests passed (2 files / 34 tests), UI typecheck passed and whitespace checks passed. Root’s earlier r6 whole UI/capture results predate this delta; root runs the final whole-suite, exact-source build/capture and source review. No whole suite, capture, native build or performance measurement was run by the supervisor for this correction.

`result.md` is the historical r4 Claude packet and contains superseded unchanged-bridge/final-state claims. `intake.md` records the earlier stopped-session snapshot. Preserve both as history; use this document and its manifests for current intake, then root’s final evidence for acceptance. r5 fixture screenshots/reports remain preserved; root owns later exact-source rendered validation. No fixture evidence establishes native persistence or native WebKit behavior.

Built-by: astra
