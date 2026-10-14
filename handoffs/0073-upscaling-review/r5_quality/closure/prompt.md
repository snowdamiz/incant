# Current priority: execute granted R5 numeric + quality protocol and review its pixels

This replaces the prior source-edit task for this turn. Required model: Claude Opus 5.5; thinking explicitly Max. Auto-memory is disabled by the supervisor. Do not read or write external memory, preferences, settings, or credentials. Keep all historical failures unchanged.

Root accepted protocol packet `792bf05694831d06ad73405fbf54e8f5613d06ca`; root review SHA93b1f7dc6d8b5b794e600bb705254c92219401edc3fea5cf66837b6095ae1889. A fresh exclusive GPU grant exists at `/Users/andreyyurlov/.codex/incant-evidence/0073-r5-numeric-quality-gpu-001/grant.json` SHAe981286ac185d5d4ea5d58b99de4f4c9555bc207fcae6f1f217e609c075a98f8. It binds exact runner f2ba4c96, plan47133884, all three frozen binaries and actual predecessor release7d642244. No other GPU work is permitted.

Immediately read `r5_quality/execution/README.md` and the final plan, then execute EXACTLY ONCE:

```
python3 -I -B /Users/andreyyurlov/.codex/worktrees/native-agent-bridge/incant/.worktrees/0073-upscaling-review/handoffs/0073-upscaling-review/r5_quality/execution/runner.py run /Users/andreyyurlov/.codex/worktrees/native-agent-bridge/incant/.worktrees/0073-upscaling-review/handoffs/0073-upscaling-review/r5_quality/execution/plan-ready.json
```

Do not invoke a binary yourself. The runner must execute all four strict numeric children before any of the same five quality groups. Ceiling: nine children, 840 queue submissions, 442 maps, 15 PNGs, max960x540 (numeric16x16), 90 seconds per child, no builds/retries/extra probes/captures. Stop at the first process or contract failure; preserve evidence and review only outputs actually produced. Do not repair runner/test/shader, rerun anything, change schedules/thresholds/scenes, or run any Rust compiler. The exact granted runner may write only its designated external evidence directory; you may read those retained outputs. All other authored output goes to this packet worktree.

After release, inspect EVERY produced PNG using your image-reading tool. Compare the actual R5 pictures to the retained R4 result/pixels under `r4_quality/execution-result.md` and `/Users/andreyyurlov/.codex/incant-evidence/0073-r4-quality-gpu-001`. Do not substitute hashes or numeric success for visual judgment. Judge stationary OWN detail retention, ordinary UP stationary moire/thin lines, moving UP/OWN current-only texture/silhouette aliasing, first4/16 +16/16 +postcut transitions, and output-overlay clarity. Preserve the exact old C7/C9/P1 definition/coverage qualifications. Report current observations candidly without moving thresholds or relabelling the old gates. If a prior identical PNG was already read, qualify that explicitly; prefer inspecting all current outputs.

R5 is narrowly scoped: history-clamp-displacement reaction, empirical r/16 current motion slack, and -1.5 internal LOD floor. Neither empirical constant is a same-surface/quantization/sampling proof. Thin clear-boundary geometry and low-scale fresh/postcut reconstruction were not claimed fixed. The new images decide their practical outcome. No new repair or exploratory modeling is authorized in this turn.

Allowed new result paths: `handoffs/0073-upscaling-review/r5_quality/execution-result.md` and `.json`, plus a short updated `handoffs/0073-upscaling-review/result.md` pointer. Include exact model/Max, executed children and numerical results, actual image-read inventory, visual verdict by area and overall, release path/hash, limitations, and process failures if any. Commit ONLY those result files with `Built-by: claude`. Do not stage the already dirty brief or protected untracked source inputs. Do not change any existing committed packet/source/fixture file.

The supervisor enforces 15 minutes by both wall and monotonic clocks. Work concretely: one exact run, pixel review, concise result, scoped commit. If incomplete at the bound, retain and report partial evidence; no restart is authorized.
