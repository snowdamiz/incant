# Scheduled render passes

The native renderer prepares owned frame packets and executes the real GPU
passes through a persistent Bevy ECS schedule. This replaces the inline encoder
sequence. It is the scheduling and resource-lifetime increment of Phase 1;
shadows, SSAO, bloom, TAA, mobile feature tiers and the full production graph
remain open.

## Execution and ownership

The pass dependencies are explicit: scene initialization/diagnostic geometry and
local-light assignment both precede imported geometry; display composition
follows geometry. The initializer always clears HDR color and depth, including
model-free frames. The light pass runs only for clustered local lights. Model
geometry preserves the existing opaque/transparent ordering and now stores depth.
Display composition retains the existing tone curve, transfer and chrome mask.
There are no shader or appearance changes in this increment.

All fallible CPU preparation finishes before the graph receives a frame. Owned
packets retain immutable camera/settings buffers, light bindings, pipelines,
attachment views and exact model versions. The single-threaded schedule records
one encoder; it does not submit or present. `Renderer::draw_scene` still returns
one command buffer and the caller owns submission. The graph removes its frame
and encoder resources after each run. Encoded wgpu commands retain the GPU
handles they need without retaining the CPU scene or cooked-asset graph.

One cached attachment size and one cached light-grid shape bound idle retention.
A frame being encoded or a command buffer awaiting submission can retain older
resources; this is necessary for correctness and is not a cap on caller-owned
queued frames. Per-frame uniforms remain immutable. Every used cluster word is
rewritten before the geometry consuming it, even when command buffers are
submitted in a different order from encoding. No cross-frame uniform queue
writes or scene borrows enter the scheduler.

## Bevy integration boundary

The pinned Bevy 0.19.1 renderer implements its render graph as an ECS schedule
with ordered system sets ([primary source](https://docs.rs/crate/bevy_render/0.19.1/source/src/renderer/mod.rs)).
Incant uses the same pinned `bevy_ecs` scheduler directly, with its own pass sets,
prepared resources and existing explicit wgpu device/queue ownership. It does
not install `bevy_render::RenderPlugin`, import Bevy's window/swap-chain driver,
or claim integration with Bevy's default Core3d pipeline. This is a custom
schedule-based graph, not an implementation of a legacy Bevy node-graph API.

The graph is currently internal and fixed. Dynamic pass registration, automatic
resource aliasing, multiple views, parallel encoders, GPU timestamps and the
remaining production effects are not implemented. These boundaries remain open
rather than being represented by unused placeholder nodes.

## Verification

The existing HDR, material, lighting, camera, editor and playback GPU cases run
through the schedule. A new GPU lifetime case prepares four frames with three different imported
model versions at alternating sizes, records frames, injects rejected viewport
requests, drops the scenes/store/project and deletes their files. It verifies
that the graph retains no CPU frame or encoder resource, submits commands in
reverse order, and compares every output byte against its immediate reference.
Two stable local lights alternate between a nearby and offscreen position. The
last two frames use identical geometry, size and mask word count, so they reuse
the same cluster buffer while selecting different light bits. Their immediate
references must differ; all reverse-submitted frames must match their own
reference. A subsequent model-free frame must contain only the display background.

The combined checks pass 139 ordinary Rust tests, 33 GPU cases (6 renderer unit,
24 renderer integration, 2 headless and 1 editor), 283 UI tests, five Python tool
tests, workspace Clippy, generated/SDK checks and the final custom-protocol
native release build/package. The native app was attached with zero reported
errors before and after collapsing the output panel to enlarge the viewport;
CUA captured both states, restored the layout and closed the app.

Against exact baseline `83c98a7`, 120 of 124 named PNGs match pixel for pixel.
Four captures differ by one channel value in one pixel (513×385) or two pixels
(640×480). These are the 96-light fixture and its all-light counterpart; each
current clustered/oracle pair remains exact. The fixture creates fresh ULIDs,
so separate runs can sum the same lights in different floating-point order.
Claude's initial review `1494fda` approved all reviewed appearance and independently
reproduced the baseline bytes with the scheduled code in repeated processes.
Following that review, the 96-light fixture now uses stable ULIDs; two separate
runs produce exactly the same four PNGs. The expanded lifetime fixture now has
four lit captures with alternating local-light membership. Claude's final
review `92a4714` passes with no open pixel-evidence gaps; five independent
stability runs and two lifetime runs reproduce the final files. The original baseline comparison is retained as historical evidence;
no runtime/shader changes accompanied these test improvements.

Three paired trials at 1920×1080, five warm-up frames and thirty measured frames
per path/trial, use the identical cooked sphere/light-lattice projects from the
mask increment. The app and other builds/tests were closed during measurement.
Median of trial medians, in milliseconds:

| Local lights | Prior inline passes | Scheduled passes |
| --- | ---: | ---: |
| 0 | 1.394 | 1.397 |
| 64 | 1.393 | 1.404 |
| 256 | 1.406 | 1.414 |
| 1024 | 2.692 | 2.697 |
| 4096 | 6.611 | 6.506 |

These fenced CPU encode/submit/GPU-wait times show no material regression in
this workload. The dense-case difference is not claimed as an optimization;
trial variation and the shared GPU limit precision. They exclude asset loading,
readback, presentation and simulation and are not GPU timestamps. Raw samples,
PNG hashes, verification-log hashes and comparison details are in the
[evidence ledger](evidence/render-pass-graph-2026-10-09.json). No phase or named
game/device performance gate is inferred from these synthetic checks.
