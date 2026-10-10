# Cooked audio without authoring dependencies

Date: 2026-10-10. This extracts the existing PCM reader and Kira mixer into
runtime crates. It does not yet connect audio to the new native ECS or satisfy
the audio device, lifecycle, Core Sample or reference-phone gates.

## Boundary and compatibility

`incant_assets_runtime` reads the retained cooked PCM files and their bounded
manifests. `incant_audio_runtime` owns buses, voices, spatial tracks, the offline
backend and native streaming. Neither depends on the project document, command
bus, agent, importer, cook pipeline or legacy play session. JSON manifests are
decoded once at asset load; sample reads use retained typed metadata and file
handles. There is no per-tick project validation or JSON conversion in these
two crates.

The authoring crates re-export the extracted types. The project-to-mixer adapter
remains in `incant_audio`; its document and JSON failures now have a separate
`SceneAudioError`, while direct mixer calls return the runtime `AudioError`.
The existing headless player handles both. Import, source grants, reimport,
Undo/Redo and authoring provenance still use their existing command paths.
Cooked keys and file formats are unchanged, and Cargo.lock adds only the two
local packages and their edges; no external resolution changed.

The extraction retains the existing limits: 256 MiB PCM per clip, 512 KiB per
manifest, 8,192 frames per read, 128 voices, 32 buses and 8,388,608 resident static
frames. Each sample chunk checks its hash and finite values. Static frames are
shared and unused cache entries are reclaimed before admitting new ones.
Runtime structs are trusted Rust interfaces, not a security boundary against
callers that mutate public metadata after validation.

## Executed evidence

- The release workspace passes 412 tests; 44 unrelated GPU checks remain ignored
  in this ordinary run. Release workspace Clippy with all targets passes.
- Five new direct-runtime tests check source-free reads/seeks, bounded reads,
  retained-file corruption, malformed metadata/PCM, static and streaming output,
  seek/loop/rate behavior, and rejected mixer settings preserving valid playback.
  Existing authoring audio tests also pass through the new re-exports.
- The existing public strict-TypeScript/RPC gameplay audio probe passes through
  the compatibility adapter: pause is silent, resume restores the signal,
  -6 dB bus gain produces a 0.501125 amplitude ratio, saved timer deadlines work,
  repeated PCM is exact, and the authored project/journal stay unchanged.
- `tools/probes/cooked-audio.py` imports a stereo WAV and Vorbis clip through the
  public command-backed importer, builds a fresh shipping executable, packages
  matching symbols, and deletes the source project and fresh build directory
  before running the packaged player.
- All 64 fresh-process runs pass: two clips, static/streaming modes, four rates,
  two render batch sizes and two repeats. PCM is byte-identical across batches
  and process repeats for each clip/mode/rate. Static and streaming interpolation
  are not claimed byte-identical to each other. Finite samples, stereo phase,
  expected level and an unchanged cooked cache are checked. Invalid modes/keys
  and existing output paths reject without publishing or overwriting output.

The arm64 macOS executable is 585,072 bytes, SHA-256
`52125cdd829c2a9de317bcd9c6b18c43e62ef144138b10c393d8fb612330e246`.
Its matching dSYM UUID is `89fd95b9-7acf-3a74-b88d-4765946c0e39`.
The probe verifies actual fresh rustc invocations: opt-level 3, one codegen unit,
abort on panic, packed debug information and fat LTO on the executable. It audits
compiled targets as well as the normal dependency graph; the runtime player
compiles no importer, authoring document, command bus, agent or legacy player.
This executable is an offline mixer probe, not the complete shipped game.

Machine-readable result: [cooked-audio-boundary-2026-10-10.json](evidence/cooked-audio-boundary-2026-10-10.json).

## Remaining work

Native ECS audio declarations, cooked scene/asset bindings, typed script audio
commands and mixer synchronization still need migration. The old document-backed
audio adapter remains a compatibility path and must not become the new gameplay
path. Native editor controls, device playback/latency, mobile interruptions and
audio sessions, browser resources/autoplay, mixer cursor saves, dynamic bus
topology and phone measurements remain open. This verification creates no OS
audio device and sends no speaker output.

The macOS and iOS device-backend targets compile, as does the WASM target for
both the runtime and compatibility crates. These are compile checks, not device
playback or browser-resource evidence. The public legacy gameplay audio probe
passes. Final main integration and hosted checks remain pending.
