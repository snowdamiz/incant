# Authored Android static Wasm on main189

The accepted Android static-module source is composed with main189's reviewed
renderer/cache changes and quota-test repair. All nine producer source files are
unchanged. Current verification passes 62 cooker cases, full Python/Node tools,
Clippy, formatting, conventions and the lean runtime graph. Both physics modes
also pass fresh public authoring/Undo/Redo/reopen and captured Wasmtime/native
oracle preflights: four current processes cover 96 ticks, 186 fields per tick,
248 vector observations and exact nonzero PCM.

Both modes pass actual `aarch64-linux-android` target `cargo check`, including
fresh build-time project admission and all five wasm2c C compilations per mode.
The actual five-solver target artifact features and C safety/numerical flags are
checked. This is target compilation and admission, not a new linked archive or
new Android process execution.

The selected normal and build dependency graphs match the accepted producer
exactly after checkout path normalization. Each mode has 692 selected local
source rows; the only changed row is the reviewed `cfg(test)` quota regression.
The only lock change adds `incant_render -> rustix`; neither that renderer nor
its shader compiler belongs to the selected Android graph. Cargo.lock remains a
conservative script build-identity input, so current host/target admission uses
freshly cooked current identities, not old data with a newly linked runtime.

The new guest contains current source paths. With only original source-location
mapping, the unchanged current SDK and guest reproduce the producer's exact Wasm
hash. The normal fresh bundle retains its own audited hashes. Rust/NDK binaries,
wrapper, target macros and flags match; Clang's retained diagnostic differs only
in independently compared compilation-directory paths. Three strict integration
runner stops are preserved: wrong WABT root/bin argument, whole-bundle path hash
comparison, and whole diagnostic cwd hash comparison. No production change was
needed to resolve them.

The six original source-deleted Android API37 caller processes and 26 precise
refusals remain **producer-scoped**. Their complete assertions, original streams,
archives and binary hashes were rechecked. No new Android archive/caller success,
physical phone, minimum-OS execution, rendering, live audio, performance, signing
or phase approval is claimed.

Machine-readable closure, raw evidence and the assertion-only collector are in
[the integration manifest](evidence/authored-android-wasm-main189.json).
Independent final current-main review is pending.
