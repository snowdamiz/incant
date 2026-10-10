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
runtime/script state and unchanged authored files. The remaining three editor/headless GPU tests also pass (44 total). The final
headless executable was frozen after Cargo finished and the public workflow
passed again against that exact copy. Native/target builds and Claude visual
review remain in progress. WASM and iOS-simulator core compilation pass; these
are compilation checks, not live device or browser-renderer acceptance.

This is a projection foundation. Pixel-perfect snapping, sprites, tilemaps and
2D physics are not supplied by this increment. No device or phase gate is claimed.

## Integrated Inspector and rendered review

Claude Opus 5.5 through ACP supplied the Camera Inspector and reviewed an
original real-engine scene. Schema metadata now supplies metres/degrees, explicit
projection order, user-facing help and the perspective default. The bridge
preserves default metadata through references/unions without authoring it. Legacy
cameras show a marked default; orthographic FOV is visibly unused only for a
well-formed projection. Tiny positive values remain visible as `1e-6`, with exact
values available on hover. The Inspector remains read-only.

At integrated source `59f3384`, 318 Rust tests, full Clippy, 387 UI tests/build,
five tool tests, generated contracts and native release/package pass. A frozen
binary passes the public authored Camera save/reopen/repeat workflow. Renderer
source and tests are unchanged from the original 44 passing GPU checks; this is
retained coverage rather than another combined-head GPU run.

Astra independently reproduced Claude's numerical report, verified all 18
committed capture hashes, and compared 72 images byte-for-byte across the two
independent original runs. Orthographic pillar silhouettes agree with analytic
projection within 0.69 px, preserve vertical framing at two aspects, and match
static/live switched captures. The ten final native CUA captures cover legacy,
explicit perspective, orthographic, tiny values and keyboard focus at 1440×874
and 1000×650, including a 280 px Inspector. Claude accepted the final native scope in `7924ff3`; all Camera labels, tags,
units, tiny values and minimum-width keyboard focus pass. Full account-bearing JPEGs stay ignored; only account-free Claude crops
may be committed. The OS sharing pill covers traffic lights in these captures.

The initial disposable fixture under Documents stalled in macOS `__open` and
reached the existing load timeout. Relocating the byte-identical app and fixture
to development artifacts outside Documents opened it immediately with zero
engine diagnostics, without changing OS permissions. This does not prove the
underlying OS cause or alter the loader timeout.

Known shadow limit: pulling an orthographic camera farther back moves receivers
through depth-based shadow cascades. In Claude's measured comparisons, up to
4.2% of pixels changed on shadow edges/self-shadow texture; unshadowed lighting,
parallel specular response and silhouettes stayed unchanged. Distance-invariant
shadow quality would require a separate receiver-depth fitting change.

The final combined source also compiles for WASM and iOS simulator core targets.
These do not count as physical-device or live browser rendering gates. Native
Camera selection and GUI field mutation remain outside this read-only increment.
Required hosted checks must pass before merge.
