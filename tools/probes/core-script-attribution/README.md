# Private correlated ScriptTick diagnostic

This is a source-only private adapter for the reviewed default-off attribution
feature. No private shipping build, full-Core diagnostic player or timing run has
been authorized or executed as part of this source preparation. It changes no
public exporter or ordinary v3 measurement consumer.

`build.py PLAN OUTPUT SCRATCH_PARENT` admits only the original fast4,
`project,render-input,profile` graph plus `diagnostic-script-phases` on the exact
four receiving crates. It uses ordinary dependency/codegen, physics compiler,
symbol, camera and data-package helpers. It writes `diagnostic-build.json`, never
`export.json`; normal measurement package admission consequently refuses it.
Actual codegen must remain Rust 1.99 arm64, opt3/fat-LTO/abort/one CGU and packed
symbols, with no source compiler, checkpoints or SIMD8 in the runtime. A fresh,
create-only owned temporary target is removed after its outputs are copied. The
builder does not launch a player. Expected scratch use is about the prior 3.24 GiB;
actual available space must be reviewed before a future build.

The host-owned plan binds the clean source commit/inventory, toolchain, original
74 authored files, baseline exporter receipt/graph/producer, functional report,
PCM, input recording and observe file. Paths and fixed outputs come from that
reviewed host plan, never from authored project fields. The authored Core project
is freshly cooked using ordinary admission. Every script metadata field except
its genuine conservative build fingerprint must remain equal: exact source,
bytecode, hardening, bindings, source map and ABI. Actual bytecode/hardening arrays
are extracted from retained trusted generated Rust and independently hashed.
All other producer fields, including the compiler manifest, scene and assets,
and every packaged data file must remain identical. No old fingerprint is forced
onto the new executable or envelope.

The inventory is checked against every tracked file in `crates`, `third_party`,
`sdk`, `tools` and `.cargo`, plus the workspace manifest, lock and Rust toolchain
files. The plan also binds the real fnm Node 24.21.0 executable by absolute path,
SHA-256 and observed version. The private build prepends its directory to PATH,
verifies that `node` resolves to that executable, and refuses Node option/module
injection. Both the Node binding and compiler inputs are rechecked after build.

The separate `collect.py` command supports exactly two predeclared stages at
create-only fixed output locations:

1. `--stage functional`: one unprofiled 2,156-tick cold/full Core win-route child.
2. `--stage diagnostic --functional-result ...`: one maintained v4 2,156-tick
   child, admitted only after the same package's successful functional result.

Each child has 45 seconds and an owned process tree; the stage supervisor has a
90-second execution deadline with cancellation latched through the existing
managed helper. Host probes are separately bounded. Reaping and filesystem IO
can extend wall time; the deadline is not claimed to interrupt cleanup. Outputs
are checked during managed waiting and read with caps: stdout 32 MiB, stderr
1 MiB, PCM 128 MiB. The output bound is checked between process waits, not an OS
quota. Each stage records actual terminal status, raw hashes, Mac identity and
load, with zero retries/exclusions. Neither stage runs a GPU or builds anything.
There is no extra startup, detailed-heap child, or hidden warmup. Future root
resource/execution review remains required, with a quiet window for the single
instrumented attribution route when coordinated.

Both full logical output and exact PCM must match the retained baseline.
`analyze.py` requires complete v4 same-frame identity/counts, exactly two known
source slots, bounded storage, major partition equality, nested containment,
existing whole-tick total/max correspondence, and exact histogram reconstruction.
It selects the same 108 largest whole ticks (frame ID breaks ties), exposing
major and nested totals plus residuals. It never subtracts marginal quantiles or
adds nested spans as exclusive work. Observed instrumentation and residual work
stay included. No 2 ms, 5%, speedup, gate or distribution approval is emitted.

`test_protocol.py` contains data-only correspondence/refusal controls for the
collector, exact feature admission and complete input inventories. The original
four controls passed before adding the final Node/inventory binding; final
control outcomes are recorded with the external source freeze.
The Rust observer's earlier focused qualification is separate; see
`docs/spikes/core-script-attribution.md` and its external receipt. The normal
shipping 2 ms failures remain unchanged.
