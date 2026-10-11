# Native save reader rejects unsupported planar solver state

The version-1 save reader previously rejected nested 3D physics declarations but
accepted a valid cooked scene containing 2D physics. It installed a new solver at
the saved tick without recovering contacts, sleeping state or trigger history.
The writer and in-place restore already rejected worlds that used either solver.
The reader now applies the same refusal to both dimensions.

The regression constructs valid static-collider and rigid-body planar scenes,
wraps each in a valid v1 envelope with a nonzero clock, and recomputes both
checksums. Loading must return `PhysicsUnsupported`; attempting an in-place
restore must leave the old world byte-identical and still able to step. The test
failed before the fix. All 102 runtime release tests/doctests across 16 groups
and all-target release Clippy pass afterward against exact PR81 source.

This closes a decoder gap; it does not implement physics or full-session saves.
[Exact source and execution identities](evidence/native-save-planar-refusal.json).
