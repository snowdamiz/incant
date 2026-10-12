# Committed native Wasm diagnostics

The additive `incant_log_v1::write(epoch, level, ptr, len)` import copies a primitive
UTF-8 message into the active module update. Levels are generated registry tags:
`debug=0`, `info=1`, `warn=2`, `error=3`. Existing numeric/input/flipbook namespaces,
wire layouts and opcodes remain unchanged; logging appends opcode 17. An older
host explicitly refuses a guest importing the new namespace.

The allocation-free `no_std` Rust guest API is
`tick.log("Ready 雪🙂", LogLevel::Info)?`. It validates message size/control
characters before the import and poisons the callback if its error is ignored.
The host independently checks the epoch, tag, complete memory range and UTF-8,
including guests bypassing the SDK. Tabs/newlines are permitted; other control
characters and malformed UTF-8 are refused. No guest string coercion, formatter,
console output, filesystem, project mutation or JSON serialization runs here.

Ingress matches native QuickJS: 1,024 UTF-8 bytes/message, 64 records/update and
16 KiB/update. Each host retains a prefix of at most 1,024 records/256 KiB text.
Once a committed record cannot fit, every subsequent committed record increments
a saturating drop counter. Full history does not reject valid gameplay. The
registry bounds are compile-time checked against the shared native contract.

Records publish only after guest return, numeric validation and the complete
structural batch succeed. A trap, ignored invalid log call, expired epoch or
rejected write publishes no pending records or drop increments. Existing numeric
rollback remains intact. Public log access is refused after terminal failure.
An independently checked successful-update clock labels records and refuses
updates beyond 9,007,199,254,740,991 before publication; it does not
advance the owner's simulation. Successful development reload preserves history
and that clock; failed reload leaves the current instance unchanged.

`incant_runtime::logging` owns the unchanged serializable history/data types;
QuickJS re-exports its original public names. The static module's actual history
now reaches `NativeSession::logs()` and existing game/report observers, replacing
the prior always-empty placeholder. Static checkpoints remain unsupported.
This diagnostic history does not authenticate or restore guest/VM/session state.

The fixed trusted Rust sample and adversarial guests run through actual Wasmtime
49.0.2 and pinned WABT 1.0.42 wasm2c output. Tests compare ordered levels, Unicode,
empty messages and accepted clocks with QuickJS; check exact update/history
limits and drops; and reject malformed UTF-8, controls, ranges, epochs, oversized
messages, excessive calls, traps, invalid numeric/structural writes and startup
imports. A static NativeSession test serializes the existing `entries`/`dropped`
report shape through its public observer and refuses observation after failure.

Validation passed: 57 combined runtime/SDK/actual-host tests and doctests, the
eight existing QuickJS log regressions, all-target Clippy on four affected
packages, 16 generator/preparer Python tests, formatting/generated checks, and
six shipping normal graphs excluding development compiler/parser/cook edges.
Exact source/tool/build receipts are recorded in
[evidence/native-wasm-logs.json](evidence/native-wasm-logs.json). These are local
macOS arm64 behavioral checks, not controller, GPU, device, performance, complete
SDK or distribution gates. No native window, account/provider, runtime source
compiler or untrusted module build service is exercised.
