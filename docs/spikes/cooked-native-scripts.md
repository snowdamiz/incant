# Build-produced native script artifacts

This Phase 1 slice cooks a single native bulk behavior into the executable. It
implements startup without JavaScript or TypeScript parsing on the existing
`incant_script` host. It does not migrate the legacy PlaySession or implement a
general export, authenticated cache loader, portable bytecode ABI or phone gate.

## Executable trust and compatibility

`incant_cook::cook_script` receives bounded CommonJS from the pinned SWC authoring
step and compiles it with rquickjs 0.14.0 (its pinned QuickJS-NG 0.16.2, bytecode
version 28). Compilation does not execute project code. It also compiles the
engine's fixed buffer-hardening module. Source text is stripped from both blobs;
debug line tables and a bounded module name remain.

The output is a generated Rust module for a trusted native build. It contains
only escaped Rust string literals, numeric byte arrays and fixed engine code.
The native player's engine-vetted build script recompiles source every time its
inputs change; it does not import project Rust, run project build scripts, or
accept bytecode caches. Project source is data supplied to QuickJS's bounded
compiler. The generated module crosses the explicitly unsafe
`CompiledScript::from_embedded` boundary. Its caller must establish compiler
provenance for both behavior and fixed hardening bytes. There is no safe loader
for arbitrary bytecode, even if a caller supplies matching hashes.

Before QuickJS sees an embedded module, the loader checks artifact version,
engine identity, exact target triple, conservative build identity, wrapper
layout, names, sizes and SHA-256 integrity. The build identity hashes the pinned
workspace lock/manifest and the script manifest, wrapper, loader, host,
metadata contract and hardening source. It deliberately invalidates artifacts
more often than a portable ABI would. It is **not authentication**. A unit test
shows that arbitrary bytes can have matching hashes, without ever feeding those
bytes to QuickJS. The compiler's package versions are checked by build.rs so an
engine update requires an explicit compatibility audit.

The first player requires native `HOST == TARGET`; the cook also rejects a target
other than its actual compiler host. Cross-target exports are unsupported until
separately verified. This does not change the independent iOS/Android native-host
compile checks. All embedded allocations remain immutable and alive until the
host's VM has been destroyed.

## Compiler and runtime separation

The `compiler` feature contains `compile`, `compile_named`, Rust emission and
source-parser context creation. It remains the default for direct development
use of the host crate; workspace consumers explicitly opt in when needed.
`incant_script_player` does not enable it. The script-only `incant_cook` build
feature omits document, scene-authoring and CRDT dependencies, avoiding that extra
build graph. Existing default scene-cook behavior is unchanged. The runtime
dependency guard now forbids `incant_cook` on normal edges and checks the player;
build/dev edges remain explicitly excluded.

Embedded instances always omit QuickJS's Eval intrinsic, even in a workspace
build whose other consumers enable the compiler feature. Direct/indirect eval,
Function and async/generator constructors fail rather than compile source.
Hardening loads from bytecode before the guest Promise hook is installed; the
existing two-internal-promise guest-module baseline and zero-promise tick policy
remain unchanged. Direct numeric loans, permanent command revocation and atomic
rollback continue through the same canonical host. No second host is introduced.

Pinned QuickJS retains `JS_EvalInternal`, a small optional-eval dispatcher which
throws when no parser is installed. The actual parser is `__JS_EvalInternal`.
The first binary audit incorrectly rejected the dispatcher; source inspection
corrected that distinction. The final audit checks actual compiler/parser
symbols and executes dynamic-source rejection tests inside the real player.

## Bounds and diagnostics

CommonJS source is capped at 1,000,000 bytes, source names at 256 bytes without
control characters, each bytecode blob at 4 MiB, and an optional version-3 source
map at 4 MiB. Indexed maps and malformed map structure fail explicitly. The
compiler retains the existing 32 MiB QuickJS allocation cap, 1 MiB stack limit
and 50 ms thread-CPU deadline. Host tick limits and the 65,536 structural-command
cap are unchanged. The player bounds scene file acquisition to 128 MiB before
the binary scene decoder applies its own stricter format/count checks, and caps
its requested run at 10,000,000 ticks.

The symbol sidecar records module name, source/bytecode/hardening/map hashes,
compiler/target/build identity and the three-line CommonJS wrapper offset. SWC
maps are retained outside the player, including source content when SWC emits
it. They are data, not executable caches. Automatic stack/source-map remapping
is still unsupported: current runtime diagnostics remain bounded error messages.
No new credential, signature or authentication claim is made.

## Usage and proof

For a native desktop build, first transpile TypeScript with the existing pinned
SWC command, then pass only source/map paths to the trusted build:

```sh
node tools/build_script.mjs behavior.ts /absolute/path/behavior.js
INCANT_NATIVE_SCRIPT_SOURCE=/absolute/path/behavior.js \
INCANT_NATIVE_SCRIPT_MAP=/absolute/path/behavior.js.map \
INCANT_NATIVE_SCRIPT_NAME=behavior.js \
./tools/cargo build -j2 -p incant_script_player --profile shipping --locked
```

With no build-time override, the package embeds its small motion/structural
fixture. The executable accepts only `<checked-scene> <ticks>` (or build info),
never a script path. The broader scene-to-behavior asset mapping/export UI is
outside this slice.

`tools/probes/cooked-script.py` runs the native structural TypeScript example
through SWC, cooks it in a fresh Cargo build, checks the actual target rustc
commands and binary symbols, packages matched native symbols, then removes all
project/journal/TS/CommonJS inputs and the temporary build tree before launching
the copied player. Source maps remain in the separate symbol directory. Twelve
fresh processes at 0/1/2/3/4/60 ticks verify repeatable component writes and
spawn/velocity/reparent/despawn transitions without modifying scene bytes.
A hostile neighboring cache and extra script argument are also covered by the
player's process test.

The initial verified shipping proof on 2026-10-10 used Rust 1.99.0 on arm64 macOS,
a fresh portable fat-LTO/one-codegen-unit/panic-abort build, and produced a
1,790,136-byte stripped player. QuickJS's bytecode reader/evaluator remained;
its actual source parser/enablement and Incant compiler symbols were absent.
The matching dSYM UUID is `1a3110d0-d648-3e71-a15a-9f53e4a1451d`. This is executable
and size evidence, not a device or engine-performance measurement. The probe's
symbol audit currently supports Mach-O/ELF; MSVC PDB parser-symbol inspection is
explicitly unsupported. Existing Windows workspace tests exercise the player,
but hosted results for this change remain pending publication.

The general macOS CI job runs this fresh proof and uploads its evidence without
caching the disposable build tree. The verified fresh local build took 43.29 seconds;
this is an uncontrolled local duration, not a CI speedup. Script-only cooking
avoids an additional scene-authoring/Loro build-script graph. Actual incremental
shared-cache sizes still require hosted measurement against the recorded budget.

Full release workspace verification passed 382 tests/doctests, with the existing
44 GPU tests ignored by the ordinary suite. Full workspace/all-target Clippy and
the separate compiler-disabled host Clippy check passed with warnings denied.
All 25 Python tool tests, runtime/player dependency guards, formatting,
generated-convention checks and workflow lint passed. No external dependency
version or checksum changed in Cargo.lock.

The compiler-disabled host also compiled for `aarch64-apple-ios-sim` with Xcode
27.0 (27A266a), Simulator SDK 27.0, and `aarch64-linux-android` with NDK r28c
28.2.13676358/API 21. Hosted Android CI uses NDK 28.0.13004108, so this local check
does not claim an identical hosted SDK. These are host compilation checks, not
mobile player export, device execution or reference-phone performance evidence.

Raw proof: ignored `artifacts/cooked-script-native-verified/`; test logs:
`/tmp/incant-cooked-script-focused.log`,
`/tmp/incant-cooked-script-player-tests.log`,
`/tmp/incant-cooked-script-clippy.log` and
`/tmp/incant-cooked-script-shipping-verified.log`. Full verification logs:
`/tmp/incant-cooked-script-workspace.log`,
`/tmp/incant-cooked-script-workspace-clippy.log`,
`/tmp/incant-cooked-script-no-compiler-clippy.log`,
`/tmp/incant-cooked-script-tools.log`, `/tmp/incant-cooked-script-ios.log` and
`/tmp/incant-cooked-script-android.log`.

### Direct-write validation follow-up (2026-10-10)

A native-session regression reproduced a missed host defect on the reviewed
baseline: with no structural commands, a callback writing `NaN` into Transform
returned success. The old-host regression failed before the fix. After all JS
views are revoked, the host now reads the actual numeric columns through the safe
native visitor and checks every Transform and Velocity. Values must be finite,
quaternion squared norm must be within `1e-4` of one, and each scale magnitude
must be at least `1e-9`, matching native component construction. Invalid data
returns the existing typed component error; the uncommitted loan restores the
entire numeric write set before scope refresh and permanently fails the host.
No JSON, authoring validation or new unsafe code is involved. This adds a bounded
linear scan of the already limited numeric loan (maximum 32 MiB); it is not a
measured performance claim.

Observable regressions cover NaN/Infinity, quaternion norm/overflow, zero/tiny
scale and nonfinite Velocity across Transform-only and moving tables, complete
undo and detached retained views. Signed unit quaternions, reflected scale at the
threshold and finite velocity remain accepted. The fix checks component domains;
it does **not** resolve the existing native simulation/hierarchy overflow case
where individually finite, huge parent/child values compose to a nonfinite world
matrix. Native schedule integration and scope-derived refresh remain unchecked
for that case; this is not full hierarchy-domain validation.

Verification on the corrected source:

```sh
./tools/cargo test -j2 -p incant_script -p incant_script_player -p incant_cook --locked
./tools/cargo clippy -j2 -p incant_script -p incant_script_player -p incant_cook --all-targets --locked -- -D warnings
./tools/cargo clippy -j2 -p incant_script --no-default-features --all-targets --locked -- -D warnings
python3 tools/probes/cooked-script.py artifacts/cooked-script-numeric-verified
```

All 35 scoped tests/doctests passed, including 27 host tests; both Clippy commands
passed. The earlier 382-test workspace result belongs to the pre-follow-up tree;
required hosted checks will test the new PR head. The fresh shipping proof again
passed all twelve source-free processes, compiler-symbol/dynamic-source checks,
and matched dSYM packaging. The player remains 1,790,136 bytes (zero byte-size
delta), with new SHA256
`68153759c792c171c3229ad5956fb49e8b00ca819faf9b77cf93e303b58ebe40`
and arm64 dSYM UUID `4f548a69-4db5-38aa-a1a1-3b5cbcd3e16a`.
[Recorded result](evidence/cooked-native-script-numeric-2026-10-10.json) preserves
the previous proof separately. Logs are `/tmp/incant-cooked-numeric-tests.log`,
`/tmp/incant-cooked-numeric-clippy.log`,
`/tmp/incant-cooked-numeric-no-compiler.log` and
`/tmp/incant-cooked-numeric-shipping.log`. Mobile compile results above belong to
the earlier tree and do not certify this corrected source or device execution.

### Rollback borrow correction (final pre-publication source)

Deeper independent review found that revisiting columns for numeric validation
creates fresh mutable slices, so restoring afterward through cached pointers
from the initial visit would have an invalid Rust aliasing/provenance argument.
Loan destruction now revokes all JS buffers, verifies the complete saved IDs and
numeric layouts through a safe visitor, and restores undo with `copy_from_slice`
through another fresh visitor. It never dereferences the original raw component
pointers after a subsequent mutable visit. This removes the unsafe rollback
copy; foreign buffer creation remains the only use of those retained pointers.
Existing invalid-write, callback-error, deadline/allocation, deferred-work and
partial-exposure regressions exercise this rollback path.

The same 35 scoped tests/doctests and both Clippy modes above pass on this final
correction; the full workspace and mobile results still belong to the earlier
reviewed baseline. A third fresh shipping proof passed the unchanged twelve
source-free processes and all compiler/dynamic-source/symbol checks. Final
stripped size is 1,790,136 bytes, SHA256
`3f7a881619fb8d065cbc25ed9c15d9f3d3324ab928cdabd8a292bb0a0b675f41`,
with matched dSYM UUID `df418244-4bfe-3281-9421-e4ae3a5b6051`.
[Final proof](evidence/cooked-native-script-safe-undo-2026-10-10.json) is preserved
separately from both earlier proofs. Raw output:
`artifacts/cooked-script-safe-undo-verified/`; logs:
`/tmp/incant-cooked-safe-undo-{tests,clippy,no-compiler,shipping}.log`.
The huge-finite simulation/hierarchy limitation above remains open.
