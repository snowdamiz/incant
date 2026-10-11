# Native scene audio migration

Work in progress, 2026-10-10. Typed native audio declarations and cooking are
implemented; native mixer/clip binding integration is the next dependency. This
is not yet a complete native audio playback path or a phase gate.

Cooked scenes use version 3 only when audio is present. Transform/Velocity-only
version 1 and physics-only version 2 encodings are retained. Binary stable IDs
identify buses, sources, listeners and clips. The decoder checks canonical order,
counts, flags, numbers, references, bus cycles and whole-scene integrity before
constructing the world. Up to 32 buses, 128 sources and one listener are accepted.
Standalone cooking refuses cross-scene bus/listener dependencies rather than
silently dropping the omitted scene. Authored projects remain unchanged.

Audio declarations live in actual ECS components. A bounded typed iterator exposes
committed derived poses; the single listener can compose ancestor rotations
without scale/shear or allocation. SetAudio changes use the runtime structural
queue, validate the entire final audio graph before writes and preserve rejected
queues/world state. Audio-only batches inspect at most 161 nodes without cloning
the full parent hierarchy. Mixed changes retain existing atomic numeric/physics
validation. Numeric loan layouts are unchanged.

Save/restore refuses worlds that have ever installed audio: playback cursors are
not yet serialized, even after the declarations are removed. The existing motion
and script probe executables explicitly refuse audio scenes until a host is
connected. Mixer/device handles and decoded clips do not enter the core runtime.

Six new behavior tests cover canonical/source-free typed cooking, malformed
binary/flags, missing/cyclic references, final-graph command rollback, monotonic
save refusal, scale-independent listener rotations and standalone scene bounds.
Scoped runtime/cook release tests and all-target Clippy pass. Full union checks,
mixer lifecycle/spatial/streaming tests, package proofs, native device/browser
execution and phone performance remain open.
