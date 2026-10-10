# Revision 4 architecture review and migration order

Reviewed 2026-10-10 against PLAN.md revision 4 and main at `af15941`, with the
in-flight pixel-camera, sprite and atlas branches inspected separately. This is
an implementation review, not phase approval or measured performance evidence.

## Outcome

Adopt the director's performance architecture. The existing authoring command
boundary stays intact; the gameplay path must change substantially. Preserve
working behavior and asset IDs while replacing the document-backed simulation,
rather than adding further gameplay features to that path. Update ADRs 0002,
0003, 0006 and 0008 and both generated agent instruction files with this revision.

Three wording clarifications accompany the supplied plan: binary scene loading
still validates data without text parsing; browser exports explicitly retain
JavaScript and WGSL; executable caches need a trusted cook/export origin, not
merely a hash supplied with the same untrusted content. No numerical target or
release requirement is removed.

## Existing work that needs rework

| Area | Current implementation evidence | Required change |
| --- | --- | --- |
| Play simulation | `crates/incant_script/src/play.rs` clones the Project into a simulation CommandBus each tick, reflects the physics snapshot into component commands and calls Engine::sync. | Cook/load a native ECS world; make simulation independent of authoring; structural changes commit at scheduler sync points. |
| Script bridge | `crates/incant_script/src/lib.rs` serializes bus.project(); `runtime.js` parses the whole world and returns serialized commands. | Typed bulk component access, checked view lifetimes, no per-tick world serialization; preserve scalar query behavior through the same runtime API. |
| Runtime dependency boundary | There is no incant_runtime crate yet. incant_core, physics, assets and script hosts depend on incant_doc. | Extract runtime component types and codecs before adding the lean runtime; enforce the real transitive graph. A facade or empty crate must not count as migration. |
| Script startup and saves | QuickJS consumes SWC output; PlaySession saves contain authored-shaped JSON state. | Trusted versioned bytecode cook/load, binary world saves, host conformance, browser worker and Wasm host. Preserve save/resume semantics and make old-save migration/version refusal explicit. |
| Scheduler and physics | Bevy ECS is configured with std only; Rapier 0.36 unconditionally enables enhanced-determinism and current stepping is serial. | Enable/test native multithreaded scheduling, create separately built project-selected physics variants, and measure target-supported vector/parallel paths. |
| Renderer | GPU objects are retained in memory; shaders are created from WGSL and render preparation consumes authoring-shaped data. | Runtime render extraction, offline native shader translation, persistent driver caches, pre-warming and hitch tracing, then temporal/dynamic-resolution/interop passes with portable fallbacks. |
| Document scaling | CrdtProject owns one LoroDoc for a whole Project; journal entries retain full states. | Lazy per-scene documents, bounded recovery/compaction, atomic cross-scene commands and measured 100,000-entity edits. |
| Inspector scaling | Claude's ongoing 0033 review reproduced a stall when the old Inspector mounted a 65,536-value navigation grid. | Bounded/virtualized panels and large-data interaction coverage within the full visual redesign; browser evidence cannot certify the M1 device gate. |
| Performance CI | Current CI runs builds, behavior tests, GPU tests and public probes; no reference-phone performance runner or shipping-profile Core Sample gate exists. | Add shipping profile, profiler instrumentation, representative workloads, reference-device baselines and >5% regression comparison with attached captures. Missing runs stay explicitly unmet. |

Current sprite/atlas work remains useful: stable frame references, deterministic
cooking, immutable asset versions, padding and region sampling are compatible
with the new direction. Their old PlaySession probes establish behavior only.
Runtime texture metadata/types and render extraction must cross the new boundary;
do not promote those probes into evidence for native-world simulation or phone
performance. Pixel-camera numerical/rendered review also remains scoped evidence.

## Ordered work

1. Record and merge the plan, decisions and instruction updates. Deliver the
   revised plan to ongoing Claude sessions; preserve their completed evidence and
   in-flight work without attributing old captures to the new implementation.
2. Extract a runtime-safe schema/component boundary and define checked, versioned
   binary scene loading. Implement a real cooked-scene-to-Bevy-world path with
   stable IDs and behavioral tests, then enforce its transitive dependencies in
   CI. Keep authoring conversion in the cooker/editor, outside the runtime.
3. Move the existing physics, input, navigation, timers, localization and script
   behavior onto that world. Retire per-tick document cloning/validation/JSON;
   preserve authoring immutability, failure semantics and exact save/resume where
   the selected physics mode promises it. Observation becomes an explicit query,
   not a prerequisite for advancing every entity on every tick.
4. Add native QuickJS bytecode with provenance/version checks and bulk query
   views. In parallel with target conformance work, prove the iOS static AOT
   module route and browser worker host; implement Rust build isolation before
   accepting arbitrary module source. The TypeScript AOT choice stays open.
5. Establish shipping-profile CPU/GPU/allocation measurements and per-project
   physics builds, then shader/pipeline cook and pre-warm work. Apply the same
   budgets to further rendering/gameplay increments; do not make optimization a
   final cleanup phase. Rendered changes continue through Claude Opus 5.5 Max.
6. Complete per-scene authoring storage and the 100,000-entity editor gate.
   Continue Core Sample, reference-phone and competitive benchmarks throughout
   the relevant phases. Hardware purchases, signing and vendor license review
   remain human-owned; their absence is not permission to fabricate a pass.

The overlapping UI redesign can proceed now, including bounded large-array
controls and clearer separation of authored selection from observed runtime
state. Astra will integrate the new backend contracts. No placeholder Play/Keep
Edit action should imply functionality that has not been implemented.

## Technical review constraints and sources

- QuickJS bytecode is version-specific and not validated for safety by its
  loader. Verify trusted origin in addition to package integrity, and rebuild
  from sandboxed source when opening untrusted projects. See the
  [QuickJS embedding documentation](https://bellard.org/quickjs/quickjs.html#Script-evaluation).
- Wasmtime precompiled artifacts also require trusted origin. Its serialized
  module loading example does not demonstrate statically linked, signed iOS
  code without executable-memory allocation. Keep that path as an explicit
  target spike until demonstrated. See
  [Wasmtime precompilation](https://docs.wasmtime.dev/examples-pre-compiling-wasm.html).
- wgpu 29 exposes persistent PipelineCache support only on Vulkan. Metal binary
  archives and DX12 pipeline libraries need their own interop paths. Cache keys
  must include compatible device/driver and build inputs. See
  [wgpu PipelineCache](https://docs.rs/wgpu/29.0.0/wgpu/struct.PipelineCache.html).
- WebGPU's shader module API accepts WGSL source. Native offline translation
  cannot be imposed unchanged on the browser backend. See the
  [WebGPU API specification](https://gpuweb.github.io/gpuweb/#dom-gpudevice-createshadermodule).
- Pinned Rapier 0.36's Cargo features include `parallel` and `simd8`. Inspect the
  resolved crate when building variants; do not copy feature names from older
  versions. Deterministic imports and floating-point behavior across all hosts
  remain part of cross-platform testing. See
  [Rapier 0.36 features](https://docs.rs/crate/rapier3d/0.36.0/features).

## Verification boundary

This change edits planning, decisions and generated instructions only. Run the
convention generator/check, tool tests, local link checks and diff checks; hosted
checks must pass on the actual PR revision before merge. Existing build/test
results are regression evidence, not evidence that the revised performance gates
are already implemented. The 2 ms phone budget, shipping runtime dependency gate,
Tracy captures, >5% reference-device PR comparisons, 100,000-entity M1 target and
competitive benchmark all remain open.
