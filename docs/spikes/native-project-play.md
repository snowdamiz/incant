# Native project playback

`python3 tools/native_play.py path/to/game.incant.json --script behavior.js --ticks 60 --observe checks.json --pcm-output capture.pcm` cooks a bounded native-SDK CommonJS behavior into the trusted headless executable, copies that binary for the run and removes temporary compiler inputs before playback. The build uses the engine checkout, never project Cargo files. A cross-process lock covers build and copy; each simulation owns its executable. An expected source SHA-256 binds playback to the requested behavior.

Without `--script`, the engine no-op behavior is embedded and the scene still simulates native motion/physics/audio. An already prepared binary exposes `native-play PROJECT`; it refuses runtime source/bytecode arguments. No JavaScript/TypeScript source is parsed by that command. CommonJS transpilation and project-attached Script component routing are not implemented by this slice; unsupported components fail explicitly.

Authoring is validated/cooked before the native world is created, then dropped. Input replay, bounded typed assertions, selected final entities and committed primitive logs use the same accepted-tick lifecycle as the standalone player. Observations are now in an authoring-free crate shared with the player. PCM buffers are reused per tick; an optional new file is staged privately and published only after completed simulation. False assertions return the completed report and PCM with nonzero exit. Terminal simulation failure exposes neither; existing destinations are preserved.

Cached static and streaming audio works after original source deletion. Spatial scenes require an explicit camera and validate retained CPU render input; this command does not submit GPU frames. This is a developer coordinator with authoring tools in its executable, not a shipped dependency-boundary or phone-performance proof. Editor Play, engine-agent play.run, arbitrary project script routing, full native SDK, native session saves and GPU screenshots remain separate work.

## Verification

The final integration with native runtime PR76 is source commit `4e421f01d6a07c1c2d5adcca53ce20b0fbfe00b2`. It passes 601 ordinary workspace tests/doctests, workspace all-target release Clippy, optional GPU-player Clippy, 88 Python tool tests, SDK conformance and the resolved runtime dependency guard for the observation and game-player crates. The 66 ignored GPU checks were not repeated locally for this nonvisual change; hosted workflows retain them.

Eight real CLI tests cover selected scenes, bounded startup/observation input, replay, native motion, actual cached static/streaming audio after source deletion, completed false assertions, and failure without partial PCM. Existing destinations, project content and journals are preserved. Two Python tests cover bounded UTF-8 and actual cross-process build-lock exclusion plus release after cancellation.

The developer coordinator proof runs six playback processes using an actual build-produced custom behavior with temporary source removed. Three fresh successful runs agree exactly; direct/indirect eval and Function construction are disabled, accepted logs preserve Unicode, a false assertion retains completed output, a later guest exception publishes none, and a mismatched source digest fails before execution. The runtime project and journal remain unchanged. Run it with `python3 tools/probes/native-project-play.py PATH_TO_NEW_OUTPUT_DIRECTORY`. This custom-build proof is local; normal hosted workspace tests exercise the default embedded behavior and existing hosted shipping proofs exercise the shared cook/observation code.

Independent Astra extra-high review found no blocker in the trusted build boundary, native lifecycle, output publication or observation extraction. The later integration only adds previously reviewed PR76 ancestry and two coordinator regression tests.

The shared cook/observation extraction also passes a fresh source-free shipping-player proof with 47 process controls, including existing output preservation and bounded logs/observations. Temporary authoring/source/build storage is deleted before execution. The 4,703,536-byte player has SHA-256 `be767c2b0f61f16aea92165d5c38563bf777c14e7ac9ea5ce1d7c2b876eb2c36` and matching arm64 dSYM `f609646a-4e5f-3912-a293-c59c6b7b930d`. [Machine evidence](evidence/native-project-play-local.json) retains both complete proof results and validation/source hashes.

## Remaining work

This slice does not implement editor Play, an engine-agent play tool, arbitrary project script routing, a TypeScript transpilation/bundling pipeline, full native SDK parity, composite session saves, GPU frame capture, phone measurements or a phase gate. Unsupported capabilities fail explicitly. The legacy headless `play` command remains for existing tests while migration continues; `native-play` is the new disposable-world route.
