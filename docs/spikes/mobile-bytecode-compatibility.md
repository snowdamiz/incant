# Pinned mobile bytecode compatibility

This slice adds a closed trusted-source cook from `aarch64-apple-darwin` to
`aarch64-apple-ios` and `aarch64-apple-ios-sim`. It does not admit arbitrary
retargeting, project-supplied executable caches, Android cross-target execution, or authored
project cross-target export. Local native checks and the target execution evidence
are separate: the iOS simulator execution proof is pending at this source freeze.
Device compilation will not establish physical-device performance or a phone gate.

## Producer and destination contract

Artifact format 3 records the actual producer target, the exact destination and a
numeric bytecode ABI beside the existing engine/build/source/bytecode/hardening
identities. Format 2 products require a fresh trusted cook. The destination loader
requires its own exact target, the admitted producer pair, matching ABI fields,
and the version returned by the actual linked QuickJS library before it reaches
unsafe bytecode loading. Hashes remain integrity checks, not authentication.

The only new constructor, `ScriptBuild::compile`, accepts bounded source and a
destination. It compiles the source and hardening module afresh and checks both
bytecode headers. It cannot accept a `CompiledScript` or executable cache, and it
cannot execute as a producer-runtime `CompiledScript`. Its generated Rust embeds
the captured producer and destination; it does not replace a target string in a
previously emitted executable artifact. Native source compilation retains the
existing `CompiledScript` route.

Admission requires bytecode version 28, 64-bit pointers, little endian, the
16-byte/alignment-8 `JSValue`, 32-bit atoms, IEEE binary64 precision/exponent,
bytecode-plus-strip-source serialization flags, and the audited default C flags.
External `CFLAGS`, host/target variants and target-specific cc overrides make the
new cross-target route fail explicitly. The existing native route remains
available. Neither shared-buffer nor object-reference serialization is enabled.
Build identity now also covers the compatibility implementation and build script.

The build asserts exact registry origin, version and checksum for all three
rquickjs 0.14.0 packages. That pins QuickJS-NG 0.16.2. Its serialization code writes
function opcodes, remapped atom indexes, scalar values, strings and BigInt bytes;
it does not capture runtime function addresses. The shared-buffer pointer branch
is not enabled. Its fixed-width native byte ordering is why this route admits
only the reviewed 64-bit little-endian pair; obsolete byte-swap flags do not
establish arbitrary architecture compatibility. The retained source audit hashes
the actual dependency files used for this inspection.

QuickJS itself cautions that bytecode is version-specific and is not verified as
safe input. The existing unsafe loader's trusted-build requirement remains in
force ([QuickJS-NG developer guide](https://quickjs-ng.github.io/quickjs/developer-guide/intro/)).
This audit is not a claim of bit-identical arbitrary libm results, resource-budget
costs or allocator behavior across operating systems.

## Observable execution fixture

`crates/incant_script/tests/fixtures/mobile_compat.js` is separate from the default
player fixture. Its adjacent expected JSON contains independently constructed
per-tick records, not an aggregate checksum. Three held/released ticks cover:

- IEEE bit patterns including signed zero, the smallest subnormal, maximum finite
  value, exact-integer boundary, one third, epsilon and a rounded float32 value;
- wide signed BigInts, BigInt64/BigUint64 typed-array reinterpretation and explicit
  low/high word order;
- captured closure state, Unicode regular expressions and normalization;
- exact native tick/dt/input records, held/released native velocity behavior;
- disabled `eval` and `Function`, plus a tick-2 failure after buffered log and
  velocity writes that makes the session unavailable.

The common native test runs freshly embedded trusted bytecode against these
records. The separate mobile caller harness will use one immutable target archive
for its C and Swift callers, retaining archive/build/source identities and comparing
all fixture messages. Its trusted build uses `INCANT_NATIVE_SCRIPT_SOURCE` with
this fixture's absolute path and `INCANT_NATIVE_SCRIPT_NAME=mobile-compat.js`.

The existing `embed_project` and authored exporter host-equals-target guards stay
in place. Android cross-target production admission stays closed until an actual matching
emulator execution has been independently reviewed. iOS simulator execution,
iOS device archive compilation, ordinary game packaging, signing and hosted
physical-phone measurements remain distinct evidence categories.

## Native validation before target execution

The retained [native packet](evidence/mobile-bytecode-compatibility/native-initial.json)
records 181 script tests across eight groups and eight selected cooker/project/
TypeScript tests across three groups, with no failures or ignored tests. Scoped
all-target Clippy, a native player build, runtime dependency guard, formatting and
diff checks pass. The first fixture test compilation used a private log field;
that failed attempt is retained and the test was corrected to the public accessor.
Independent source review requested a portable positive ABI test instead of
assuming the test runner itself always has the reviewed ARM ABI. That test-only
correction passes all six artifact tests; production hashes are unchanged between
retained freezes 001 and 002. Target execution remains pending.
