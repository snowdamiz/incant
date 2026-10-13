# Native MetalFX CPU verification

The [contract](native-metalfx.md) is implemented on the private interop branch.
[Independent source review](evidence/native-metalfx-source-review.json) accepted
the exact source003 ownership, encoding, lifetime and fallback contracts with no
findings. Final artifact audit and delegated GPU/appearance acceptance remain
pending. The
[evidence manifest](evidence/native-metalfx.json) binds source003, ten unchanged
baseline files, ten pinned dependency/SDK inputs, all raw/losslessly retained
records and exact executable copies. No GPU was executed.

The ordinary native feature run passed 128 CPU tests with 99 GPU cases ignored.
The default run passed 127 with 98 ignored. The cooked-feature refusal run passed
133 with 41 ignored. These suites overlap and are not added together. Eleven
broad checks passed, including both all-target Clippy runs, default/native web
compilation, iOS native compilation, lean graphs, formatting and conventions.
The non-Apple unused-argument cleanup passed another five affected checks. The
final cache-retention correction also passed five affected checks, all terminal
and reaped. These final checks completed after the independent source review;
that receipt retains its original source-only scope. Source001 and its unchanged
successful results remain retained. The five
pre-existing WebAssembly timing warnings remain root-owned and unchanged.

The final copied native library-test executable is 32,071,488 bytes, mode 0755,
SHA-256 `53ecd261e03fb21655519969123806d49d3489645ce4bf1e504f9d54ed23e671`.
Its one new ignored retained test is bounded to six 64x64 queue submissions,
including five native three-buffer batches and one portable batch, plus one
non-submitted unsupported-format refusal. That test checks native admission,
accepted geometry/history ownership, fallback, reset and replacement. It does
not establish native output quality or screen-space appearance. Any execution
requires a separately granted, supported Metal device and an explicit attempt
budget; fallback must not count as native success.

Early compiler/test diagnostics are preserved, not replaced by later green logs.
No current-main merge, phone/device performance, MetalFX internal memory budget,
cooked-native compatibility, or full renderer completion is claimed here.
