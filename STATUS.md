# Implementation status

[PLAN.md](PLAN.md) revision 8 is the source of truth. Phase 1 is in progress;
the full engine and Driftwake release are not complete. Calendar targets and
phase durations have been removed. Work proceeds continuously and in parallel
where dependencies allow; exit gates still require their stated evidence.
Service-backed features use local containers or in-process substitutes during
development, behind production-shaped contracts. Real provider integrations,
security/device evidence and release gates remain real; local substitutes do not
satisfy those gates.

The director authorized Phase 1 with remaining Phase 0 items deferred, sensible
engineering progress without repeated advancement questions, and agent merges
after scoped review and passing checks. These authorizations do not approve a
phase gate. Section 12 decisions are settled. Current computer use and screen
capture are authorized; Claude Opus 5.5 with explicitly verified Max effort owns
visual work, and Astra owns correctness and integration.

## Durable budget lock release — correction validated locally

Checkpoint7 exposed Busy when reopening a completed ledger. A private lock guard
now explicitly releases ownership before descriptor close; two deterministic
shared-description regressions fail old source and pass fixed. All 64 agent
tests and Clippy pass, retaining live exclusion and conservative reservations.
Exact hosted scheduling cause and the next full candidate remain unverified.
[Evidence](docs/spikes/ledger-explicit-release.md).

## Release checkpoint6 correction

Failed checkpoint6 selected main110 and was never promoted. Three native physics jobs
exposed a stale isolated probe lock edge; the reviewed one-edge correction and
required-dependency guard pass local locked Cargo/Clippy and tooling checks.
The fixes are merged in PR #112; checkpoint7 now verifies frozen repaired
main113 through the same 23 required contexts in PR #111. Checkpoint7 finished
with 22 successful contexts and one macOS ledger failure; it is not promoted.
PR #115 repairs that failure locally, pending the next full candidate.
The independent fresh observation-routing correction restores the precise
pre-startup refusal while preserving absolute checkpoint-resume bounds; its
unchanged process regression, 36 scoped tests and Clippy pass.
[Lock evidence](docs/spikes/checkpoint6-probe-lock.md),
[routing evidence](docs/spikes/checkpoint6-player-routing.md).
The separate Windows budget-test correction closes the journal owner before
independent reads, then verifies recovered state. Seven local tests and Clippy
pass; PR #113 merged the fixture correction. Checkpoint7 Windows desktop and
Wasm conformance both pass.
[Journal test evidence](docs/spikes/checkpoint6-budget-journal-tests.md).

## Current milestone

Native two-input crossfade clocks and controls now retain independent outgoing
playback, pause both inputs, finish at explicit endpoints and persist active
transitions in world-save v5/session checkpoints. Older v4 animation saves keep
their encoding. The local core passed 319 runtime/script tests and scoped Clippy;
independent review accepted the exact sources. Canonical render bindings now carry
both samples through rollback and preallocated CPU blending; 350 distinct affected
CPU tests, Clippy and the wasm32 adapter check pass locally. Crossfade GPU/Claude
acceptance and TypeScript/Wasm controls remain active follow-ups; this is not full
blend trees, state machines, IK or root motion. See
[the consumer evidence](docs/spikes/native-animation-crossfade-consumer.md) and
[the crossfade evidence](docs/spikes/native-animation-crossfades.md).

The project is still building the Phase 1 core engine. Native simulation,
authored scripting, imports, physics, audio, input and source-free exports have
working, tested paths, but the complete Core Sample and its device gates remain
ahead. Editor and agent work overlaps later phases without completing their gates.

Native Wasm logging/localization, desktop gamepad input, session and
exported-player checkpoints, packaged-font text shaping, and the browser renderer
compile repair are merged into main, with skeletal asset foundations and font
import/cooking added in PRs #107–108. The actual native editor Play/chat/budget
review completed one real account-backed turn, Undo and reopen with retained
login and usage. Both follow-up UI fixes passed focused native rechecking; PR #109
merged this editor/agent/budget integration into main.

Skeletal animation merged in PR #116: authored clip metadata, native
clocks, atomic TypeScript controls, retained deformation and save continuation
are implemented locally. The combined runtime/cook/render suite passes 397 CPU
tests; a separate frozen renderer passes all 70 numerical Metal tests. Independent
review found two save-version admission defects, now corrected with actual world
and script-session continuation regressions. The integrated editor/runtime/script
suite passes 357 CPU tests, Clippy and a fresh native build with the current
production UI. The final main110 union passes 519 ordinary tests, 57 cooker/cooked-geometry CPU
tests (overlapping scope), Clippy and a fresh source-free export proof with exact
replay/checkpoint continuation and refusal controls. Claude0052 passed native playback/transport/resize/authoring checks, then found
the nullable clip field was unsupported in the Inspector. A bounded bridge
correction and focused Claude0053 native recheck now pass. The main114 fresh
export repeats exact source-free continuation after the atlas lockfile change;
main115 adds only the reviewed authoring ledger repair with 64 agent tests/Clippy.
Two adjacent Runtime focus/transient-layout observations remain scoped followups. [Integration evidence](docs/spikes/native-animation-integration.md).

Opt-in cooked output/diagnostic/shadow shaders merged in PR #110 after scoped
Metal numerical and Claude appearance acceptance; both features stay off.
Runtime glyph atlases proceed independently. Advanced
rendering, animation blending/state machines/IK/root motion, complete game UI/text
input, remaining SDK capabilities, and the Core Sample/2D sample with specified
device performance checks still prevent a Phase 1 completion claim. The feature queue is clear after repairs #112–113; animation is next, with
reviewed font atlas and Wasm animation parity queued separately.

Progress reports are scheduled approximately every 30 minutes while the goal is
active. Report merged work separately from local checks and unmet exit gates.
The project is still early relative to the complete Engine 1.0 and Driftwake
scope; no defensible percentage or completion date follows from the PR count.

## Native text scene, packaging and composed CPU input — locally checked

Typed Text declarations now cook into bounded v12 binary scene data with explicit
ordered font chains and immutable recipe bindings. Structural edits and
save/restore retain/check the same font catalog and invalidate retained consumers
by world identity/generation. Font loading and export data packaging preflight a
64 MiB aggregate before payload reads. The actual public command/import/cook,
export producer and data packager feed a fresh source-deleted numerical
Latin/Arabic/CJK shaping/atlas test. The existing composed host now retains that
canonical text input beside spatial/script/audio state; motion preserves immutable
layout/atlas allocations and failed candidates preserve accepted CPU input.
Three actual fresh exported `--package` runs after source/build deletion pass
mixed model, native script motion, multilingual shape/atlas and audible PCM
controls; missing/tampered packaged fonts refuse before output. The affected
120 CPU tests and Clippy pass; 74 GPU tests were intentionally not executed.
Plain games and legacy Play remain strict, and the canonical GPU consumer
explicitly refuses Text before submission. No second player mode was added.
Actual text drawing/Claude acceptance, widget layout/IME/focus and reference-phone
gates remain open. Evidence: [text bindings](docs/spikes/native-text-bindings.md),
[composed text input](docs/spikes/native-text-input.md) and
[draw handoff](docs/spikes/native-text-draw-handoff.md).

## Runtime font atlas — integrated CPU foundation

A retained CPU alpha atlas now rasterizes actual packaged TrueType glyphs with
content-bound keys, stable placements, explicit reset/invalidation and bounded
outline/mask admission. Latin, Arabic, Hebrew and a real CJK subset pass numerical
coverage tests, including a cache-deleted cooked-font reader route. Scoped 31
tests, Clippy, wasm32 compilation and lean runtime graph checks pass on the
main113 integration. Renderer
upload/draw consumption, widgets/input/IME, color/CFF support, Claude appearance
acceptance and device evidence remain; this does not complete Game UI.
[Scope and evidence](docs/spikes/native-font-atlas.md).


## Opt-in cooked output, diagnostic and shadow shaders — scoped Metal acceptance

The build-only cooker emits actual native MSL/SPIR-V/HLSL for output, diagnostic
geometry and all eight shadow variants. `cooked-output` and `cooked-geometry`
remain disabled by default. Private fixed-family consumption validates the full
vertex/resource contract before native shader calls. CPU cooker/renderer tests,
Clippy, default/feature Web compilation and build-only dependency guards pass.
The frozen v2 default/cooked pair passes all 138 existing/new numerical renderer
tests on Apple M5 Pro/Metal: all 16 output and 12 actual-diagnostic readbacks are
byte-identical. The actual main109 integration repeats all 138 tests successfully,
with identical paired and prior raw results. Claude0051 inspected 24 bounded PNGs
from eight successful/reaped processes and reports scoped Metal appearance passes;
all 12 pairs are identical. Shared thin-wall contact leakage, low-sun aliasing,
far thin-caster breakup and subtle receiver modulation remain renderer follow-ups;
the shadow fixture does not establish main-pass caster visibility. Vulkan/DX12
drivers, phones/performance, remaining shader families, caches and whole-renderer
translation removal remain open. Both features stay disabled by default.
[Contract and evidence](docs/spikes/cooked-geometry-shaders.md).

## Skeletal pose blending — reviewed main integration
