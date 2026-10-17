# Phase 1 review ledger

Historical evidence retains main359, the fresh CJK/pseudo shipping players from
`e3dfcd9c695d6e1a74f82cb1613891c9e6046bb5`, and the corrected Arabic shipping
export/CPU/native routes from `069bbe54dc529511138867dfd4fbe50507a38b6b`.
The later combined [sampled Core locale proof](../spikes/core-locale-sampled-captures.md)
uses four newly packaged shipping players from
`50b5408b1d6be66dbb1dcbca501d4380add9d656` and five same-package CPU/capture routes.
Those producing identities remain
explicit; they are not relabeled as later integration builds.
Release checkpoint50 tested the main376 tree; later changes
remain outside that release baseline.
Phase 1 remains open; this ledger records evidence and remaining work, not director
approval. [PLAN.md](../../PLAN.md#phase-1-core-engine) defines the requirements.
Only actual testing on the available Mac is required as device evidence. Other
device execution is deferred; hosted build and release checks remain required.

| Exit requirement | Recorded evidence | Remaining qualification |
| --- | --- | --- |
| Core Sample runs at 60 fps, using TypeScript and no editor | Authored Core gameplay, packaged HUD/audio and source-free native execution have scoped proofs. A [shipping native-window replay](../spikes/native-window-replay.md) completes the full 2,156-step route with exact headless observation parity. The later [complete host-stage diagnostic](../spikes/native-presentation-host-stages.md#complete-same-source-live-replay) also records all 2,156 steps with independent stage arithmetic, preserving its historical package and earlier failed attempt. [Core integration](../spikes/core-main182.md), [HUD](../spikes/core-result-hud-main192.md). | A shipping run of the complete rendered game meeting the frame budget. Zero callbacks, incomplete display-interval coverage and an early host/callback discrepancy keep displayed cadence open. Complete host timing and CPU simulation timing do not establish FPS, causality or shader-stall freedom. Rendering reconstruction still has unresolved appearance defects. |
| CI plays Core Sample and asserts game state | The general Mac and desktop workflows execute `samples/core/verify.py` against the native headless binary. All required checks passed on [checkpoint50](../spikes/release-checkpoint-50.md). | Preserve these checks as the sample and runtime change. This requirement's recorded coverage does not pass the other exit requirements. |
| Navmesh enemies, pseudo-localization, RTL/CJK rendering and the 2D sample | Native Core/planar scenarios, text shaping and presentation have scoped evidence in [STATUS.md](../../STATUS.md). The [2D sample](../spikes/native-window-eligibility.md) completes its full native Mac win route with exact CPU parity and three actual window captures; desktop CI also runs `samples/planar/verify.py`. The combined [Core locale proof](../spikes/core-locale-sampled-captures.md) now passes five full CPU-matched routes with 37 selected offscreen Metal frames. Independent review of 13 originals verifies winning text, Arabic damaged-pip direction, complete CJK glyphs, pseudo expansion and narrow loss/win wrapping. | Sampled locale appearance is verified on this Mac, without phase approval. Continuous motion and native-window presentation are outside the new sampled result. Earlier [Arabic](../spikes/core-arabic-corrected-window.md) and CJK native routes retain their successful numerical runs and missed winning images; the pseudo native replay remains focus-paused/timed out. Those attempts are not relabeled by the new offscreen proof. Preserve the 2D run's focus interruption/surface deferral and the separate complete rendered-game FPS requirement. |
| Script p95 within 2 ms with 10,000 scripted entities; no shader compilation stalls above twice the frame budget | Shipping script comparisons and exact gameplay/audio parity are recorded. The latest bounded [navigation cache comparison](../spikes/navigation-search-cache-outcome.md) remained above 2 ms and was not adopted. | The 2 ms requirement remains unmet. The director's 2026-10-14 clarification permits periodic bounded investigations, including subagents, while other engine work continues. Preserve negative results and require a distinct hypothesis before another measured experiment. Whole rendered-game shader/pipeline stall qualification also remains open. |
| Native runtime data path and shipped dependency boundary | Cooked-world, source-free gameplay and runtime dependency checks have recorded implementations and proofs. See [Core integration](../spikes/core-main182.md) and the [shipping observations](../spikes/quickjs-loan-main278.md). | Retain the boundary on the final complete sample/export configuration. No authoring document, command bus or agent dependency may enter its runtime tick path. |
| Shipped wasm2c module runs on this Mac, with cost recorded against TypeScript | A [matched shipping workload](../spikes/wasm-typescript-cost.md) records both backends and functional parity. Existing native target builds remain required. | Keep its limited workload and historical build identity explicit; it is not a Core10k, rendered frame-time or startup result. This evidence does not establish complete Wasm SDK parity. |

## Current implementation priorities

- Finish portable temporal reconstruction and the Apple MetalFX path, then review
  the rendered result and measure the complete shipping game. Finite visibility
  remains a private experiment. The default-off MetalFX foundation is merged,
  but its [material/motion comparison](../spikes/metalfx-image-motion.md) fails
  appearance review. Neither route has accepted quality or performance evidence.
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
  This adds no appearance or performance acceptance. Changed skeleton/geometry and scene structure still need
  their own admission rules.
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
The director has allowed capture again; the retained
focus-interrupted runs are not relabeled as complete.
