# Native flipbook script capability

This adds script access to the existing native playback controls. Playback plus
motion-only batches preflight all touched states before publishing either,
without staging hierarchy metadata. Asset/configuration authoring, cooked scene
versions, render geometry, animation timing and legacy document Play are outside
this change. Local results below cover this additive capability; hosted checks
for this revision remain pending.

QuickJS receives `flipbooks`, one aligned bulk snapshot with stable `ids`, numeric
`frames` and `flags` (bit 0 playing, bit 1 completed). It describes the committed
world after native simulation and before that callback's commands. Array order is
unspecified. `commands.controlFlipbook(id, 'play'|'pause'|'restart')` and
`commands.controlFlipbook(id, 'seek', frame)` use the existing structural queue.
Same-callback snapshots do not reflect queued controls. Seek is a zero-based
integer, checked against the final native animation declaration at commit.

Invalid arguments to an active command poison its whole callback even when
caught. Final native rejection, throws, async work or invalid numeric edits restore
numeric writes and discard all controls/structural commands. Retained command
functions never reactivate: they throw after their original update expires.
Catching that expired-function error alone does not poison the current callback,
matching the established command epoch contract. NativeSession remains terminal
on any failure after accepting input/simulation; this is not a rollback of the
accepted input/native tick, audio cursor or VM state.

Snapshot collection retains a Bevy query and caller scratch, visits only playback
archetypes, and discovers structural changes. At most 32,768 rows are copied; no
frame lists, assets, configuration, private rational clock remainder or native
pointer is exposed. QuickJS's numeric arrays are VM-owned copies accounted to its
existing 32 MiB memory limit; ordinary numeric ECS loans retain their separate
revocation contract. ID entries are own data properties, and these copied values cannot mutate native
state. This is copied readback, not zero-copy or a performance gate.

## Additive Wasm wire contract

The existing `incant_v1` and `incant_input_v1` signatures are unchanged. The new
namespace is `incant_flipbook_v1`:

| Function | Core Wasm signature | Result |
| --- | --- | --- |
| `snapshot_len` | `(i64 epoch) -> i32` | Initialized snapshot byte length |
| `read_snapshot` | `(i64 epoch, i32 ptr, i32 capacity) -> i32` | Copies all initialized bytes, returns length |
| `control` | `(i64 epoch, i32 id_ptr, i32 kind, i32 frame) -> ()` | Queues one native control |

Snapshot bytes are little-endian: `INCFB001` (8 bytes), row count (`u32`), row
stride (`u32`, exactly 24), then rows of 16 stable ID bytes, frame (`u32`), flags
(`u32`, bits 0/1 only). ID bytes retain their existing canonical big-endian ULID
representation. Maximum initialized length is 786,448 bytes. Empty snapshots
contain the 16-byte header. Destination capacity/range must contain the whole
snapshot; rejection never writes a partial prefix. No Rust layout/padding leaks.

Control tags are 0 play, 1 pause, 2 restart, 3 seek. Non-seek `frame` is reserved
and must be zero. Seek accepts a `u16` value before final per-animation range
validation. Imports require the active epoch and share the existing host-call,
transfer and command quotas. Wasm traps cannot be caught with the supported
proposal set; imported failures permanently fail the module and discard staged
writes. wasm2c traps remain wholly in C, after Rust callbacks return normally.
There is no runtime file/compiler/JIT capability or translator version change.

## Observable verification

Native tests exercise owned snapshot retention, empty worlds, archetype changes,
structural growth/removal, and scratch reuse. QuickJS tests exercise bulk alignment,
read-after-write semantics, active versus expired functions, prototype hazards,
rollback and accepted-input ordering. Shared wire and actual Wasmtime/wasm2c cases
exercise tags, bounds, epochs, quotas, playback controls and old module compatibility.

The optional existing game-player `--static-module NAME` chooses only trusted
build-linked descriptors, behind its explicit build feature. Existing nonvisual
constructors and default player invocation remain strict. The fresh source-free
proof compares replay-driven QuickJS and static controls against independent
static-sprite GPU frame hashes, with physics/audio and retained bindings intact.
The same trusted Rust guest is also executed by Wasmtime for conformance. No full
script migration, visual approval, portable export, reference-phone, browser,
performance or phase gate is claimed.

The existing macOS native-game CI command adds `--flipbook-sdk` to its existing
`--gpu --flipbook` execution. The three-desktop development/static conformance
helper now includes the real Rust flipbook guest and eight new WAT fixtures while
retaining the original 13 Wasmtime cases, numeric/input/fuel/trap tests and fresh
standalone/static-game proofs. Compiler features remain confined to the explicit
development graph. No existing checks are removed; the new GPU controls reuse
the same freshly built native game executable.

## Local results and reproducibility

[The durable evidence](evidence/native-flipbook-sdk-local.json) records every
changed source hash, test-log hashes and the full fresh-process report. The final
local runs passed 204 scoped release tests/doctests, 158 combined actual
static/Wasmtime tests, 687 workspace release tests/doctests, all-target Clippy,
90 Python tool tests, strict SDK TypeScript, actionlint, conventions and format
checks. The workspace run left 71 renderer GPU tests ignored; renderer production
code is unchanged here, and the new GPU behavior was exercised by the fresh
executable proof. Default-only `wasm32-unknown-unknown` compilation passed for
native/runtime capability crates. The 16-root dependency guard passed, and the
fresh player audit separately excludes authoring, Wasmtime, Cranelift and module
compiler dependencies from its normal runtime graph.

Commands, with `/opt/homebrew/bin` on PATH and all Cargo calls limited to `-j2`:

```sh
./tools/cargo test --locked --release -j2 -p incant_runtime -p incant_script -p incant_wasm_runtime -p incant_wasm_cook
python3 tools/probes/shipping-wasm2c.py /tmp/incant-flipbook-sdk-conformance-03 --conformance-only
./tools/cargo test --workspace --release --locked --features incant_audio_runtime/scene -j2
./tools/cargo clippy --workspace --all-targets --release --locked --features incant_audio_runtime/scene -j2 -- -D warnings
./tools/cargo build --workspace --bin incant_headless --example cook_scene --example native_scene_audio --example no_renderer_fixtures --release --features incant_audio_runtime/scene --locked -j2
./tools/cargo check --locked --target wasm32-unknown-unknown --no-default-features -p incant_runtime -p incant_wasm_runtime -j2
python3 tools/probes/native-game.py /tmp/incant-flipbook-sdk-shipping --gpu --flipbook --flipbook-sdk
python3 -m unittest discover -s tools/tests
```

On aarch64 macOS with the Metal Apple M5 Pro adapter, the fresh shipping proof
preserved 32 baseline replay/GPU/PCM success processes and 35 refusal controls.
It added 32 successful SDK processes: pause, seek while paused, restart and resume,
each with static/streaming audio, QuickJS/wasm2c and two fresh repetitions. First
and final frame hashes match independent static-sprite references; the same
120 accepted ticks, physics/PCM and unchanged binding/topology generations are
asserted. Eight additional invalid-seek/trap processes fail without a PCM file or
success report. The original four animated/static comparisons, log/observation
controls, strict default nonvisual refusal and CPU mode remain covered. No image
appearance judgment is inferred from these numerical checks.

The copied player is **8,995,472 bytes**, SHA256
`046a02dd1fe8ed9c4e9e35a5836aa5006ece7e3baa5fb57b2951a5cbbc2bf4fc`.
The separately retained arm64 dSYM matches UUID
`face3505-03d5-3158-9394-67bca31c8623`. Rust 1.99.0/LLVM 23.1.1 commands verify
optimization 3, fat LTO, one codegen unit and panic abort. The translated C uses
the unchanged pinned WABT 1.0.42/runtime with strict numeric/fuel flags. The C
audit verifies global flag presence in a trusted build log, not adversarial
build-environment correctness. License and source/translated-module identities
are retained. Compared with the prior matching-profile flipbook fixture's
8,943,600 bytes, this executable adds 51,872 bytes; this is a whole fixture/graph
comparison, not an isolated ABI cost or performance result.

Temporary project, script, Rust/Wasm/generated C, translator and fresh shipping
build storage were deleted before player execution. Cooked scenes/assets and
startup recording/action JSON are deliberately retained runtime inputs; the
repository and validator tools also remain. QuickJS stays linked with source
parsing disabled; this is not a QuickJS-free binary. Simulation/host failures
publish no PCM file, but an output I/O failure can leave a partial file. Session
saves, generated Rust SDK completeness, Linux/Windows execution of this new
capability, browser execution and device/performance gates remain unverified.

The original fresh result SHA256 is
`82b74b3dbc116821efee142449b85666a8dde7bc6a0faf52f1ce8d51e99c59be`.
Parent independent review accepted the frozen sources and exact fresh report.
