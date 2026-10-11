# Native retained sprites and atlases

Sprites now cook into native binary scenes and render from retained GPU resources.
Version 5 adds typed texture and atlas-frame identities plus numeric sprite
configuration. Scenes without sprites retain their existing version 1–4 encoding.
Unknown tags, invalid flags/numbers/references, truncated records, ambiguous
mesh/sprite bindings and a version-5 payload without sprites fail explicitly.
Cold sprite configuration is boxed so unrelated spatial nodes and structural
commands do not carry its full inline size.

The native input adapter loads sealed color textures once, resolves stable atlas
frames from checked cooked metadata, and validates actual region/nine-slice
bounds before publishing input. Missing frames, wrong texture usage or extents
leave the previous adapter unchanged. Configuration changes use the existing
atomic `SetSpatial` command and prepare a replacement. Ordinary motion copies
committed world poses into retained slots; it performs no file reads, project
cloning, JSON work, atlas lookup or tessellation.

The renderer preserves deterministic layer/order/ID sorting, nearest/linear
sampling, tint, flips and nine-slice geometry. Startup caches local vertex
positions. Each motion update transforms those positions into reserved CPU
storage, encodes successfully, then writes and submits private GPU buffers.
Separate live/staging buffers preserve accepted output after ordinary rejection.
Ordering, geometry or exact texture-version changes require replacement; they
cannot silently reuse incompatible buffers.

## Executed checks

On Apple M5 Pro / Metal, 123 scoped ordinary release tests/doctests and all 60
renderer GPU tests pass, followed by scoped all-target release Clippy and normal
authoring-free dependency checks. Independent nonvisual review found no blocker.
Machine-readable [evidence](evidence/native-sprites-2026-10-10.json) records exact
source and log hashes.

New tests exercise version-5 canonical round trips, binary atlas identities,
every truncation, tag/flag/numeric rejection, atomic structural failure, actual
authoring cook and source/cache-deleted retained CPU binding. A native binary
world with three sprites, two atlas frames, nearest/linear sampling, flips and
nine-slice submits eight moving frames that exactly match freshly prepared output.
The fixture includes inherited movement, reflection/scaling and an ordering
change requiring exactly one replacement. A rejected target after staging
preserves both accepted GPU output and the CPU adapter tick before retry.
Another test measures zero allocations across 64 sprite CPU staging calls;
initialization, ECS ticks, encoding, GPU work and assertions are excluded.

## Memory and remaining work

The existing 128 MiB sprite vertex limit is **per scene**, not a total retained
memory budget. At that cap, the current double-buffer implementation can retain
576 MiB of vertex-related state: two 128 MiB GPU buffers, two 128 MiB CPU update
vectors and two 32 MiB f64 local-position arrays. Textures, decoded assets,
metadata, replacement candidates and preparation peaks are additional. These
upper bounds are not a phone-memory result. Reducing duplicate CPU geometry and
measuring shipping memory against the plan's device budgets remain required.

This establishes numerical native sprite correctness and retention, not the
complete 2D feature set or a performance gate. Tilemaps, flipbook playback,
Rapier 2D integration, camera-relative pixel placement, known extreme-magnification
sampling limits, a standalone rendered sprite player, appearance review and
reference-device measurements remain open. Existing authored pixel-perfect
camera behavior is preserved; this does not claim the separately specified
camera-relative placement contract is implemented.
