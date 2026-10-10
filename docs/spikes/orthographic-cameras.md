# Orthographic cameras

Authored `Camera` components accept a `projection` tagged union: `perspective`
(the default for legacy documents that omit the field) or `orthographic` with
positive finite `vertical_size` in world units. Width follows viewport aspect.
The existing `fov_degrees` remains required and retained while switching modes;
it affects only perspective projection. Near remains positive and far greater
than near, preserving the shared logarithmic light-depth partition.

All mutations use the shared command bus and preserve atomic validation,
provenance, persistence and Undo/Redo. Unknown projection kinds and fields fail.
The renderer rejects unrepresentable GPU extents before encoding work. Geometry,
diagnostic rendering, local light clusters and directional shadow fitting use
the selected projection; orthographic material shading uses parallel view rays.
Retained scenes and queued commands retain their camera parameters through later
projection changes and viewport resizing.

The command regression covers legacy defaults, malformed projection rollback,
provenance and exact history. CPU tests verify world extents, clip depths,
parallel rays and directional-cascade receiver coverage. Real GPU checks pass
constant world size across camera distance, retained projection, explicit near/
far clipping, specular-light invariance, clustered/all-light equivalence across
four poses and two aspects, offset/queued viewports, cascade transitions and
mixed-projection queued shadows. All 41 incant_render GPU checks pass locally.
All 303 Rust tests, full Clippy, 315 UI tests/build, five tool tests and generated
contracts pass. The public strict-TypeScript workflow passes authored legacy
cameras, ordinary projection commands, a mid-run save/reopen, exact final/repeated
runtime/script state and unchanged authored files. Remaining editor/headless GPU,
native/target builds and Claude visual review are in progress.

This is a projection foundation. Pixel-perfect snapping, sprites, tilemaps and
2D physics are not supplied by this increment. No device or phase gate is claimed.
