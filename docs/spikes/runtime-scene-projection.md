# Runtime scene projection

PR #9 merged into main as `331a3cb` on 2026-10-09 after all twelve hosted checks
passed on `97cf521`. This completes the scoped ECS projection increment, not the
Phase 1 engine or its release gate.

The Phase 1 ECS projection now retains local rotation, scale, parent references
and typed `MeshRenderer` bindings. Parent-first matrix composition supplies initial
and fixed-step world transforms, including parents whose IDs sort after children.
Velocity remains parent-local. Scenes stay independent and play never modifies
the authored document.

Synchronization validates and decodes the whole candidate before changing live
entities, then reuses the Bevy world, schedules and retained entity IDs. Deletion,
scene moves, component removal and reparenting are reflected together. Invalid
topology or overflowing composed transforms leave the previous projection intact.
Changing the tick rate preserves accumulated elapsed time. Model and material
references must identify the corresponding asset kinds.

The existing diagnostic renderer consumes these world transforms. The native GPU
probe compares a rotated/scaled parent-child scene with its equivalent flattened
scene and requires identical PNG output, different from the original scene. This
is a mathematical rendering check; it is not a Claude appearance approval.
The same probe is invoked by Windows/Linux desktop CI.

Five new scene behavior tests cover composition, local motion, independent scenes,
reparenting and binding removal, failed synchronization, a 512-node hierarchy,
entity replacement, tick-rate changes and numeric overflow. A document test covers
asset-kind mismatches. Existing script sandbox and isolated-play checks remain
applicable. All 108 workspace release tests, workspace Clippy, generated contracts
and five tooling tests pass locally. The Apple M5 Pro GPU equivalence probe passes.

The local thousand-entity workload issues 120,000 script commands over 120 ticks.
An earlier alternating comparison measured 17.49–19.00 ms p95 for the previous
implementation and 21.68–23.85 ms for the new projection. A later comparison measured
11.42–13.69 ms for the previous implementation and 11.53–12.90 ms for the new one.
Both versions varied substantially on this actively used Mac. An instrumented
driver measured roughly 0.15 ms additional median synchronization cost. Preserve
all runs rather than attributing their variation to a proven cause. The later
script-only runs pass 16.667 ms p95; sustained rendering/game frame time is unproven.
The final build with the numeric-overflow guard measures 11.66 ms p95.
[Measurements](evidence/runtime-scene-2026-10-09.json) retain the comparisons,
executable digests and scope.

Imported GPU geometry/material binding, lighting, active-scene/camera selection
and GPU hot reload remain open. The viewport still draws diagnostic cubes.
Arbitrarily large velocities can overflow on later simulation ticks; rejecting
an overflowing initial/synchronized hierarchy does not solve that existing
simulation-range limitation.
