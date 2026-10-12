# Public root-motion Rust gameplay package

The ordinary authored Wasm route now passes a source-deleted root-motion game
proof on `aarch64-apple-darwin`. Public imports capture a real eight-clip glTF
model and Rust SDK module. One command transaction creates the scene, selects
the module and binds input actions; Undo, Redo, project save and the editor-default
journal path reopened through public RPC with `--journal` preserve the exact
document. Original model/module sources and the guest build are removed before
development play, native-oracle cooking and export.

Four owners exercise bodyless Transform, inactive ExtractOnly, collision-limited
Character3d and velocity-driven Kinematic3d application. Distinct LINEAR/STEP
track durations, antipodal quaternion signs, reverse multi-cycle playback,
paused clocks, state/parameter changes, legacy spherical pairs, nested weighted
trees and actual compound 16-pose fades all run through the captured scene.
Application toggles, accumulation resets and ordered IK updates remain typed
staged controls. A blocking wall produces a measurable requested/applied motion
difference. A separate CUBICSPLINE import refuses with its exact diagnostic and
leaves the project unchanged.

The independent oracle loads captured project resources, cooks the scene and
executes typed `NativeWorld` updates. It never reads guest bytes or the copied
ABI parser. Each tick preserves 702 scalar fields: complete pose/graph/IK
operands, root identities and fingerprints, exact clock words, four rigid motion
receipts, and body transforms/velocities. The real guest logs 26 records of 27
scalars per tick. All 22,464 values across 32 ticks match the oracle within
`1e-12`; exact integer words preserve signed cycles and ticks. There are also
256 selected public vector checks within the unchanged observation bound.
The oracle shares the canonical native solver; separate primitive, interval,
rollback and allocation tests establish its mathematical/runtime contracts.

Captured-only Wasmtime development play passes before the expensive export.
The normal exporter translates the captured prepared module with pinned wasm2c
1.0.42 and generates its ordinary package binding. The cooked scene hash exactly
matches the oracle input. After removing this proof's project/cache, translator,
private export build and source maps, three fresh `--package` processes match
development records, public observations, frames, tick and PCM hash exactly.
No static-module fixture override is used. The 10,150,976-byte native player has
SHA-256 `1c3045e0337618cad4a72c2f54645cff7664e4458434bb7e873b0f5bbee1d609`.
Child processes receive only the package path, temporary paths and required
Windows platform keys when applicable; `HOME` is omitted.

Five negative replays on both development and shipping paths exercise old
complete pose-v5/IK-v2 downgrade, short scratch, a late missing entity and an
invalid root owner. Each requires its exact diagnostic, exit 1 and no stdout or
PCM output. The same input replay route first succeeds on the positive fixture,
preventing a missing flag/file from masquerading as refusal. A further ordinary
checkpoint request reports `Unsupported("static module state ABI")` with neither
PCM nor save output. These eleven checks prove the public no-output boundary;
full staged-prefix rollback and all seven old pose/IK reader pairs are covered
by the separate actual-host suite.

The final producer audit rehashes 1,693 unchanged engine/tool inputs, all nine
probe inputs, fourteen package files, three actual report/PCM pairs and 31 command
stream receipts. The durable packet retains 147 exact UTF-8 logs/receipts in a
lossless gzip JSON archive. Failed preliminary runs remain: the initial positive
CUBICSPLINE fixture, an over-budget guest import set, and insufficient reverse
wrap coverage each failed before the final successful run. No engine limit was
relaxed. Independent final package source and evidence review is accepted
without findings; the [retained receipt](evidence/native-root-motion-package-final-review.json)
records direct verification of the surviving sources, package and observations.

Reproduce with a Wasmtime-enabled `incant_headless` and the cooked oracle example:

```sh
./tools/cargo build --locked --profile ci -j2 -p incant_headless --features wasmtime-host
./tools/cargo build --locked --profile ci -j2 -p incant_cook --example root_motion_package_oracle
python3 tools/probes/native-wasm-root-motion-export.py "$proof_dir" \
  --binary "$headless" --oracle "$oracle" --stage author
python3 tools/probes/native-wasm-root-motion-export.py "$proof_dir" \
  --binary "$headless" --oracle "$oracle" --stage export
```

[Exact sources, commands and retained evidence](evidence/native-root-motion-package.json)
bind run `root-motion-package-004`; result SHA-256 is
`76497d629e59b7fe3495cb4dbb02b54fb1e1e5619f5624f7e7399b04aeae9ea4`.
Only newly created private proof inputs were deleted; the engine repository,
toolchain, retained receipts and earlier failed-run inputs remain. This is local
CPU render-input preparation, with no GPU/window or rendered-pixel acceptance.
Physics and arbitrary guest checkpoints, general affine/CUBICSPLINE roots,
reference-phone performance and the complete Phase 1 gate remain open.
