# Strict C11 declarations for the Linux Wasm trap bridge

PR62 job 114359756090 reached static/development linking and failed with
`undefined symbol: sigsetjmp` from the generated C facades. Pinned WABT 1.0.42
uses `sigsetjmp`/`siglongjmp` outside Windows. Under strict `-std=c11`, glibc
hides their POSIX declarations unless a feature-test macro enables them.
The [glibc documentation](https://sourceware.org/glibc/manual/latest/html_node/Feature-Test-Macros.html)
requires these macros before system headers and documents `_POSIX_C_SOURCE`.

The correction defines `_POSIX_C_SOURCE=200809L` for Linux C translation units.
Configured Unix GCC/Clang builds also reject implicit function declarations,
so this class of missing declaration fails during compilation. Vendored files,
C-only trap targets, signal-mask restoration, callback lifetimes and numerical
flags remain unchanged. Windows retains its existing separate configuration.

A disposable Ubuntu24.04 arm64 container using GCC13.3/glibc2.39 reproduced the
same link failure without the define, then compiled and executed three actual
C-only trap round trips with the define and error flag. The fixture includes
the real `imports.h` and links the unmodified `wasm-rt-impl.c`; it uses the same
fixed-memory/bounds/depth/Segue defines as the runtime. Link-only `-lpthread`
keeps compilation equivalent to the actual cc invocation. An initial experiment
using `-pthread` did not reproduce the defect because it enables the required
POSIX declarations itself; that experiment is not the negative control.

On macOS Clang, the combined helper after this correction passes all 25 tests,
Clippy, and three real development guest processes. The seven compiled cases
include startup/bounds/recursion traps, loops, rollback and concurrent instances;
static/development parity is preserved. Full Linux x64 Rust conformance and its
fresh shipping proof remain pending the corrected hosted job. The local C probe
is not a substitute for those checks or a device/performance gate.

[Exact source, fixture and log identities](evidence/wasm2c-linux-c11-2026-10-10.json).
