# Physics snapshot boundary diagnostic

This standalone CPU diagnostic answers two limited feasibility questions using
pinned Rapier 0.36.0 / Parry 0.31.1 with the native deterministic parallel mode:

1. Does copying the full **trusted typed persistent state** into a fresh pipeline
   retain subsequent solver state, native-style overlap events and handle order?
2. Does rebuilding the same current poses, velocities and sleeping flags through
   public constructors have the same continuation?

The first route is not a persisted save loader. It accepts no input file, does
not deserialize Rapier data and does not validate a hostile graph. The second
route is deliberately named `pose_reconstruction` and is a lossy comparison,
not an adopted ordinary-load policy. Neither route is exposed to engine code.
The production `PhysicsUnsupported` guards remain unchanged.

Run from the repository root, using a private Cargo target:

```sh
CARGO_TARGET_DIR=/path/to/private-target cargo run --locked --offline --release -j2 \
  --manifest-path tools/probes/physics-snapshot-boundary/Cargo.toml
```

The JSON result reports 2D and 3D runs at four checkpoint boundaries, each with
120 further ticks. It compares complete diagnostic serialization, body/shape
handles, published poses and velocities, overlap identities and entered/exited
notifications. A two-thread Rayon pool matches the selected parallel mode.
The fixture contains a compound rectangular ground, a stack, circle/sphere and
capsule, a kinematic body, a sensor, and removed/reused arena slots. Rectangular
compound geometry exercises the tile-box shape representation; it is not an
Incant tilemap-cooking test. Explicit post-boundary motion, removal and reentry
exercise overlap history. No visual, GPU, device, shipping-profile performance
or source-deleted process claim is made.

Serialization is diagnostic-only JSON produced from trusted finite state; it
is never read back. A complete-byte comparison is useful for hidden ordering,
but this cloning experiment does not prove that dropping every serde-skipped
workspace on a binary restore is equivalent. The native adapters and ordinary
session save composition are also outside this diagnostic.

The isolated Cargo lock retains every shared dependency version/checksum from
main163. Enabling upstream serialization adds only `serde_arrays 0.2.0`; no
engine Cargo manifest, lockfile or numerical feature is changed.
