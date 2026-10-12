# Review the connected native game UI in a real desktop window

Use Claude Opus 5.5 through ACP with explicitly verified Max thinking. Read
AGENTS.md, CLAUDE.md, PLAN.md and the accepted 0056 result. This packet follows
0056 offscreen rendering and the independently reviewed managed desktop IME
implementation. Astra owns native input/session correctness; Claude owns
appearance and every pixel review/capture.

## Scope and acceptance

Build this exact committed source, then review the actual ordinary native window
player with cooked game UI and packaged fonts. The GPU consumer now opts into
screen-space UI; the window host already configures the native UI owner and
routes keyboard, pointer and opt-in managed IME receipts. Astra has added exact
reserved-frame logical sizing and UI resource statistics. Test real drawing and
input together, rather than replaying an offscreen fixture alone.

Author a fresh small mixed scene through public imports and incant_cmd. Reuse
0056's fixture authoring script as a reference, but create new paths, identities,
packages, binaries and evidence. Include labels, enabled/disabled buttons and
Latin, Arabic/RTL and CJK text fields over a moving world. Exercise actual pointer
focus/activation, keyboard Tab/shift-Tab, editing, resizing, blur/refocus and a
short lifecycle run through the normal composed player. Capture and inspect the
actual window yourself. State exactly which native event mechanisms were used.
A synthetic preedit receipt is not a live input-method result. If a live system
IME is unavailable, record the untested part and do not ask the director to
change their system setup during this packet.

Review any mistakes in positioning, clipping, selection/caret presentation and
focus state through the actual consumer. You may correct renderer appearance,
shader/quad presentation, theme and their numerical tests. Keep changes restrained
and professional. No decorative glow, extra explanatory chrome or styling unrelated
to game UI. Preserve existing numerical regressions and resource bounds.

The 0056 limitations are still explicit: text rasterization at logical size is
soft at 2x, hover and content alignment are absent, precise native candidate-window
caret placement is pending. Do not claim those complete. For high-density text
or canonical caret geometry, propose a precise read-only/scale contract in the
result; root will coordinate the CPU binding changes. This packet may implement
a narrowly useful read-only caret geometry accessor in the renderer with numeric
tests, but must not infer ownership or mutate the native UI/input semantics.

## Ownership and performance

Do not edit incant_runtime, incant_script, incant_game_runtime, incant_ui or
incant_render_game: the UI save/restore agent owns those files concurrently. Do
not edit the player/window platform loop, input queues, vendored Tao or IME code;
report concrete failures to root with source location and reproduction. Do not
change checkpoint formats, physics queries, public Wasm registries or other
lanes. Review/source fixes in incant_render and incant_render_gpu_host are yours.

Use the canonical retained UiInput, shaped text, atlas and consumer. No second
font discovery/shaping path, project JSON in frame updates, per-frame allocations
of unchanged GPU buffers, readbacks in the normal player, or authoring mutation
bypass. CPU/GPU fixture counts are resource evidence, not shipping/phone timing.
No complete Game UI or Phase 1 claim.

## Commands, identity and resources

Use ./tools/cargo with --locked, bounded -j2 and the explicit ci profile for local
checks. Build the fresh native-window player and relevant harnesses. Record the
exact source/tree, Rust/dependency lock, binary/profile, fixture/package/font
hashes, actual native process PIDs, OS/device/backend and screenshots. Distinguish
CPU input tests, offscreen tests and actual native-window execution. Do not use
old binaries from 0056 or overwrite any older packet evidence.

0056 is terminal, process47937 reaped and its /tmp/incant0056/released marker
remains final. This NEW packet has exclusive GPU/native review rights and
/tmp/incant-agent-native-target, which may be reused as a warm build cache.
Create /tmp/incant0057 with fresh lease/output markers. Never reopen 0056 tools
or markers. Other lanes use separate CPU targets; leave them alone. Use brief
native-window review bursts, preserve the previous foreground app/pointer and
yield immediately to human interaction. No account/provider action is needed.

Commit focused changes with Built-by: claude; never publish or merge. Return
handoffs/0057-native-window-game-ui/result.md with checks, your visual judgment,
image links, exact limitations and proposed CPU contracts. Stop and reap all
owned player/helper/build processes, remove only your temporary node_modules
link if used, retain useful caches and record release of the GPU/native lease.

## Priority recovery and cross-platform failure — 2026-10-12

The host restarted; previous /tmp files, build caches and owned processes are gone.
Inspect and preserve your existing source edits and tools, then finish this packet.
Store NEW proof artifacts and logs under ~/.codex/incant-evidence/0057 instead of
/tmp, and the exclusive GPU/native cache under ~/.codex/incant-native-target.
No old missing process or result may be treated as a passed check.

First diagnose the actual Linux full-checkpoint failures in the accepted 0056
UI renderer tests at ui_draw_tests.rs:437 and :545: scale2 comparison worst7,
275 channels beyond1; nested clip worst2, one channel beyond1. Root's integration
owner retains the remote failure log and will provide it. Compare the intended
portable rendering behavior against the independent oracle, backend precision and
sampling rules. Correct a genuine rendering or reference-test defect at its source.
Do not simply widen thresholds to force a pass; any justified numerical tolerance
must state why it preserves the tested geometry/color/clip contract and demonstrate
that actual wrong geometry, clipping and opacity still fail. Avoid new CI runs;
return a focused source patch and local Metal/numeric review plus honest Linux
verification limitations to root for the next frozen checkpoint. Continue the real
native-window acceptance scope afterward. Root owns CPU geometry-receipt Windows
checks; do not edit tools/probes/render-check.py.
