# 0065 — Core Sample live HUD and restart presentation

Use Claude Opus 5.5 over ACP, explicitly verified Max thinking for every new or
resumed session. Read CLAUDE.md. You own appearance/layout/rendered-pixel review;
Astra owns runtime logic and correctness. Preserve the director's preference for
clean professional presentation, readable spacing and restrained ornamentation.

## Current behavior and accepted source

Start at a9e26999e0763de7cbd625930595be8a36f3f460. Read
samples/core/PRESENTATION.md, docs/spikes/core-authored-runtime-bindings.md and
its evidence manifest. Existing Claude-authored Core art, HUD, strings, fonts and
layout are unchanged. The actual gameplay controller now drives the existing
health/pickup/objective/result labels and the authored music plus pickup, damage,
win and loss voices. R and gamepad Start trigger a complete session restart.

Independent nonvisual review accepted14 native positive processes and10 actual
source-deleted ordinary shipping processes, including complete win/loss routes,
four locales (English, Arabic, Simplified Chinese and pseudo),4 cue-isolation
controls and4 full session restarts matching fresh state/HUD/audio exactly.
Those results are not rendered proof. The visible controls hint currently omits
restart, so the action is undiscoverable. Add a concise, coherent restart hint
through the existing localized assets and public authoring path. Do not expose
technical proof text, UUIDs or implementation phases in the game UI.

Review the actual live HUD through real controller routes, not a staged label
replacement. Verify readable changes at pickups/damage, locked exit, win/loss,
and return to the initial HUD after R/Start. Include an Arabic and CJK state and
pseudo-localization. Use the numerical routes and fixture IDs; no gameplay math,
collision, pathfinding, controls, engine code or observation-oracle changes.
Small warranted HUD/typography/layout/copy corrections are yours to implement;
keep source/asset/command mutations explicit and reversible through incant_cmd.
If a runtime defect is found, report its reproduction to Astra. A packet-only
capture adapter may read the public runtime snapshot and use existing render
APIs, but must not substitute gameplay or alter its state to stage the result.

Keep old0060 evidence honest: its historical helper hash discrepancy and old
false locked-route result are preserved; new acceptance comes only from this
packet's exact execution. Current locked route is corrected and independently
verified alive with3of4 pickups at the exit. New evidence must not retroactively
relabel old records. Read samples/core/playtest.py and verify_bindings.py for
actual current routes and accepted boundaries before choosing captures.

## Ownership and resource bounds

0064 owns the GPU and target. Do not build, use GPU, open a native window or
capture until Astra creates
/Users/andreyyurlov/.codex/incant-evidence/0065/review-ready.json naming this packet
and releasing the target from0064. Only Astra grants this; if absent, preserve
preparation and stop rather than poll. After that grant, exclusively use
/Users/andreyyurlov/.codex/incant-core-presentation-target, ./tools/cargo,
--locked, ci profile and at most2jobs. Reuse compatible artifacts. Never write
another agent's target/worktree. Renderer branch experiments are out of scope.

Evidence root: /Users/andreyyurlov/.codex/incant-evidence/0065. Use offscreen
engine captures only, at most24 actual retained frames at960x540 maximum. No
desktop/browser capture, native windows, device audio, credentials or accounts.
Run serial GPU groups with exclusive lock, bounded timeouts (5minutes per route,
2minutes per capture), and reap all children. Do not acquire or extend a lease
without Astra. Freeze source/helper/binary identities and command/exit ledgers;
release GPU and Cargo target explicitly before the final CPU-only report work.
If full numeric routes and all desired frames exceed these bounds, prioritize
live readable transitions plus the four locale/result/reset states and explain
the remaining scope; never silently extend the limits.

## Deliverables

Return result.md, exact source/asset differences, a compact unedited capture
selection and reproducible helpers/evidence under this packet. Record model,
effort, OS/adapter/backend, command exits, scope and all remaining limits. Use
samples/core existing validators for changed authored content and actual runtime
rechecks if IDs/binding structure change. No image parity claim replaces your
visual critique. Root reviews numerical/correctness evidence separately.
Commit with Built-by: claude. No pushes, PRs, merges, CI changes or phase approvals.
This is Mac presentation review; minimum-OS/device/performance and other backend
gates remain open.
