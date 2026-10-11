# Source-free native game offscreen GPU player

The existing `incant_game_player` has an opt-in `render-gpu` feature and explicit
`--gpu-offscreen MODEL_CACHE TEXTURE_CACHE CAMERA_HEX` suffix after its existing
eight arguments. This uses the reviewed `NativeRenderHost<GpuRenderConsumer<_>>`;
there is one native world, one CPU adapter and one actual retained renderer.
The default build/invocation remains nonvisual and strict. `render-gpu` also
retains the separate `--cpu-render-input` mode without initializing a GPU there.
No build-script or bytecode loading/trust API changed.

The private target provider owns a 160×96 RGBA8-sRGB texture and a 768-byte-stride,
73,728-byte readback buffer. Every accepted tick submits through the real GPU
consumer. Only the first and final accepted frames are read back, with a bounded
10-second device-poll wait and 10-second callback wait. Readback bytes are hashed
and discarded; no image or screenshot is produced. Offscreen frame acceptance
is not OS/surface presentation. Target/readback failure occurs inside mandatory
host sync, so it destroys the composite and clears offline PCM; submitted GPU
work cannot be undone. The complete renderer still allocates bounded frame
resources; this is not a zero-allocation pipeline or a frame-budget result.

## Fresh executable evidence

`python3 tools/probes/native-game.py OUT --gpu` keeps the public import/command/
cook route, one fresh owned shipping target, exact rustc flag and symbol audits,
matched crash symbols, and deletion of authoring/source/build storage before
launches. Its v4 fixture has an imported glTF triangle, independent selected
camera, typed input, embedded QuickJS, kinematic physics, and actual static or
streaming PCM mixing. Camera placement for this numerical proof is eight metres
in front of the geometry; the audio listener remains independent.

All 28 launches pass: eight successful 120-tick runs (two audio modes, two sample
rates, repeated processes), sixteen failure/capability controls, two plain v1
controls, one explicit CPU-mode control within the GPU build, and preservation
of an existing output file. Every successful GPU run reports 120 accepted
frames, 55 non-background pixels in both bounded readbacks, and distinct first/
final frame hashes under motion. Repeated process reports, including GPU hashes,
PCM hashes and continuous committed poses, match exactly. A late selected-camera
despawn rejects without a success report or output file. A separately built
default-feature executable also refuses v4 and both unavailable spatial modes.

Actual device/backend: **Apple M5 Pro / Metal / IntegratedGpu**, macOS **26.6
(25G72)**. Target: `aarch64-apple-darwin`, Rust **1.99.0**, wgpu **29.0.4**,
rquickjs **0.14.0**, QuickJS-NG **0.16.2 / bytecode 28**. The actual packaged
executable is **7,388,160 bytes**, SHA-256
`23e0f27152263f37685e0e78898989df2d3c8748b33b3b2752dcf5427fd7cfa9`.

The player and its owned runtime/render dependencies are inspected in real
fresh target invocations: optimization 3, one codegen unit, panic abort, fat LTO
on the executable and portable CPU selection. QuickJS source compiler symbols
and source comments are absent; all six dynamic source compilation routes reject.
Normal graphs for default/CPU/GPU modes exclude authoring/importers/compiler;
only the explicit GPU feature includes wgpu. Build-only cooking remains separate.
The existing host==target embedding limitation is unchanged.

Integrated validation also passes 34 ordinary CPU-host tests/doctests, all three
actual GPU-host regressions (including target failure/unwind and observed device
loss), release all-target Clippy for all three player configurations, 40 tools
tests, formatting, conventions, dependency checks and actionlint 1.7.12. The GPU
core is identical to reviewed `19e502c`; this proof deliberately does not pull in
the concurrently developed native v5 sprite slice.

Full evidence: [result and source identities](evidence/native-game-gpu-player-2026-10-10.json).
Raw executable, symbols and build/process evidence: `/tmp/incant-game-gpu-shipping/`.
Logs: `/tmp/incant-game-gpu-{player-clippy,union-tests,host-tests,shipping,tools}.log`;
normal graphs: `/tmp/incant-game-{plain,cpu,gpu}-graph.log`.

## Limits

This is numerical native-process evidence, not visual approval, native surface
ownership, mobile execution, phone performance or a whole-engine migration gate.
Materials, native 2D, pixel placement, renderer memory/frame budgets and full
session restoration have separate work. The existing renderer still creates its
WGSL pipelines at runtime; offline shader compilation/prewarming is not certified
by deleting project and script sources. No signing/export authentication claim is
made. The bytecode hashes detect identity/integrity; trust comes from the fresh
compiler output embedded by the trusted build, not the hashes.

## Integrated renderer and sprite-storage rerun

The integrated source tree at `d56e74b` passes **539 ordinary release tests and
doctests**, **66 actual GPU tests**, workspace and optional-player all-target
release Clippy, and the combined native editor/headless/tool build. The optional
development Wasmtime host's 13 tests passed before the subsequent renderer-only
immutable sprite-storage change; its sources are unchanged.

A new initially empty shipping build also passes the same **28 process
invocations** after deleting the project, imported sources, script source and
build storage. Its Apple M5 Pro / Metal player is **7,404,688 bytes**, SHA-256
`b884b7514a71f18d4f9f7d30d8c1138af86e0b040d187e7d65de14eb57b9e582`,
with matched arm64 dSYM UUID `11d4f0fa-9490-300b-ba59-36238e4dafb0`. The first and
final numerical frame hashes, PCM hashes and poses match the earlier proof.

This executable contains the integrated v5 sprite reader and retained geometry
implementation, but this particular process fixture still exercises the v4
model path. Native v5 sprites are covered by the separate exact numerical GPU
regressions, including the measured storage reduction; this rerun does not
substitute for a future v5 source-free executable fixture. It does not include
the separately developed native-window host.

See [integrated result and check identities](evidence/native-runtime-integration-2026-10-10.json).
Raw package, matching symbols, audit and process logs are retained in
`/tmp/incant-native-final-gpu-shipping/`.
