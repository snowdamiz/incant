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
through the schedule. A new GPU lifetime case prepares three different imported
model versions at alternating sizes, records frames, injects rejected viewport
requests, drops the scenes/store/project and deletes their files. It verifies
that the graph retains no CPU frame or encoder resource, submits commands in
reverse order, and compares every output byte against its immediate reference.
A subsequent model-free frame must contain only the display background.

Final test totals, baseline capture comparison, timings and Claude's scoped
pixel review are recorded in the evidence ledger once completed. No phase gate
or game/device performance gate is inferred from these synthetic checks.
