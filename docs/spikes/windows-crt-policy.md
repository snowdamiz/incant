# Windows CRT policy and executable doctests

The Windows PR #48 desktop run reached the native runtime's five doctests: four
compile-fail examples passed, but the runnable numeric-column example failed to
link. Its linker command included the editor's build output directory, where
pinned `tauri-build` 2.7.1 had created a synthetic `msvcrt.lib`. Rustdoc requested
the normal dynamic MSVC runtime, found that stub, and failed with LNK4003 and
unresolved CRT symbols including `__CxxFrameHandler3`, `memcpy` and `strlen`.
The editor's package-local replacement linker arguments were absent from this
unrelated runtime doctest. Ordinary runtime test executables had passed.

## Policy

The workspace `.cargo/config.toml` selects `-C target-feature=+crt-static` for
both `rustflags` and `rustdocflags` on Windows MSVC targets. The editor sets the
supported Tauri `build.windows.staticVCRuntime` option to `false`, disabling
Tauri's shim while Cargo supplies real static CRT linkage consistently. Native
tests, executable documentation, the editor, headless tools and shipping player
use this same policy. The existing runnable example and full-workspace test
command remain unchanged.

[Rust's CRT linkage reference](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes)
defines `crt-static` and its build-script target-feature signal. Pinned `cc` 1.6.0
and `aws-lc-sys` 0.45.0 consume that signal for their MSVC native compilation.
[Cargo's configuration reference](https://doc.rust-lang.org/cargo/reference/config.html#targetcfgrustdocflags)
documents target-selected rustdoc flags separately from rustc flags. The pinned
Rust 1.99.0 MSVC target supports static CRT linkage for dynamic libraries too,
including host procedural macros. Tauri documents its separate setting in
[WindowsBuildConfig](https://v2.tauri.app/reference/config/#windowsbuildconfig).

Unlike Tauri's previous hybrid shim, Rust's setting statically links the Universal
CRT as well as the Visual C++ runtime. Executable size can increase; no size or
performance improvement is claimed. This keeps the CRT self-contained rather
than adding a Visual C++ Redistributable installation prerequisite. WebView2 and
Windows system libraries remain editor prerequisites. Future native DLL/FFI
interfaces must return allocations to the runtime that created them and must
not pass CRT-owned objects such as `FILE*` across independent CRT instances.

Cargo must run from the checkout so it discovers the workspace configuration.
Environment `RUSTFLAGS`/`RUSTDOCFLAGS` and their encoded equivalents override
Cargo configuration; custom overrides must preserve the CRT setting. Shipping
verification continues to reject arbitrary environment Rust flags and verifies
the actual Windows runtime/player compiler commands contain `+crt-static`, in
addition to the existing portable-CPU, fat-LTO and abort-profile checks. The
shipping manifest records the CRT flag and hashes the Cargo configuration.

Path-filtered Rust workflows now include `.cargo/**`. Windows desktop CI still
executes the full workspace's doctests, builds the editor and headless executables,
and runs the packaged player. It additionally inspects all three PE import tables
with the pinned toolchain's `llvm-readobj`, rejecting Visual C++/Universal CRT DLL
imports. This catches a missing static policy even on a hosted runner that has
the redistributable installed.

## Verification boundary

On arm64 macOS, Cargo 1.99.0 (`5f94df478`) was also tested with a dependency-free
executable doctest that contains a compile error unless a custom `--cfg` marker
is present. A matching `target.<cfg>.rustdocflags` entry supplied the marker: the
verbose rustdoc command included it and the generated doctest ran successfully.
Omitting that entry made the same doctest fail compilation (exit 101). This proves
the pinned Cargo supports target-selected rustdoc flags and passes them to the
executable's compilation; it is not a Windows execution result. The temporary
probe lives under ignored `target/crt-policy-proof`; verbose positive/negative
logs are `/tmp/incant-pr48-cargo-rustdoc-config-proof.log` and
`/tmp/incant-pr48-cargo-rustdoc-config-negative.log`.

`cargo fmt --all --check`, editor Clippy with `--all-targets -- -D warnings`,
all 352 release workspace tests (including doctests), and all 25 Python tool tests
pass locally. The 44 ignored GPU tests retain their separate CI invocation.
The unchanged runtime doctests include four
compile-fail borrow checks and one executed numeric-column example. LLVM import
output was checked against the pinned WebView2 package's real Windows loader DLL
to confirm both regular and delayed DLL imports are present in the parsed output.
That is parser/tool evidence, not validation of our Windows executables.
The embedded CI Python was also exercised with that real import output, eight
CRT DLL name variants (including unnumbered `msvcrt.dll`/`msvcrtd.dll`) inserted
as delay imports, and empty import output; expected acceptance/rejection passed.

Local check logs are `/tmp/incant-pr48-crt-workspace-tests.log`,
`/tmp/incant-pr48-crt-editor-clippy.log` and
`/tmp/incant-pr48-crt-tool-tests.log`. All Cargo builds use two jobs.

Native Windows linking, PE imports,
PDB packaging and execution require the updated hosted Windows job; they are not
claimed from local checks. No physical Windows machine or signing account is
required for this repair. Device/performance and phase gates remain open.
