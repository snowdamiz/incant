# Shipping runtime profile and crash-symbol packages

The workspace now has a `shipping` profile for the native runtime foundation.
It inherits `release` and explicitly selects optimization level 3, fat LTO, one
codegen unit, abort-on-panic, disabled debug assertions/incremental compilation
and full debug information. The existing `dev` and `release` profiles are unchanged.
The [Cargo profile reference](https://doc.rust-lang.org/cargo/reference/profiles.html)
defines these settings and notes that test harnesses override the panic strategy;
therefore a `cargo test --profile shipping` result would not verify abort behavior.

## Reusable build command

```sh
./tools/cargo build -j2 -p incant_headless --release --locked
./tools/cargo build -j2 -p incant_cook --example cook_scene --release --locked
python3 tools/shipping.py artifacts/shipping-runtime
```

Python 3.11 or newer, the pinned Rust toolchain and its `llvm-tools` component are
required. macOS additionally uses the selected Xcode's `dwarfdump` and `strip`.
The output directory must be new. `--headless-binary` and `--cook-binary` can select
already-built authoring-side tools. `--target` defaults to the compiler's host;
cross-device packaging/execution is explicitly unsupported by this first utility.
Supported hosts are aarch64/x86_64 macOS, GNU/Linux and MSVC Windows.

The utility uses a disposable fresh Cargo target directory, `--locked`, the
shipping profile and two build jobs. It records effective compiler settings from
the real runtime-library and standalone-runner compiler invocations and rejects
incompatible settings, test harnesses and `target-cpu=native`. Compiler wrappers
and arbitrary `RUSTFLAGS` are excluded from this verification path. POSIX builds
use `tools/cargo`; Windows uses the corresponding pinned toolchain's Cargo binary.

`player.zip` contains the standalone `run_scene` executable and a manifest.
`symbols.zip` contains only the matching crash-symbol files and the same manifest.
The manifest records the compiler, target, source revision/input hashes, codegen
settings, executable/symbol hashes, symbol identity and executable checks. A
verification failure exits unsuccessfully without producing the final archives
or a successful manifest. Intermediates are never included in either archive.

## Target-specific symbol handling

The shared profile uses `split-debuginfo = "off"` and preserves debug information
until packaging. Stable Rust's available modes differ by target; setting packed
globally would break targets including Android and WebAssembly. See the
[rustc target support table](https://doc.rust-lang.org/rustc/codegen-options/index.html#split-debuginfo).

| Host | Build/package operation | Required executable-to-symbol match |
| --- | --- | --- |
| macOS | Override split mode to packed, copy the real compiler-generated dSYM, strip debug/local symbols from the player copy | Mach-O UUIDs agree before/after strip and with the dSYM; nonempty DWARF info/line sections; `dwarfdump --verify` succeeds |
| GNU/Linux | Keep linked DWARF, extract with Rust's `llvm-objcopy --only-keep-debug`, strip player, add GNU debuglink | ELF build IDs agree; nonempty DWARF info/line sections; debuglink basename and CRC match the separate debug file |
| MSVC Windows | Override split mode to packed, retain the compiler/linker PDB separately | PE CodeView GUID/age agree with the PDB info stream; DBI contains module/source-file information and symbol records |

The Linux pairing follows the official
[GDB separate-debug-file protocol](https://sourceware.org/gdb/current/onlinedocs/gdb.html/Separate-Debug-Files.html).
Windows metadata validation uses the documented
[PE debug directory](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format#debug-directory-image-only),
[LLVM MSF layout](https://llvm.org/docs/PDB/MsfFile.html),
[PDB info stream](https://llvm.org/docs/PDB/PdbStream.html) and
[DBI stream](https://llvm.org/docs/PDB/DbiStream.html).
Unsupported symbol formats or targets fail explicitly.

## Executable verification and CI

Verification executes the packaged `run_scene`, after its temporary build tree
has been removed. `--build-info` reports its compiled panic/debug-assertion
settings. A deliberate subprocess panic must print the probe marker, terminate
unsuccessfully and skip a Drop guard that would otherwise create a file. POSIX
additionally requires SIGABRT. The probe disables core dumps or unattended OS
crash dialogs; it needs no signing credentials.

The source-free cooked-scene probe then runs that exact packaged executable.
Authoring uses `incant_cmd` through the headless RPC path; cooking must leave the
project and journal unchanged. Both authoring files are deleted before the
standalone runtime starts. Repeated runs must agree, simulation reset must discard
changes, the cooked bytes must remain unchanged and corrupted scenes must fail.

macOS source CI and the Windows/Linux desktop jobs now build/run this path and
upload separate player, symbol and verification artifacts. Hosted results remain
pending until these workflows execute on the submitted commit. Parser fixtures
are unit tests, not evidence of live Windows/Linux symbol production.

## Local verification on 2026-10-10

The actual packaged aarch64 macOS executable is 816,808 bytes. It and the dSYM
share UUID `2b6e714a-2d45-3352-9935-637175315ff7`; the dSYM passed DWARF validation.
The subprocess reported `panic=abort debug_assertions=false`, exited with SIGABRT
(-6) and did not create its unwind marker. All source-free scene checks passed.
Both ZIP member lists and payload hashes were independently checked against the
manifest. The separate archives are 407,853 bytes (player) and 4,245,918 bytes
(symbols); these sizes are artifact inventory, not a performance comparison.

Passed commands:

```sh
python3 tools/shipping.py artifacts/shipping-runtime-verified
python3 -m unittest discover -s tools/tests -v
./tools/cargo fmt --all --check
./tools/cargo clippy -j2 -p incant_runtime --all-targets --locked -- -D warnings
./tools/cargo test -j2 -p incant_runtime -p incant_cook --release --locked
./tools/cargo check -j2 -p incant_runtime --lib --profile shipping --target wasm32-unknown-unknown --locked
./tools/cargo check -j2 -p incant_runtime --lib --profile shipping --target aarch64-linux-android --locked
./tools/cargo check -j2 -p incant_runtime --lib --profile shipping --target aarch64-apple-ios-sim --locked
```

The tool suite passed 24 tests; the runtime/cook suites passed 21. The last three
commands verify portable profile compilation only: they do not produce or verify
device symbol packages, run a device player or satisfy a device gate. Live Linux
ELF and Windows PDB packaging/execution remain pending hosted CI. Source hashes,
compiler flags, binary/symbol identities and exact results are retained in
[machine-readable evidence](evidence/shipping-runtime-2026-10-10.json).

## Scope still open

This is the Transform/Velocity native-world runner, not the completed exported
game or migrated editor play process. PGO, complete export integration, mobile/web
symbol packaging, real-device execution, Tracy captures and matched performance
baselines remain open. Device Farm/reference-phone and phase gates are not
satisfied by desktop execution or portable compilation checks.
