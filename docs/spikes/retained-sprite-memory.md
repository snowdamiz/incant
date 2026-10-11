# Retained sprite vertex storage

Retained sprite frames previously duplicated every 64-byte vertex on both the
CPU and GPU, plus each vertex's 16-byte local position. Ordinary motion changed
only 12 bytes of that vertex. The renderer now separates positions from the
52-byte immutable attribute stream, preserving shader locations, numeric
transforms, sorting, sampling and the existing 128 MiB logical geometry cap.

Accepted and staged scenes share attributes, texture batches and f64 local
coordinates. Each still owns independent mutable CPU positions and a GPU
position buffer. The staging scene reuses the first scene's validated geometry
without a second sort or tessellation. Rejection and replacement semantics are
unchanged. Normal authoring frames use the same two-stream pipeline.

| Vertex storage at the existing cap | Before | After |
| --- | ---: | ---: |
| GPU streams | 256 MiB | 104 MiB shared attributes + 48 MiB private positions |
| CPU update vectors | 256 MiB | 48 MiB |
| Retained f64 local positions | 64 MiB | 32 MiB shared |
| Total above | 576 MiB | 232 MiB |
| Raw queue upload per moving vertex | 64 bytes | 12 bytes |

The cap is 2,097,152 logical vertices. These values derive from stream layouts
and capacities at that cap: 288 bytes versus 116 bytes per vertex. They exclude
textures, decoded assets, batch/placement metadata, driver allocations, upload
staging, pending replacement scenes and temporary preparation allocations.
They are not process RSS, frame-time or reference-phone measurements. Smaller
scenes can have CPU vector capacity slack.

The source-free native sprite test measures actual GPU buffer sizes and CPU
vector capacities, counts shared geometry only once, and requires a reduction
greater than 50% even for its small unevenly sized fixture. It also verifies the
12-byte upload extent. Its eight numerical frames still exactly match fresh
preparation across inherited motion, reflection, nearest/linear sampling,
nine-slice, one ordering replacement and rejected-target retry. The existing
64-call CPU staging allocation test remains at zero.

All 27 renderer ordinary tests and all 60 native GPU tests pass on Apple M5 Pro /
Metal, followed by all-target release Clippy and formatting checks. Independent
Astra review found no blocker. [Evidence](evidence/retained-sprite-memory-2026-10-10.json)
records source/log hashes. No appearance or device-performance gate is approved.
