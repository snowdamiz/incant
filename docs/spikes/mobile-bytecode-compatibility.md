# Pinned mobile bytecode compatibility

This slice adds a closed trusted-source cook from `aarch64-apple-darwin` to
`aarch64-apple-ios` and `aarch64-apple-ios-sim`. It does not admit arbitrary
retargeting, project-supplied executable caches or Android cross-target execution.
Authored project export was outside this checkpoint; the separate
[authored iOS archive route](authored-ios-project-export.md) records its later proof. Local native checks and actual iOS simulator execution pass on the retained
immutable source. iOS device archives also compile and link to both callers, but
were not executed on a device. Independent final execution review is accepted.
These results do not establish physical-device performance or a phone gate.

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
records. The separate mobile caller harness uses one immutable target archive
for its C and Swift callers, retaining archive/build/source identities and comparing
all fixture messages. Its trusted build uses `INCANT_NATIVE_SCRIPT_SOURCE` with
this fixture's absolute path and `INCANT_NATIVE_SCRIPT_NAME=mobile-compat.js`.

The existing `embed_project` and desktop exporter host-equals-target guards stay
in place; the later authored iOS route uses a separate explicit producer. Android cross-target production admission stays closed until an actual matching
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
retained freezes 001 and 002. The subsequent target evidence below closes the simulator execution check.


## Actual target evidence

[Target packet](evidence/mobile-bytecode-compatibility/target-proof.json) retains the
exact build commands, generated bytecode/metadata, source freeze, caller requests,
reports and command streams. All three archives use the `ci` profile and exactly
`mobile-ffi`. The compiler is commit `a54a9da5`; the immutable tested union
`24363a10` composes reviewed mobile/headless source `45d7ffee`. The separate caller
harness is unchanged commit `270294e5`. All 10,754 tracked files were unchanged through the archive builds. This
closeout changes documentation/evidence afterward; compiler and fixture source
hashes remain identical.

| Destination | Actual result | Caller commands / controls |
| --- | --- | --- |
| macOS ARM | C and Swift execute all numeric/input/failure cases | 39 / 32 |
| iOS ARM simulator | C and Swift execute the same cases on iPhone 18 Pro / iOS 27.0 simulator | 44 / 32 |
| iOS ARM device | Archive compilation and both caller links only | 7 / 0 |

The disposable simulator was created, booted, used through `simctl spawn`, shut
down and deleted; no Simulator window or GPU proof was used. Each target has one
retained archive used by both callers. The three independently fresh cook
manifests differ only in their exact destination. The captured generated arrays
were independently rehashed to the recorded bytecode and hardening hashes.

All eight complete game reports (held, repeated, released and deliberately failed
observation runs for each language) match exactly between macOS and simulator,
including 15 typed log records per report, entity observations and PCM hashes.
The source-specified vectors are checked field by field by the caller harness.
Malformed requests, null/oversized buffers, missing input/package, corrupt package,
create-only output and the post-write guest failure retain their exact expected
statuses and report/no-output behavior. The full results and commands are retained;
no aggregate checksum substitutes for the numeric records.

Union all-target Clippy for script/cook/player, the mobile runtime dependency guard,
formatting and diff checks also pass. Nine larger archives/caller executables remain
hash-bound at durable paths outside `/tmp`; their bytes are not duplicated into the
repository packet. Independent source review accepted the compiler and fixture;
[Final independent target-evidence review](evidence/mobile-bytecode-compatibility/final-review.json) is accepted. The original failed native fixture
compilation and the pre-correction ABI test freeze remain preserved.
