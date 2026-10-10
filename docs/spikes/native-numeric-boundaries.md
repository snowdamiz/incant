# Native numerical boundaries

Date: 2026-10-10. Four new regressions first failed against the preceding native
runtime/script implementation: finite values could overflow during integration
or parent/child composition, publish invalid derived transforms, and leave a
successful script loan impossible to save. This change closes those cases for
native stepping and the script transaction. It adds no authoring dependency.

## Checked publication

`NativeWorld::step` computes candidate global transforms in the existing
parent-ordered hierarchy scratch buffer before advancing live components or the
tick. It checks typed component domains and finite composed results. A failure
returns the affected stable ID and leaves components, published globals, clock
and binary save bytes unchanged. A corrected subsequent step rewrites every
scratch parent before reading it, so a failed attempt cannot leak movement.
Successful integration uses the same translation arithmetic as preflight; the
native parallel integration schedule then publishes the prepared matrices.

After revoking every foreign view, the script host calls
`NumericColumns::validate` before releasing its bounded undo data. Invalid
component values or hierarchy composition restore all numeric writes through
fresh safe visits, and permanently fail that host. Cached foreign pointers must
never be dereferenced after validation or another native borrow. Queued
structural commands retain their existing checked, atomic publication.

This uses native typed arithmetic and the existing derived-transform scratch;
there is no document projection, JSON, authoring validation, world clone or new
unsafe code. Trusted native immediate-write APIs still retain their documented
unchecked semantics. A caller using them directly remains responsible for
valid values. `NativeSession` stays terminal after a runtime failure once input
was accepted; the input/VM session is not a rollback transaction.

## Verification

- All four new regressions reproduce against the old implementation and pass
  after the change, including late failure, retry, complete script undo,
  retained-view revocation, and session failure before script execution.
- 80 scoped runtime/script tests and doctests pass. All 407 release workspace
  tests and doctests pass; the normal 44 ignored GPU tests were not rerun for
  this numeric change. Workspace/all-target and compiler-disabled Clippy pass.
- Independent source review found no blocking issue. Its pointer-lifetime
  documentation suggestion was applied.
- A fresh shipping build passes all 24 standalone source-free input/script
  runs, removes build inputs before execution, excludes source-compiler symbols
  and rejects dynamic source compilation in the actual player.

The stripped arm64 macOS player is 2,170,968 bytes, 16,560 bytes above the prior
2,154,408-byte native-input fixture. SHA256
`be547381f6fd09dcadc3138b218458b1b31e5564c42f5e3123821b26bc7ac03a`;
matched dSYM UUID `e7e9324a-2622-35cc-b153-db1b15259907`. This is the complete
fixture-player delta, not an isolated library size. The
[recorded evidence](evidence/native-numeric-boundaries-2026-10-10.json) includes
source hashes, codegen, symbol identity and exact observed process results.

## Local cost diagnostic

The standalone [probe](../../tools/probes/native-step.rs) links the baseline and
candidate ordinary-release libraries with the same pinned compiler and runtime
dependency graph. It creates 10,000 or 100,000 entities outside the timed region,
warms 20 steps and measures 120 individual steps. Static roots, moving roots and
moving eight-entity chains each run three times, alternating binary order.
Both binaries reach the same tick and last world position in every case.

On this Apple M5 Pro, median-of-three per-run p50 values at 100,000 entities were
1.632/1.527 ms for static roots, 1.740/1.777 ms for moving roots, and 4.560/4.458 ms
for parented entities (baseline/candidate). Tail measurements were noisy with
other development work active: parented p95 was 5.448/6.695 ms. This is a local
diagnostic, **not a performance pass or an improvement claim**. It neither uses
the shipping profile nor measures the Core Sample, script loan phase, memory,
Tracy traces or a reference phone. The plan's matched shipping/device gates
remain open, including investigation of tail cost under controlled conditions.

Build the same runtime-only release graph in each frozen checkout with
`./tools/cargo build -p incant_runtime --lib --release --locked -j2`. Compile the
probe using pinned `rustc --edition=2024 -C opt-level=3`, the checkout's
`target/release/deps` dependency path, and its exact `incant_runtime` and
`incant_types` rlibs; run each binary with `100000 parented` (or the other count
and modes above). Library, probe and executable hashes and all measurements
are retained in the evidence. The baseline library was built from 517b6aa;
its runtime/types source matches the subsequent native-input integration.

Raw shipping files are in ignored `artifacts/native-numeric-shipping/`.
Verification logs are `/tmp/incant-native-numeric-{before,script-before,tests,
workspace,clippy,no-compiler,shipping}.log`; local timing raw output is
`/tmp/incant-native-step-measure.log`. Hosted checks remain pending publication.
