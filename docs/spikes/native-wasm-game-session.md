# Static Wasm in the native game session

A trusted build-linked module now uses the existing accepted-tick native session
and game composition. `SessionProgram` selects build-produced QuickJS bytecode
or, under `static-module`, a reviewed `ModuleExports` descriptor. Neither variant
loads source or native function pointers from project data. The existing unsafe
descriptor-construction contract and C-only trap boundary are unchanged. A private
`SessionVm` owns the selected host; its box is allocated once at startup. Existing
`CompiledScript` constructor calls remain valid through `Into<SessionProgram>`.

Pending-world identity, visual-host preparation and all acceptance ordering remain
unchanged. The host validates the immutable action map once at construction or
rebind. Each tick accepts input atomically, performs native simulation, then calls
the selected guest. The game host then synchronizes audio, renders offline PCM
when requested, and synchronizes an explicitly retained visual host. Invalid input
or a wrong offline buffer is retryable and does not execute the module, advance
the world/audio clock or consume pending input edges. After acceptance, simulation,
module, audio or visual-host error/unwind is terminal. Public world/input/audio/host
state disappears; accepted offline failures zero the caller's output buffer.
Already advanced simulation and asynchronous device cursors are not rolled back.

The existing Wasm host still uses copied, bounded numeric transfers and bounded
undo through the shared native command transaction. It is not zero-copy, and the
session adds no second world clone, serialization or authoring validation. Its
`incant_v1` guest interface currently has **no input capability**. This proof
observes replay through the native session, not from inside the guest. The native
module's own counter proves rejected packets did not execute it.

## Observable checks

From the repository root (local Cargo commands use `-j2`):

```sh
python3 tools/probes/static-game.py /tmp/static-game-conformance --conformance-only
python3 tools/probes/static-game.py /tmp/static-game-shipping
./tools/cargo check -j2 -p incant_script -p incant_game_runtime --no-default-features --locked
./tools/cargo tree -p incant_game_runtime --no-default-features --edges normal --prefix none --locked
python3 tools/check_runtime_dependencies.py incant_wasm_player incant_script_player incant_game_player --features incant_wasm_player/fixture
```

The conformance helper provisions checksum-verified WABT 1.0.42, compiles the
trusted Rust fixture with pinned Rust 1.99.0, validates/meters/translates it using
the existing cook, and runs the actual linked module. It passed 66 scoped tests
and doctests plus all-target Clippy with warnings denied. Nine new module tests
cover identity, accepted/rejected input and edges, an internal guest counter,
physics movement without double integration, structural spawn, static/streamed
spatial audio after cache deletion, exact rational frames, numeric/structural
rejection, simulation-before-guest failure, post-module mixer-domain rejection,
strict camera/manifest refusal before a looping module start, and terminal host
unwind with output silence. Existing 56 script/game cases and one documentation
case remain. Removing a required listener is correctly rejected by the native
structural validator; moving a valid listener outside the mixer's numeric domain
exercises the later audio preflight error instead.

Default script/game checks also pass. The ordinary game dependency tree contains
neither `incant_wasm_runtime` nor Wasmtime/Cranelift. The new player opts into the
static host explicitly. Full normal-edge traversal and fresh target compile
commands reject compiler/JIT and authoring dependencies. The optional feature's
actual module tests require the helper; plain workspace tests alone do not run
them. No workflow changes are included in this slice; integration must explicitly
invoke this conformance/proof command. Formatting, conventions, dependency guards
and 49 Python tool regressions pass.

## Fresh executable evidence

[Exact source hashes, logs and result](evidence/native-wasm-game-local.json) record
the local `aarch64-apple-darwin` run on base `f90b8d5`. A new target directory
verified optimized shipping Rust flags (fat LTO on the final executable, one
codegen unit, panic abort), configured C flags, compiler-symbol exclusion and
matched symbols. The normal player graph still includes QuickJS through
`incant_script`; this is **not a QuickJS-free architecture**. The source compiler
feature is absent and no runtime script/Wasm loading route is provided. No new
external dependencies or upstream versions were introduced.

- Player: **3,897,424 bytes**, SHA-256
  `5c88ce639a7a24eac7e35349541c6173fced712cabd1476d2677661f714ae43d`.
- Matched arm64 dSYM: `794a2530-599d-3a71-b239-e992d9961972`.
- Raw result SHA-256:
  `08edb9b35365d0c9f9784e4e8897c8550cf4efdc8277025efaf628c84aa077d3`.

Six fresh success processes run the real cooked v3 physics/audio scene: static
and streamed PCM, repeated three times each. At tick 64 the module-driven body is
`[3,1,0]`, the module-spawned entity is at x `1.96875`, the native input timeline
has one press and one release, and the rational 44,117 Hz clock produces exactly
44,117 stereo frames. All samples are finite, spatial movement favors the right
channel, and repeated-process PCM hashes match. Twenty-six failed processes
publish no PCM: guest traps/invalid writes/structural commands/fuel, missing linked
module, real v4 camera-only and manifest-only scenes, simulation overflow before
a looping guest, recording clock mismatch, missing clip fingerprint and runtime
source-loading arguments.

These executions happen after deletion of the temporary guest Rust, Wasm,
generated C, translator tools and fresh build storage. The repository and validator
build remain. Retained PCM metadata/data and recording JSON are intentional
startup runtime exchange files, not deleted compile inputs or cooked binary input
assets. Output I/O is not atomic: a write failure may leave a partial disposable
PCM artifact even though simulation/module/audio failures create none.

This composed proof was executed locally on macOS only. The underlying static
host's exact `f90b8d5` [PR #64](https://github.com/snowdamiz/incant/pull/64) passed
actual Linux/macOS conformance and fresh packaging, plus Windows MSVC 19.51.36260
`/Od` and `/O2` conformance and fresh `/O2` PE/PDB packaging. Those jobs do not
establish cross-platform execution of this newer composed player. The C-flag
audit remains a trusted-build global-presence audit, not adversarial validation of
every translation unit or external compiler flags.

Renderer composition for static modules, complete guest input/SDK capabilities,
VM/input/audio save/restore, portable/device execution and the PLAN performance
and release gates remain open. No default strict visual-host refusal was relaxed.
