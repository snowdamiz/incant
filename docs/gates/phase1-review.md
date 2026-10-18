# Phase 1 review ledger

Historical evidence retains main359, the fresh CJK/pseudo shipping players from
`e3dfcd9c695d6e1a74f82cb1613891c9e6046bb5`, and the corrected Arabic shipping
export/CPU/native routes from `069bbe54dc529511138867dfd4fbe50507a38b6b`.
The later combined [sampled Core locale proof](../spikes/core-locale-sampled-captures.md)
uses four newly packaged shipping players from
`50b5408b1d6be66dbb1dcbca501d4380add9d656` and five same-package CPU/capture routes.
Those producing identities remain
explicit; they are not relabeled as later integration builds.
[Release checkpoint58](../spikes/release-checkpoint-58.md) promoted the main425 tree
after all 23 required contexts and eight frozen-head workflows succeeded. Release
`ae58b0bb` has the identical tested tree; main426 and later work remains outside
that release baseline.
Phase 1 remains open; this ledger records evidence and remaining work, not director
approval. [PLAN.md](../../PLAN.md#phase-1-core-engine) defines the requirements.
Only actual testing on the available Mac is required as device evidence. Other
device execution is deferred; hosted build and release checks remain required.

A [fresh Core v2 native diagnostic](../spikes/native-wake-acquisition-live.md)
now completes all 2,156 steps with exact offline gameplay parity and full
wake/acquisition attribution. Its original audio-schema analysis refusal and
separate saved-data correction remain explicit. A second same-player run completes
without focus loss and with positive timestamps for all gameplay frames. Uneven
display intervals persist, including 27 long-then-short pairs. The complete
rendered-game performance gate remains open; no gate follows from timing coverage.
A later [matched minimum-duration diagnostic](../spikes/native-metal-minimum-duration.md)
is negative for adoption: fewer short intervals accompany a longer presentation
span, substantially more drawable acquisition wait and four dropped wall ticks.
The selected arm's strict timing receipt also remains failed at its partial final
scheduler batch. The ordinary policy stays unchanged.

The subsequent [upstream comparison](../spikes/upstream-rendering-comparison.md)
uses pinned Godot, Bevy, Filament and FSR2 implementations to define new experiments.
The [batched presentation route](../spikes/native-batched-presentation.md) has
local CPU/Metal correctness and fresh shipping gameplay/audio parity. The later
quiet three-arm comparison completes without focus loss but rejects latest-batch
timing: presented-interval p95 is 25 ms versus ordinary's 16.667 ms. Its strict
selected-arm capture receipt also fails because committed-redraw wake metadata
is incomplete; independent analysis preserves that failure and includes all
display roles. The separate [simulation-deadline experiment](../spikes/native-independent-simulation-deadline.md)
now has a fresh shipping build, exact full CPU gameplay/PCM parity and one quiet
native pair. Valid positive-adjacent interval p95 and maximum are unchanged;
long intervals number 10 ordinary and 12 independent, with two and one zero
callbacks retained. Its focus/resize attempt timed out without a resize, and
native restart remains unexecuted. Only the report and evidence enter main;
the scheduling source stays on its experiment branch. The default-off
[pose-interpolation adapter and composed host](../spikes/native-pose-interpolation.md)
are merged through PR403 with CPU and actual Metal correctness; they are not yet
qualified in the native window. The later default-off
[Core pose diagnostic](../spikes/native-core-pose-diagnostic.md) adds native-window
wiring, simple clip animation and fixed-tick HUD. Its full 2,156-step CPU replay
preserves historical gameplay, complete PCM and HUD reports through 6,468 pose
displays; the original incomplete-report failure remains recorded. This is not
shipping, window presentation or frame-pacing evidence. GPU color reconstruction improves the small
analytic edge metric but still loses thin geometry and has no full visual
acceptance. Its output-resolution visibility probe now isolates 48 inconsistent
owner/depth samples for further diagnosis. These results narrow the remaining
problems without changing any exit requirement.

| Exit requirement | Recorded evidence | Remaining qualification |
| --- | --- | --- |
| Core Sample runs at 60 fps, using TypeScript and no editor | Authored Core gameplay, packaged HUD/audio and source-free native execution have scoped proofs. A [shipping native-window replay](../spikes/native-window-replay.md) completes the full 2,156-step route with exact headless observation parity. The later [complete host-stage diagnostic](../spikes/native-presentation-host-stages.md#complete-same-source-live-replay) also records all 2,156 steps with independent stage arithmetic, preserving its historical package and earlier failed attempt. [Core integration](../spikes/core-main182.md), [HUD](../spikes/core-result-hud-main192.md). | A shipping run of the complete rendered game meeting the frame budget. The latest same-player diagnostic has positive timestamps for every gameplay frame but retains uneven displayed intervals; earlier missing/zero callbacks remain historical. Complete timing coverage and CPU simulation timing do not establish FPS, causality or shader-stall freedom. Rendering reconstruction still has unresolved appearance defects. |
| CI plays Core Sample and asserts game state | The general Mac and desktop workflows execute `samples/core/verify.py` against the native headless binary. All required checks passed on [checkpoint54](../spikes/release-checkpoint-54.md). | Preserve these checks as the sample and runtime change. This requirement's recorded coverage does not pass the other exit requirements. |
| Navmesh enemies, pseudo-localization, RTL/CJK rendering and the 2D sample | Native Core/planar scenarios, text shaping and presentation have scoped evidence in [STATUS.md](../../STATUS.md). The [2D sample](../spikes/native-window-eligibility.md) completes its full native Mac win route with exact CPU parity and three actual window captures; desktop CI also runs `samples/planar/verify.py`. The combined [Core locale proof](../spikes/core-locale-sampled-captures.md) now passes five full CPU-matched routes with 37 selected offscreen Metal frames. Independent review of 13 originals verifies winning text, Arabic damaged-pip direction, complete CJK glyphs, pseudo expansion and narrow loss/win wrapping. | Sampled locale appearance is verified on this Mac, without phase approval. Continuous motion and native-window presentation are outside the new sampled result. Earlier [Arabic](../spikes/core-arabic-corrected-window.md) and CJK native routes retain their successful numerical runs and missed winning images; the pseudo native replay remains focus-paused/timed out. Those attempts are not relabeled by the new offscreen proof. Preserve the 2D run's focus interruption/surface deferral and the separate complete rendered-game FPS requirement. |
| Script p95 within 2 ms with 10,000 scripted entities; no shader compilation stalls above twice the frame budget | Shipping script comparisons and exact gameplay/audio parity are recorded. The latest bounded [navigation cache comparison](../spikes/navigation-search-cache-outcome.md) remained above 2 ms and was not adopted. | The 2 ms requirement remains unmet. The director's 2026-10-14 clarification permits periodic bounded investigations, including subagents, while other engine work continues. Preserve negative results and require a distinct hypothesis before another measured experiment. Whole rendered-game shader/pipeline stall qualification also remains open. |
| Native runtime data path and shipped dependency boundary | Cooked-world, source-free gameplay and runtime dependency checks have recorded implementations and proofs. See [Core integration](../spikes/core-main182.md) and the [shipping observations](../spikes/quickjs-loan-main278.md). | Retain the boundary on the final complete sample/export configuration. No authoring document, command bus or agent dependency may enter its runtime tick path. |
| Shipped wasm2c module runs on this Mac, with cost recorded against TypeScript | A [matched shipping workload](../spikes/wasm-typescript-cost.md) records both backends and functional parity. Existing native target builds remain required. An [ordinary authored shipping save/load proof](../spikes/native-wasm-checkpoint-export.md) now verifies declared host-owned state in fresh VMs, exact checkpoint/audible PCM continuation and six specific refusals. The original wrapper failure and corrected data-only acceptance remain separate. | Keep each workload and native producer explicit; neither is a Core10k, rendered frame-time or startup result. Save/load covers opted-in explicit state in a plain scene, without physics/UI or arbitrary guest memory capture. A subsequent [authored timer export](../spikes/native-wasm-timers-export.md) verifies pending simulation schedules and exact gameplay/audio continuation through normal packaging and fresh processes. [Cooperative native coroutines](../spikes/native-wasm-coroutines.md) also pass local runtime and actual two-host continuation/refusal checks; the [normal authored shipping coroutine proof](../spikes/native-wasm-coroutines-export.md) now also passes exact owned continuation and complete audio parity in fresh processes. The [authored localization export](../spikes/native-wasm-localization-export.md) now also verifies canonical coverage pages, committed settings and exact save/audio continuation in fresh players. Its original checker-framing failure and separate same-package follow-up remain explicit. An [ordinary instance export](../spikes/native-wasm-instances.md#ordinary-authored-export) also preserves typed owner tuning, immediate-removal checkpoints, same-ID retirement and exact save/audio continuation through four fresh processes. Complete Wasm SDK parity remains open. |

## Current implementation priorities

The real [Rust input guest qualification](../spikes/native-wasm-input-conformance.md)
also passes typed collection/gesture/focus parity and transactional refusals on
both native module hosts. This extends CPU conformance; physical input hardware,
complete SDK parity and the separate shipping/frame-time gates remain open.

- Finish portable temporal reconstruction and the Apple MetalFX path, then review
  the rendered result and measure the complete shipping game. Finite visibility
  remains a private experiment. The default-off MetalFX foundation is merged,
  but its [material/motion comparison](../spikes/metalfx-image-motion.md) fails
  appearance review. Neither route has accepted quality or performance evidence.
  The private reconstruction [complete-route cost comparison](../spikes/reconstruction-complete-route-cost.md)
  now rejects its current CPU-admission layout: 94–97 ms versus about 3 ms for
  original shading in the matched CI-profile diagnostic. GPU-only admission and
  reduced transfers/passes are the next unqualified design; the new numerical
  result does not approve appearance, shipping performance or a phase gate.
- Extend live asset refresh beyond the verified existing-texture and
  [compatible static-model workflows](../spikes/native-static-model-refresh.md).
  Both preserve the running world and scripts on this Mac. Compatible animation
  keys now have [CPU and numerical Metal qualification](../spikes/animated-model-refresh.md)
  and an [actual editor-window workflow](../spikes/native-window-animated-model-workflow.md)
  covering import, watcher, history, committed refusal and recovery.
  [Model-local material table replacement](../spikes/live-material-binding-refresh.md)
  now passes CPU and numerical Metal checks, including geometry-divergence
  provenance for navigation and atomic publication. Its actual native-window
  workflow now also verifies table replacement, watcher/Undo/Redo, malformed-source
  retention, recovery, continuing VM/world state and durable journal reopen.
  This adds no appearance or performance acceptance. Compatible animated vertex
  content now has [scoped CPU and Metal qualification](../spikes/animated-vertex-refresh.md):
  import/history preserve the world, VM and clock, while unchanged skin bindings
  cannot mask an index-only incompatibility. Position-only and tangent-only
  replacements match independent fresh-game RGBA at the same clock on Metal;
  the actual editor-window workflow now also verifies import, watcher, history,
  topology refusal/recovery, continuing state and fresh journal reopen on this Mac.
  Changed skeleton, topology and scene
  structure still need their own admission rules.
- Preserve the now-qualified [ordinary-editor 4x antialiasing](../spikes/editor-scene-msaa.md).
  Actual Mac editor review and default/cooked Metal controls verify the reported
  cube-edge fix. This does not qualify temporal reconstruction or frame budgets;
  additional cold pipelines and memory remain explicit costs.
- Complete off-frame shader/pipeline preparation and cache coverage for the final
  renderer configuration. The existing optional cooked families and Vulkan cache
  do not establish every backend, permutation or frame-time requirement.
  The [combined native observer replay](../spikes/native-compilation-live.md)
  now records all 95 native creation calls outside gameplay on its complete Core
  route. Deferred driver work, ambiguous display timestamps and uneven cadence
  remain open; this observation does not establish the complete shader-stall gate.
  The default-off [host-stage observer](../spikes/native-presentation-host-stages.md)
  now has CPU/analyzer qualification and a fresh shipping build with exact full
  offline gameplay/PCM replay. One native run records valid wake/reservation/body
  stages for 263 of 2,156 requested steps before focus loss suspends simulation.
  A separately granted same-player repeat now completes all 2,156 host-stage
  rows, with exact gameplay checks and independent arithmetic. Its two zero
  callbacks and host/callback discrepancy preserve the displayed-cadence gap;
  no scheduling fix or performance acceptance follows from these spans.
- Finish and qualify remaining deliverables against the complete native sample.
  The [TypeScript AOT spike](../spikes/typescript-aot.md) remains an adoption
  experiment; its language controls are not a measured replacement for QuickJS.
  The [native Static Hermes follow-up](../spikes/static-hermes-feasibility.md)
  passes seven erased-JavaScript fixtures but retains typed-input refusals and a
  failed native deadline control. It does not justify replacing QuickJS or
  satisfy the script-time gate.

The new [repeated-start attribution](../spikes/navigation-start-attribution.md)
also rejects that specific scan-only hypothesis for reaching 2 ms: its hypothetical
same-frame zero-cost p95 is still 2.544041 ms. The reusable observer is default off;
no navigation optimization or new shipping performance result is adopted.

The retained [QuickJS C profile-guided experiment](../spikes/quickjs-core-pgo.md)
completes one coherent ordinary/generation/use Core producer, with exact full
win/loss/restart gameplay and PCM after build/input deletion. Its eight matched
CPU arms preserve a mixed win result: ScriptTick p95 improves from 2.270–2.280 to
2.170–2.180 ms in one pair, but regresses from 2.230–2.240 to 2.410–2.420 ms in
the other. All four win arms remain above 2 ms. Held-out loss improves modestly,
with both modes already below 2 ms. Main receives documentation and self-contained
inert evidence only; the full default-off implementation remains on the retained
`codex/quickjs-core-pgo` experiment branch at `bb5ae0db`. The ordinary build stays
unchanged. No adoption, rendering/FPS result or phase approval follows. Original
failures and exact producing sources remain preserved; a new source-specific
residual-cost hypothesis is required before another experiment.

Compatible HUD text updates now [reuse checked font engines](../spikes/ui-font-reuse.md)
through PR406. Scoped behavior tests and strict lints pass; the observed long
event frames still need causal attribution and matched shipping measurement.

The default-off [native-query diagnostic](../spikes/script-query-attribution.md)
passes focused CPU controls, strict lints and a same-source shipping comparison
with exact full-Core gameplay and offline PCM parity. It separates parsing,
native execution and result conversion while preserving query admission,
rollback and bounded storage. Parsing and result conversion account for only
0.305% of ScriptTick time in the same-frame tail of the instrumented run; this
does not support query marshalling as the next optimization. The 2 ms gate remains
unmet. A distinct callback, VM or typed-view cost hypothesis is needed before
another measured experiment; this diagnostic is not a performance improvement.

The real-mouse capture check is pending the director's availability and is not
blocking these lanes. Broader editor, agent, content, export, multiplayer, game
production and distribution work remains in the plan's later and overlapping
phases. [Phase 0](phase0-review.md) also retains its explicit deferred items.

The [Arabic health-label correction](../spikes/core-rtl-health-direction.md) now
passes actual Core localization and font-shaping regressions across damage,
reset and repeated locale switches. A [fresh corrected shipping export and all
three CPU routes](../spikes/core-arabic-corrected-export.md) now pass, with exact
localized HUD, state and PCM parity. The earlier native package is unchanged. The corrected package now has
[two complete native replays and partial actual appearance review](../spikes/core-arabic-corrected-window.md),
including readable normal-size Arabic and corrected pip direction. The later
[combined sampled proof](../spikes/core-locale-sampled-captures.md) now adds actual
winning pixels for Arabic/CJK/pseudo and narrow pseudo loss/win layouts, with
exact full-route CPU parity. This is sampled appearance evidence, not complete
native-window motion/presentation or phase approval.
Computer use and screen capture are currently paused for all agents by the
director. Native window checks are held. The retained focus-interrupted runs
are not relabeled as complete; terminal implementation and data-only replay continue.
