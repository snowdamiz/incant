# Public authored WebAssembly selection and native export

A scene can select one imported module with `program: {"kind":"wasm",
"module":"<asset ULID>"}`. Omitting or clearing `program` preserves the existing
TypeScript route. The typed `set_scene_program` command uses the shared atomic
command transaction, revision check, actor provenance, Undo/Redo and durable
journal. Document validation refuses missing/wrong-kind references and any
per-entity TypeScript `Script` attachment in a Wasm scene; no attachment is dropped.

The public asset importer accepts bounded core `.wasm` files up to 1 MiB. Import
and reimport retain the asset ULID and commit a new captured fingerprint only
through `incant_cmd`. Immutable `.incant/cache/wasm/<fingerprint>.wasm.json`
entries capture original and prepared binaries, original/prepared SHA-256,
the generated capability registry identity and the fixed metering policy.
Every consumer verifies the fingerprint, validates the original, regenerates
metering and requires byte equality with the captured prepared module. Changing
untrusted envelope hashes cannot substitute an unmetered binary. Default cache
resolution rejects symbolic links and oversized cache files. Source
files are unnecessary after import; a source change takes effect through reimport.

Build the development host with `--features wasmtime-host`, then use ordinary
`native-play` or native editor Play. Native startup transfers the captured
original binary into the optional Wasmtime `SessionProgram` variant. The normal
native world, accepted input timeline, audio clock and committed log mechanism
remain shared. No document or command bus reaches the tick path. Disabled hosts
explicitly refuse selected modules. The legacy document simulator also refuses
the selector rather than silently running a TypeScript fallback.

`python3 tools/export_native.py PROJECT OUTPUT --wabt PATH_TO_WABT_1_0_42`
resolves the same selector. The engine-owned exporter validates and snapshots the
captured module in fresh private build storage, checks the translator version,
and translates the exact captured metered bytes. The existing wasm2c descriptor
and C facade generator produces the linked module. The native build producer
rechecks the project/cache identities, records them in `project-build.json`, and
binds the module as the player's default program. Ordinary `--package` startup
automatically selects it; the acceptance path never uses `--static-module`.
The authored program cannot be overridden using that fixture-selection flag.
The exported normal dependency graph excludes the cooker, document, command bus,
agent, Wasmtime and Cranelift. The executable never loads a native code cache,
Wasm source file, arbitrary descriptor or runtime source compiler.

Both development and static module VM checkpoints explicitly return Unsupported.
This includes module sessions whose world contains an active or paused skeletal
crossfade; neither a world-only snapshot nor a linked-module interface establishes
full guest state restoration. The existing TypeScript checkpoint route is
unchanged. This work is one scene module on the current host==target desktop
exporter, not complete gameplay SDK, iOS/Android distribution, routed UI ABI,
physical-phone performance, rendered-pixel approval or a phase gate.

## Reproduction

```sh
./tools/cargo build --locked -j2 -p incant_headless --features wasmtime-host
python3 tools/probes/authored-wasm-export.py /tmp/new-authored-wasm-proof \
  --binary /path/to/incant_headless --wabt /path/to/wabt-1.0.42
```

The probe authors through public init/import/RPC transactions, verifies unchanged
reimport, selection/clear Undo/Redo and reopening the editor-default journal
path through public RPC (`--journal` remains explicit for RPC), compiles a real
Rust guest against the generated SDK, then deletes its Rust/Wasm source before
development Play and export. It compares movement, clock, input edges and every
log record. After export it removes only its own authoring/cache/build fixture
and newly produced source maps, records their hashes, runs three fresh ordinary
packaged processes, and verifies module checkpoint refusal before any output is
published. Cache identity corruption is also refused through public native Play.

## Local measured results

The 2026-10-12 `aarch64-apple-darwin` CPU proof used Rust 1.99.0 and WABT
1.0.42. A real SDK Rust guest receives accepted input actions, moves its entity,
counts press edges and emits one committed log per tick. Development Play and
three fresh ordinary packaged processes each produced 20 ticks, 16,000 audio
frames, translation `[1.2,20,2]` and identical ordered logs. The shipping player
SHA-256 is `d71f80e6490a2df2490ae284b5ea1f5e8935d29aedac46c243ed7f6aee5c3aa1`.
Its 268-package normal dependency graph excludes all forbidden authoring/JIT
packages; the shipping symbol audit found no runtime source compiler. Codegen
used opt-level 3, one codegen unit, panic abort and fat LTO for the player. The
probe deleted only its private source, project, cache and build inputs, with
hashes recorded before deletion, and used no fixture-selection flag. Its actual
checkpoint request failed before publishing a PCM or checkpoint destination.

The affected Rust suites pass 261 tests/doctests with Wasmtime enabled; the
focused cache-forgery/startup checks and the feature-disabled startup route also
pass. Scoped all-target Clippy, optional-host editor compilation, supported
`wasm32-unknown-unknown` importer/command/development-interface compilation,
226 Python tooling tests (four platform skips), 21 Node tests, all 936 editor UI
tests, UI build and generated schema/SDK/bridge checks pass. These are local
correctness and compilation results. An initial broader browser check including
the native agent hit its existing `uuid/v4` randomness configuration; the
supported browser crate scope passed without changing dependencies to mask it.
No native window, GPU, screenshot or physical device was exercised.

[Machine-readable receipt and input/log hashes](evidence/authored-wasm-export.json).
