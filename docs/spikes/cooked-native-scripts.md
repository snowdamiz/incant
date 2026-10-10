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
