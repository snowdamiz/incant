# Phase 1 review ledger

Recorded against main287 (`bb131b9347dbfc6e1ea8e5763bc0fe29f6b5bccc`).
Phase 1 remains open; this ledger records evidence and remaining work, not director
approval. [PLAN.md](../../PLAN.md#phase-1-core-engine) defines the requirements.
Only actual testing on the available Mac is required as device evidence. Other
device execution is deferred; hosted build and release checks remain required.

| Exit requirement | Recorded evidence | Remaining qualification |
| --- | --- | --- |
| Core Sample runs at 60 fps, using TypeScript and no editor | Authored Core gameplay, packaged HUD/audio and source-free native execution have scoped proofs. [Core integration](../spikes/core-main182.md), [HUD](../spikes/core-result-hud-main192.md). | A shipping run of the complete rendered game meeting the frame budget. CPU simulation timing alone does not establish FPS. Rendering reconstruction still has unresolved appearance defects. |
| CI plays Core Sample and asserts game state | The general Mac and desktop workflows execute `samples/core/verify.py` against the native headless binary. All required checks passed on [checkpoint37](../spikes/release-checkpoint-37.md). | Preserve these checks as the sample and runtime change. This requirement's recorded coverage does not pass the other exit requirements. |
| Navmesh enemies, pseudo-localization, RTL/CJK rendering and the 2D sample | Native Core/planar scenarios, text shaping and presentation each have scoped evidence in [STATUS.md](../../STATUS.md). Desktop CI also runs `samples/planar/verify.py`. | Qualify the complete specified presentation on this Mac; individual CPU, shaping and numerical pixel controls do not substitute for the combined rendered sample. |
| Script p95 within 2 ms with 10,000 scripted entities; no shader compilation stalls above twice the frame budget | Shipping script comparisons and exact gameplay/audio parity are recorded. The latest bounded [navigation cache comparison](../spikes/navigation-search-cache-outcome.md) remained above 2 ms and was not adopted. | The 2 ms requirement remains unmet. Per director instruction, further optimization attempts for that threshold are stopped while other engine work continues. Whole rendered-game shader/pipeline stall qualification also remains open. |
| Native runtime data path and shipped dependency boundary | Cooked-world, source-free gameplay and runtime dependency checks have recorded implementations and proofs. See [Core integration](../spikes/core-main182.md) and the [shipping observations](../spikes/quickjs-loan-main278.md). | Retain the boundary on the final complete sample/export configuration. No authoring document, command bus or agent dependency may enter its runtime tick path. |
| Shipped wasm2c module runs on this Mac, with cost recorded against TypeScript | A [matched shipping workload](../spikes/wasm-typescript-cost.md) records both backends and functional parity. Existing native target builds remain required. | Keep its limited workload and historical build identity explicit; it is not a Core10k, rendered frame-time or startup result. This evidence does not establish complete Wasm SDK parity. |

## Current implementation priorities

- Finish portable temporal reconstruction and the Apple MetalFX path, then review
  the rendered result and measure the complete shipping game. Private finite
  visibility and native interop experiments are not merged features or accepted
  appearance/performance results.
- Extend live asset refresh beyond existing textures. The actual Mac editor
  [texture workflow](../spikes/native-window-texture-workflow.md) now passes while
  preserving the running world and scripts. Compatible static-model replacement
  is the next slice; animation, topology and baked navigation dependencies require
  their own admission rules.
- Complete off-frame shader/pipeline preparation and cache coverage for the final
  renderer configuration. The existing optional cooked families and Vulkan cache
  do not establish every backend, permutation or frame-time requirement.
- Finish and qualify remaining deliverables against the complete native sample.
  The [TypeScript AOT spike](../spikes/typescript-aot.md) remains an adoption
  experiment; its language controls are not a measured replacement for QuickJS.

The real-mouse capture check is pending the director's availability and is not
blocking these lanes. Broader editor, agent, content, export, multiplayer, game
production and distribution work remains in the plan's later and overlapping
phases. [Phase 0](phase0-review.md) also retains its explicit deferred items.
