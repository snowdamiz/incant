# MSVC static Wasm conformance follow-up

This follow-up configures native x64 `cl.exe` for the existing pinned wasm2c
bundle and adds a real Windows execution job. **Hosted Windows execution is
pending; configuration and macOS checks do not establish Windows support.**
The completed macOS slice and its exact binary remain recorded in
[static Wasm evidence](shipping-wasm2c.md).

The translator and six vendored runtime files remain WABT 1.0.42 without source
changes. Its [spec runner](https://github.com/WebAssembly/wabt/blob/1.0.42/test/run-spec-wasm2c.py)
already invokes `cl.exe` on Windows; its C11 mode enables experimental atomics.
The pinned runtime uses Windows setjmp/longjmp and compiler TLS, and generated
code has MSVC intrinsics. Only `x86_64-pc-windows-msvc` with native `cl.exe` is
accepted here; neither GNU Windows nor clang-cl substitutes for it.

The default C baseline uses `/Od`, `/TC`, `/std:c11`,
`/experimental:c11atomics` and `/fp:strict`. This is an interim conformance
baseline, not fulfillment of the optimized shipping path. Setting the trusted
build input `INCANT_WASM_MSVC_OPT_LEVEL=2` selects an `/O2` candidate; all other
values fail. The probe exposes this as `--msvc-opt-level 2` and records the
actual discovered compiler path/version, selected optimization, Rust codegen,
executable hash and matched PE/PDB identity. The workspace's static CRT policy
remains in force for Rust and C.

[Microsoft's floating-point documentation](https://learn.microsoft.com/en-us/cpp/build/reference/fp-specify-floating-point-behavior?view=msvc-170)
describes `/fp:strict` as retaining source ordering/rounding and respecting the
floating-point environment, without FMA contraction. This is paired with a real
separate multiply/add regression. It is not a claim of complete deterministic
floating-point or NaN conformance across platforms.

Bounds checks, fixed memory/table limits, private fuel and the explicit 64-call
TLS depth check stay enabled in both configurations. The recursion test now
requires the depth trap specifically rather than accepting fuel exhaustion as
equivalent. The same suite also executes direct and indirect infinite loops,
startup traps, bounds/import errors, numeric/hierarchy rejection, structural
rollback, concurrent instances and exact Wasmtime/static state parity. No C
facade, runtime callback, unsafe descriptor or capability semantics changed.
Traps still jump only between C frames after any Rust callback has returned;
this does not use Microsoft C++ exception unwinding across Rust.

The Windows probe provisions only the exact named tools from the official
`wabt-1.0.42-windows-x64.tar.gz`, SHA-256
`62f1eb2b51aa57cf0f0b8ba333bf2c72a049930aa9c5de01dcad271c4fe48c88`.
It uses the pinned `cargo.exe`/Rust toolchain. Since a linked PE normally lacks
the COFF symbol table used by `llvm-nm`, the final link emits a fresh MSVC map
for the translated-code/compiler-symbol audit. The existing PDB parser verifies
the executable's GUID/age and real debug records before separate packaging.
Normal dependency and actual target compilation checks independently exclude
the development JIT/compiler. The C-flag audit retains its trusted-environment,
global-presence limitation; it is not an adversarial toolchain audit.

CI runs `/Od` conformance, then `/O2` conformance on the same development graph,
followed by one fresh `/O2` shipping build and source-free process proof. The
complete original 13 development tests, Clippy and actual Rust guest probe
remain in that combined helper. macOS/Linux execution and default/mobile
isolation checks remain. No second cold `/Od` shipping build or new dependency
cache is added.

Local checks before integration: 25 combined tests/doctests, combined Clippy,
three development guest processes, all seven static release tests (including
the stronger recursion assertion), final static release Clippy and 44 Python
tool tests pass. The new tool tests reject altered download checksums, extract
only the two verified executable names, reject missing/compiler symbols, and
require strict arithmetic, bounds and CRT flags in the audit. Actionlint,
formatting and generated conventions pass. These local tests execute macOS,
not MSVC. Integrated source/proof identities and actual hosted results will be
recorded separately before claiming the Windows gap is closed.

## Runtime integration check

The follow-up was applied to PR62's exact `d8d00c3` source as `fa408d0`; only the
status ledger needed reconciliation. The combined helper passes 25 tests,
Clippy and three real development guest processes on that union. All 44 tool
tests and the six-root authoring dependency guard pass.

`python3 tools/probes/shipping-wasm2c.py /tmp/incant-wasm2c-msvc-union-shipping --rustup /opt/homebrew/bin/rustup`
passes on macOS arm64 at `fa408d0`, with three exact-motion processes and 18
refusals after temporary source/Wasm/C/tool/build deletion. The executable is
2,797,360 bytes, SHA-256
`b75544b640fde6396fc458b98420fb60398a578092c5153d4a75de85ef38fb29`,
with separately packaged arm64 dSYM UUID
`f856c2c3-89a3-3d44-a28c-2d2da79c456d`. The
[local evidence](evidence/wasm2c-msvc-local-2026-10-10.json) records the exact
commit, sources, logs and report. This is regression evidence for the existing
macOS path after integrating the runtime; it does not execute either MSVC
configuration. Hosted Windows and optimized candidate acceptance remain pending.

No phone performance, distribution signing, full SDK/export migration,
arbitrary-source build service or phase gate is claimed.
