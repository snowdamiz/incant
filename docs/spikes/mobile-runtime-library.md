# Cooked mobile runtime library

The ordinary packaged player exposes the same execution path as both a CLI and a Rust library. The optional `mobile-ffi` feature exports `incant_game_run_v1` for native platform hosts. It executes build-linked gameplay against a cooked binary package; it does not load executable code from the request. Authoring, command-bus and agent crates remain build-only or absent from the runtime dependency graph.

## Request and outputs

The C header is `crates/incant_game_player/include/incant_game.h`. The caller supplies readable, immutable UTF-8 JSON bytes for the duration of the synchronous call, at most 65,536 bytes. A native host should use its own background execution thread; this function does not manage a UI thread, GPU surface or audio device.

```json
{
  "version": 1,
  "package": "/absolute/app/data",
  "output_directory": "/absolute/app/results/new-run",
  "ticks": 120,
  "input_recording": "/absolute/app/replay.json",
  "observations": "/absolute/app/assertions.json",
  "ui_viewport": [1280, 720]
}
```

Only `version`, `package`, `output_directory` and `ticks` are required. All paths must be absolute UTF-8, at most 4,096 bytes without control characters. Unknown fields are rejected. Tick count is 1–3,600; the existing player also caps total output at 1,000,000 stereo frames. Optional viewport dimensions are 1–65,536 and still require the existing cooked camera/UI host. Missing compiled capabilities fail explicitly.

The caller supplies an output directory that does not yet exist. A directory already present is refused before gameplay, including after an earlier failed run. The player retains its normal package hash checks, recorded input admission, native simulation, observations, 48-kHz PCM output and build-linked script sandbox. The request cannot select arbitrary CLI flags, window mode, save routes or external executable code.

`result.json` records success/failure, OS, architecture, panic strategy, debug assertion state, `execution: offline_cpu`, the ordinary game report when available, and an error when execution fails. The file is staged in the output directory, synced, then published without replacement. `audio.pcm` uses the ordinary player's output behavior: simulation failure publishes no PCM, while a completed failed observation can retain PCM and a failed game report. PCM and JSON are separate publications, not a multi-file atomic or universal power-loss durability promise.

Return codes: 0 passed; 1 the player failed (including its input/PCM I/O) and `result.json` was written; 2 malformed request; 3 output-directory/report I/O failure. An I/O failure can leave a reserved output directory or already-written PCM. The caller must not infer success from file presence alone. Invalid C pointers violate the caller contract; panic/OOM behavior follows the linked profile, including abort in shipping.

## Building

The normal package remains an rlib plus the CLI, avoiding a static archive on every ordinary build. A host integration explicitly asks Cargo for the archive:

```sh
CARGO_TARGET_DIR=/private/build/target tools/cargo rustc --locked --profile shipping   -p incant_game_player --lib --features mobile-ffi   --target aarch64-apple-darwin --crate-type staticlib
```

The current script cooker still refuses a host/target mismatch. The first iOS simulator archive attempt hit that explicit guard; no mobile archive or simulator execution is claimed here. A separately reviewed cross-target compatibility route is required before selecting iOS/Android targets.

Use the ordinary trusted project cook environment and `project` feature for authored gameplay, and `static-module` with its prepared build-time module registry for wasm2c programs. The default build links only the existing trusted test fixture. This library alone is not a complete mobile exporter or app host; it does not package an authored game's data, supply touch input, show a window, or sign/install an app.

## Evidence boundary

Local CPU/C ABI checks and platform archive builds are recorded separately. They do not establish a hosted-phone run, rendered correctness, hardware audio, the specified script/frame budgets, binary-size gate, or Phase 1 completion. The earlier document-oriented platform smoke application remains a separate historical probe and is not counted as this runtime executing a game.
