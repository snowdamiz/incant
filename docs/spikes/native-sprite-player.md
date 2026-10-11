# Source-free native sprite player

The existing `native-game.py --gpu` proof now exercises both original v4 model
scenes and v5 atlas sprites through the same fresh executable build. Public
CLI imports cook a small numerical PNG and typed stable-frame atlas; shared
commands create the entities, and the existing cooker writes checked binary
scenes. No project mutation or alternate runtime path was added.

The sprite fixtures use nearest-sampled simple geometry and a mirrored,
linear-sampled nine-slice, each coupled to the same input, embedded script,
kinematic body and static/streamed spatial audio. The final observation now
reports sprite counts and committed positions alongside the existing model
fields; it is not a per-tick JSON path. The retained package includes only the
checked KTX2 and atlas metadata, not the imported PNG or atlas source.

## Executed result

One initially empty shipping build produced a **7,404,688-byte** macOS executable,
SHA-256 `26ddc9a5dcce54029c8e5d4c81941e759909721d9aed2f70df324c22addcd026`.
The separately packaged arm64 dSYM matches UUID
`d36d0db9-9c0f-3449-936f-104f20a80e2b`. Actual backend: Apple M5 Pro / Metal,
macOS 26.6 (25G72), Rust 1.99.0. Existing fresh compiler/codegen/normal-graph and
source-compiler-symbol audits remain unchanged.

All **40 executable invocations** pass after project/source/build deletion:

- Sixteen successful 120-tick motion/audio/GPU runs: four fixtures, two actual
  output sample rates and two fresh processes per case. Repeated complete
  reports match exactly. Both sprite modes show 420 non-background pixels in
  each first/final numerical capture, with different hashes under motion.
- Nineteen rejection cases, retaining all previous v4/input/script checks and
  adding default no-host refusal, missing sprite cache and an unknown atlas
  frame. The last case changes the one frame ID and recomputes the binary
  envelope checksum to test the actual binding error, not generic corruption.
- Two ordinary v1 controls and two explicit CPU-only controls within the GPU
  build, one each for model and sprite scenes. CPU sprite pose/PCM agrees with
  the original model simulation without initializing the GPU path.
- Existing output-file preservation.

Package hashes use relative paths, so different subdirectories cannot hide a
changed file behind a duplicate basename. All runtime package hashes are stable
through the repeated runs. The original v4 controls and one-build/source-removal
requirements remain intact; adding sprite coverage does not add another cold
shipping build to CI.

Release all-target player Clippy, 40 tools tests, formatting and diff checks pass.
Independent Astra read-only review found no blocker. Full sources and observations:
[evidence](evidence/native-sprite-player-2026-10-10.json). Raw package, matching
symbols, audits and process logs: `/tmp/incant-native-sprite-player-shipping-final/`.

## Scope

These are actual native-process path and deterministic-output checks. Nonblank
moving captures are not an independent nine-slice sampling oracle or appearance
approval; existing numerical renderer regressions and Claude review cover those
separately. No screenshot is produced. Native window presentation, pixel-placement
migration, shader cooking/prewarming, real-phone performance and phase gates
remain separate. This proof deliberately uses the reviewed renderer/game source
without the concurrently developed native-window or static Wasm follow-ups.
