# Phase 1 review ledger

Updated against main321 and the fresh CJK/pseudo shipping players from
`e3dfcd9c695d6e1a74f82cb1613891c9e6046bb5`. Those producing identities remain
explicit; they are not relabeled as later integration builds.
Release checkpoint40 tested the main309 tree; later main310+
work remains outside that release baseline.
Phase 1 remains open; this ledger records evidence and remaining work, not director
approval. [PLAN.md](../../PLAN.md#phase-1-core-engine) defines the requirements.
Only actual testing on the available Mac is required as device evidence. Other
device execution is deferred; hosted build and release checks remain required.

| Exit requirement | Recorded evidence | Remaining qualification |
| --- | --- | --- |
| Core Sample runs at 60 fps, using TypeScript and no editor | Authored Core gameplay, packaged HUD/audio and source-free native execution have scoped proofs. A [shipping native-window replay](../spikes/native-window-replay.md) completes the full 2,156-step route with exact headless observation parity. [Core integration](../spikes/core-main182.md), [HUD](../spikes/core-result-hud-main192.md). | A shipping run of the complete rendered game meeting the frame budget. The [complete observer/audio replay](../spikes/native-presentation-audio.md) verifies gameplay parity, while zero/equal drawable timestamps keep full display timing open. CPU simulation timing alone does not establish FPS. Rendering reconstruction still has unresolved appearance defects. |
| CI plays Core Sample and asserts game state | The general Mac and desktop workflows execute `samples/core/verify.py` against the native headless binary. All required checks passed on [checkpoint40](../spikes/release-checkpoint-40.md). | Preserve these checks as the sample and runtime change. This requirement's recorded coverage does not pass the other exit requirements. |
| Navmesh enemies, pseudo-localization, RTL/CJK rendering and the 2D sample | Native Core/planar scenarios, text shaping and presentation each have scoped evidence in [STATUS.md](../../STATUS.md). The [2D sample](../spikes/native-window-eligibility.md) now completes its full native Mac win route with exact CPU parity and three actual window captures. Desktop CI also runs `samples/planar/verify.py`. | The [Arabic sample](../spikes/core-locale-native-window.md) has complete native/CPU state parity and partial actual pixels; its visual replay timeout and crowded initial Retina layout remain explicit. A fresh CJK package now completes all 3,056 native host steps at 640×480 logical units, with four actual images of readable HUD, loss/reset and resumed play. Its final win image was missed. The pseudo-locale now passes fresh export and all three CPU routes, with readable normal-size initial/loss text in actual images; its focus-paused native replay timed out after restart. Complete normal-size Arabic, remaining CJK appearance, full pseudo native replay and narrow pseudo layout on this Mac. Preserve the 2D run's focus interruption and surface deferral; it is not a performance or continuous-motion result. Individual shaping and numerical pixel controls do not substitute for the combined rendered Core sample. |
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
  covering import, watcher, history, committed refusal and recovery. Changed skeleton/geometry,
  scene/material-binding structure and baked navigation dependencies require
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
  gameplay/PCM replay. Its wake/reservation/body fields have not been observed in
  a native window; live coverage and any resulting frame-pacing change remain
  unqualified.
- Finish and qualify remaining deliverables against the complete native sample.
  The [TypeScript AOT spike](../spikes/typescript-aot.md) remains an adoption
  experiment; its language controls are not a measured replacement for QuickJS.

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
reset and repeated locale switches. The earlier native package is unchanged;
normal-size Arabic appearance and the combined locale gate remain open pending
fresh native review. The director has allowed capture again; the retained
focus-interrupted runs are not relabeled as complete.
