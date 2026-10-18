# Upstream rendering and pacing comparison

The director's request to study other engines led to two concrete experiments:
separate fixed simulation steps from drawable submission, and reconstruct color
history on the GPU with filtered sampling and explicit depth/motion support.
These are implementation hypotheses, not evidence that upstream performance or
appearance transfers to Incant. Existing negative results remain retained.
The retained [temporal source manifest](evidence/upstream-rendering-20261017/temporal-sources.json)
and [Godot source manifest](evidence/upstream-rendering-20261017/godot-sources.json)
record the exact upstream commits, paths and downloaded content hashes.

## Pinned sources and useful differences

| Source | Relevant mechanism | Incant consequence |
| --- | --- | --- |
| [Godot loop](https://github.com/godotengine/godot/blob/81ab961f91066a63bcbf3ec340962bb1a1ada558/main/main.cpp#L4997) and [timer synchronization](https://github.com/godotengine/godot/blob/81ab961f91066a63bcbf3ec340962bb1a1ada558/main/main_timer_sync.cpp#L419) | Fixed physics steps are separate from drawing; interpolation is a separate concern. | Test accepting every due simulation step while presenting the last accepted state once. This experiment does not implement interpolation. |
| [Godot Metal surface](https://github.com/godotengine/godot/blob/81ab961f91066a63bcbf3ec340962bb1a1ada558/drivers/metal/rendering_context_driver_metal.cpp#L107) | Drawable acquisition and presentation are explicit lifecycle operations. | Measure them separately from simulation. Incant's prior roughly 0.03 ms acquisition p95 does not identify acquisition as its bottleneck. |
| [Bevy TAA](https://github.com/bevyengine/bevy/blob/85503941e488152711a4695bbe248e9ed9f453fc/crates/bevy_anti_alias/src/taa/taa.wesl) | Filtered history and neighborhood rejection; output alpha remains current-frame alpha. | Useful reconstruction structure, but not a direct solution for Incant's separate coverage layer. |
| [Filament TAA](https://github.com/google/filament/blob/9680e59f3fae866ef8f801524e77dd6fbbf760d3/filament/src/materials/antiAliasing/taa/taa.mat) | Current reconstruction, history filtering and color-space neighborhood constraints are explicit shader operations. | Keep color filtering separate from geometric history validity instead of tuning a single nearest-history blend. |
| [FSR2 depth clipping](https://github.com/GPUOpen-Effects/FidelityFX-FSR2/blob/1680d1edd5c034f88ebbbb793d8b88f8842cf804/src/ffx-fsr2-api/shaders/ffx_fsr2_depth_clip.h) and [lock rejection](https://github.com/GPUOpen-Effects/FidelityFX-FSR2/blob/1680d1edd5c034f88ebbbb793d8b88f8842cf804/src/ffx-fsr2-api/shaders/ffx_fsr2_postprocess_lock_status.h) | Reprojected finite history locks coexist with depth, shading and reactive rejection. | Test whether missed thin geometry needs separately qualified temporal support. FSR2's history alpha is not Incant's coverage alpha. |

## Current observed results

The [batched-presentation diagnostic](native-batched-presentation.md) preserves
the complete Core gameplay report and offline PCM. Its first ordinary native
timing arm loses focus twice and remains failed. The subsequent quiet three-arm
comparison completes without focus loss: ordinary and display-link-only interval
p95 are 16.667 ms, while latest-batch reaches 25 ms. Its strict capture receipt
also fails on omitted idle-wake metadata and an initial-timestamp assumption;
independent saved-data analysis retains that failure and includes all role
transitions. Most long intervals end at two-tick batches. The next scheduling
hypothesis gives simulation its own rational deadline instead of waiting for
display callbacks. The earlier display-link-only comparison is not overwritten.

The [local-pose interpolation adapter](native-pose-interpolation.md) now separately
passes scoped CPU and actual Metal correctness tests. It retains fixed simulation
states and interpolates a temporary render hierarchy, with explicit continuity
resets and one fixed step of visual latency. Window integration, animated/UI
support and matched shipping-performance qualification remain open.

The separate, unmerged GPU color reconstruction candidate at `18aa76bb` uses
current-sample reconstruction, filtered history, color-space variance clipping
and depth/motion rejection. Its small actual Metal raster fixture passes
registration, object/camera motion, depth and camera-cut checks. Independent
analytic slanted-edge mean absolute area error improves from
`0.020373106944628603` on the first phase to `0.015446297177053528` on the ninth
same-phase frame. This is a small-scene numerical result, not visual acceptance.

That fixture also exposes a specific failure: all 49 analytically covered thin
strip pixels disappear on six of eight jitter phases, in both tested cycles.
The low-resolution raster has no strip samples in those phases. The resolver's
mandatory current support and zero current color/alpha bounds erase history.
Earlier full images still fail visual review; they are not excused by the edge
metric.

The subsequent scoped hypothesis carries observed coverage for at most one eight-phase
cycle only when accepted and candidate camera, geometry, placement, materials and
actual packed lights are unchanged. It must improve full-cycle alpha-area error,
not merely leave nonzero pixels. Movement, removal, camera changes, occlusion,
lighting changes, cuts and failed preparations must not preserve stale coverage.
This conservative experiment fails its area-error criterion: it retains thin
geometry but worsens slanted-edge error by 1.26%, introducing 148 changed pixels
whose analytic geometry coverage is zero. Its raw input planes remain identical.
The failed output remains retained and the candidate is not adopted.

The next test isolates visibility instead of changing color reconstruction:
output-resolution four-sample owner, depth and prior-motion observations, with
hardware sample positions calibrated rather than assumed. CPU shader/oracle
controls and strict lint pass; actual Metal results remain pending. Four samples
still cannot guarantee arbitrary thin geometry, and the additional memory and
render cost must be measured before considering integration. FSR2 thin-feature
locks help identify possible history support but do not prove current visibility.

No experiment changes ordinary exports or establishes the rendered Core 60 fps,
ScriptTick 2 ms, appearance or phase gates. The source comparison replaces blind
parameter tuning with falsifiable changes; it does not lower those requirements.
