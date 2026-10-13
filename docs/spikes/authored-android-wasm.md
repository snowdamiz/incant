# Authored Android static WebAssembly export

The ordinary Android archive exporter now accepts one publicly selected,
captured Wasm scene module. Pinned wasm2c 1.0.42 translates its prepared bytes
inside the trusted private export build. The existing native runtime and bulk
SDK execute the generated module automatically; no new ABI opcode, fixture
selector, runtime source compiler, JIT or executable-cache loader is added.

Admission is deliberately narrow: the reviewed macOS arm64 producer and exact
`aarch64-linux-android` destination, complete original/prepared/registry capture
identity, complete six-file generated bundle, and the project's selected
physics family must agree. Missing capability/bundle, mixed per-entity
TypeScript, stale input or producer identity and unsupported architectures fail
explicitly. Existing Android TypeScript and iOS routes remain available.

`tools/export_android.py --wabt <trusted-wabt-root>` checks the actual NDK
28.2.13676358 API24 wrapper and resolved compiler binary, target predefines,
cc1 triple/resource/system headers, selected normal dependency graph, five
compiled target physics artifacts and shipping Rust code generation. All five
runtime/module C sources must use the exact wrapper and numerical/bounds flags.
The wrapper supplies the target internally; command-line target/sysroot,
response-file and toolchain overrides are refused. Raw compiler, symbol,
section and ELF audit commands are retained with hashes and exits.

The unlinked archive contains unused QuickJS parser definitions. The final
C caller uses `--gc-sections` and is separately checked for the mobile entry
point and compiled module, absent source-parser symbols, and absent DWARF.
Archive membership is not described as the final executable dependency graph.

## Actual public proof

The fixture imports real WAV assets and a Rust SDK Wasm module through public
commands, atomically selects the scene program and physics mode, and verifies
Undo/Redo plus the persistent journal reopened through public RPC with
`--journal`. Private Rust/Wasm/WAV sources are removed before development.
For each physics family, two fresh Wasmtime processes match an independent
native world/mixer oracle over 96 ticks, six records and 186 scalars per tick
(17,856 fields per process). Exact stable identities and u64 transport epochs
remain observable; 248 bounded vector checks supplement the full records.
Six exact guest-error controls also pass in each development mode.

Two fresh ordinary shipping exports compiled successfully, in 246.32 seconds
for deterministic and 256.54 seconds for fast on this producer. Each export
checks its cooked scene against the oracle and removes its private build.
Temporary compiled backups and trusted translation tools are hash-recorded
and removed before callers. Project/cache/journal and private source maps are
also removed. Six fresh local ARM Android API37 C processes (three per physics
family) then match every record and the exact 614,400-byte nonzero/silent PCM
sequence. This is offline numerical audio evidence, not device audio playback.

Twenty-six exact refusal controls pass: valid opposite-mode cooked packages in
both directions; short snapshot and command buffers; invalid gain; late invalid
owner, role and seek; create-only output preservation; malformed C ABI requests;
and a bad scene checksum. Failed game requests publish their defined error
report without partial game/PCM output. Callback rollback itself remains
covered by the earlier native host conformance evidence. Static module
checkpoints remain unsupported; the unknown save request tests only C ABI
refusal and does not establish checkpoint support.

Each proof uses a newly named owned AVD, two cores, 2 GiB, software rendering,
`-no-window` and `-no-audio`, with a cleared C caller environment. Both AVDs
shut down and were deleted successfully. The unrelated port5556 emulator was
untouched. The first fast caller attempt refused an immediately reused port
before creating an AVD; that failure is retained. Only the missing fast caller
stage was rerun on a new port with unchanged sources and archives.

## Checks and evidence

Implementation is `026426d1dca4ff8e41aacbf02741ec68fc4aa386` on immutable
`4b277d577c6627e98e407b2c25daf274a99fd406` (main185 plus accepted iOS Wasm).
Both archive/caller runs bind 14,411 unchanged inputs. The manifest retains
lossless raw logs/receipts and hashes external native binaries; the included
producer audit rechecks the public transactions, all-field/PCM parity, exact
refusals, target/compiler/physics/symbol identities and owned cleanup.

Four captured-bundle Rust unit tests and all four `mobile_project` integration
tests pass, together with all-target scoped Clippy and 33 Android/iOS Python
exporter tests. The stale Android integration expectation and the subsequent
incorrect inner-vs-outer architecture diagnostic expectation are retained as
failed attempts. The final test checks the exact actual guard. A first producer
re-audit counted chmod/checksum commands as game invocations; that observer
failure is retained and its filter now requires the actual cleared-environment
caller command. Neither correction changed compiled production inputs.

Source and public-probe reviews are accepted. Independent final execution review
and publisher integration are pending. See the [evidence manifest](evidence/authored-android-wasm/manifest.json).

API24 is a compilation target; actual execution used API37. These results do
not establish minimum-OS compatibility, a physical phone, frame performance,
GPU rendering, a mobile window/input/audio host, signing, store distribution,
or any Phase 1 exit gate. Mixed scene Wasm/TypeScript and static guest saves
remain explicitly unsupported.
