# Bounded C compilation for static Wasm builds

The trusted static-bundle feature enables pinned `cc` 1.6.0's `parallel` feature.
Independent generated C translation units now use Cargo's shared jobserver instead
of the crate's serial compilation loop. Existing `-j2` conformance commands cap
the available compilation jobs; `-j1` still serializes them.

The exact Windows conformance feature graph previously enabled only `cc/default`.
The retained checkpoint20 logs report 26.57 seconds for the cooker build,
30m48s for the test build and 26m21s for Clippy's build. Those are historical
Windows observations, not a matched before/after timing result. Actual Windows
execution and timing of this change remain for the release checkpoint.

## Change boundary

Only `incant_wasm_runtime`'s `embedded-fixture` feature mapping changes. The
compiler version, C and Rust flags, safety definitions, prepared module list,
workflow commands, source generation and Cargo.lock bytes are unchanged.
The normal runtime graph remains free of compiler dependencies.

Cargo feature unification means other build-time consumers of this pinned `cc`
in the same selected host graph can also use its jobserver-backed scheduler.
The default runtime selection without `embedded-fixture` still does not enable
parallel compilation. This is build concurrency, with no runtime execution change.

The pinned implementation builds the same object list and passes it to the
archiver in original order after successful compilation. A compile error still
fails the build. MSVC C debug information uses `-Z7`, not a shared compiler PDB;
the unrelated `-Zi` path in this dependency is for assembler inputs.

## Local observed controls

A separate locked Cargo fixture compiled six retained, hash-verified generated
modules and facades plus the three real C runtime sources (15 translation units).
It uses an unchanged copy of the runtime build script and an observer that invokes
actual Apple clang 21.0.0. The observer records compiler intervals and arguments;
it does not delay commands. All three successful cases produced byte-identical
objects and archives, identical archive member ordering and matching compiler
arguments after excluding output-object paths.

| Case | Cargo jobs | Peak C compiler calls | Observed C interval |
| --- | ---: | ---: | ---: |
| Existing serial cc | 2 | 1 | 4.552 s |
| cc parallel | 2 | 2 | 3.236 s |
| cc parallel | 1 | 1 | 6.812 s |

These are single local scheduling observations under concurrent machine load,
not benchmark estimates or a Windows speedup claim. An injected C `#error`
returns Cargo status 101 with the diagnostic retained and no output archive.
The initial observer attempt failed before the scheduling comparison because its relocated
compiler path hid cc's Apple SDK detection. That failure remains retained; the
successful observer preserves the `.xctoolchain/` path classification.

## Fresh native conformance

The ordinary conformance helper freshly prepared all 221 modules. All 1,106
prepared files were directly rehashed while still present. Its eight-package
actual Wasmtime/wasm2c test command passed 492 tests across 89 summary groups
(75 nonempty), with zero failures or ignored cases. All-target Clippy passed;
three fresh development processes returned the exact expected four-tick records.

The actual C build transcripts contain 445 translation units per invocation
(221 guests and facades plus three runtime sources). Three retained invocations
all preserve the existing strict arithmetic, bounds and call-depth flags.
Seven focused Python tests, runtime dependency admission, formatting, conventions
and diff checks also passed. All frozen source hashes remained unchanged during
the conformance run, which finished with status 0; its foreground handle is reaped.

The full command was:

```sh
CARGO_TARGET_DIR="$HOME/.codex/incant-windows-wasm-build-target/conformance" \
  python3 tools/probes/shipping-wasm2c.py \
  "$HOME/.codex/incant-evidence/windows-wasm-build-001/conformance" \
  --conformance-only --profile dev
```

The helper requires a fresh output directory. Its retained receipts contain the
exact three subordinate commands, each keeping Cargo `-j2` and `--locked`.


## Evidence and limits

[The manifest](evidence/wasm-c-parallel.json) binds the
[lossless packet](evidence/wasm-c-parallel-logs.json.gz), which retains source/feature identities, the observed compiler
commands, logs, assertions, failure attempts and raw command receipts. Large
native artifacts and the 422 retained guest files remain at the manifest's
hash-bound external paths; records explicitly distinguish retained artifacts
from the 1,106 prepared files directly checked before the original helper's
normal cleanup. No new Windows, shipping-package, device, frame-performance or
phase gate is claimed.
