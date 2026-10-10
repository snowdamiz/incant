# Compound colliders — in progress

The authored Collider union now includes one-level compounds of 1–64 local
primitive parts. Parts retain stable ULIDs and their own translation/quaternion,
while the parent entity supplies one body, material, sensor state and collision
masks. Nested compounds fail explicitly. Existing primitive documents remain
compatible. All authoring still uses the shared atomic command bus.

Preparation sorts parts by stable ID without changing authored presentation order.
Reordering parts therefore preserves live handles, contact state and sleep rather
than rebuilding the body. Runtime raycasts and sensor events identify the parent
entity. Character bodies must remain primitive; compound obstacles are supported.
The character-only box-face normal correction also resolves faces inside compound
parts, preserving continuous motion across their planar surfaces.

Initial checks pass 168 Rust tests. New behavior tests cover rotated parts and
hollow openings, dynamic contact and exact future-state equality while reordering,
one enter/exit event across overlapping sensor parts, masks, explicit capability
failure and atomic command rejection with Undo/Redo/journal recovery. The expanded
walking regression covers 24 combinations, 300 ticks each. Full visual/native,
public CLI, performance and hosted verification remain open.

Claude handoff 0024 owns the object-array Inspector and real rendered look-dev.
Mesh terrain, hierarchical body ownership, local material/mask overrides, nested
compounds and physical-device game performance remain outside this increment.
