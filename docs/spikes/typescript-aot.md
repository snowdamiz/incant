# TypeScript AOT language and host spike

Decision 18 remains open. This spike pins Static Hermes first and AssemblyScript
as the typed-to-Wasm alternative, runs actual bounded language controls, and
leaves a runnable path toward the required native host comparison. No engine
host, shipping benchmark, iOS artifact or candidate adoption is claimed.

## Actual observations

The current strict TypeScript 5.9.3 frontend and SWC 1.16.13 accept a real
`NativeBehavior` fixture using immutable row metadata and fresh numeric views.
That fixture is cooked only. Six independent language fixtures also pass strict
TypeScript and execute their expected values in Node v22.23.3. Node is a trusted
control, not the Incant sandbox.

AssemblyScript 0.28.20 actually rejects five of those same-source fixtures:

| Current requirement | Narrowed compiler observation |
| --- | --- |
| Recursive primitive/array/object props | Union types not implemented |
| WeakMap metadata keyed by frozen IDs | WeakMap missing; Object.freeze missing |
| Captured state in callbacks | Closures not implemented |
| Catching a thrown error | Exceptions not implemented |
| Iterating a Float64Array with `for...of` | Iterators not implemented |

The initial run also exposed optional-catch syntax and typed-array constructor
incompatibilities. Both logs remain intact. Two fixtures were narrowed to a
named catch parameter and explicit array writes; the successor then produced
explicit exception/iterator diagnostics. These are current direct-compilation
gaps, not a rejection of every possible language lowering or future adapter.
A lowering would need its own reviewed semantics and full conformance proof.

A separate fixed arithmetic kernel writes 9,999 triples. It compiles to a
4,926-byte Wasm module and matches JavaScript sums and first/last values exactly
at ticks 1, 32, 64 and 120. This has no ECS loan, host command, physics, controller,
audio or shipping profile. Node WebAssembly execution does not establish native
wasm2c performance, sandbox safety, mobile size or no-JIT execution.

## Pinned candidates and security prerequisite

Static Hermes is pinned to `static_h` commit
`1d2fc48c021fbda8eb14478379112ed67bed8f0f` (MIT). Its
[compiler source](https://github.com/facebook/hermes/blob/1d2fc48c021fbda8eb14478379112ed67bed8f0f/tools/shermes/shermes.cpp)
provides C output, disables lazy compilation in this path, and explicitly
refuses combining TypeScript stripping with typed mode. The
[build target](https://github.com/facebook/hermes/blob/1d2fc48c021fbda8eb14478379112ed67bed8f0f/tools/shermes/CMakeLists.txt)
uses a host C compiler and distinguishes host compiler from target VM during
cross-compilation. No installed `shermes` was found and no C/C++ build was run;
neither fact is an adoption verdict. Generated `.hermes.js` inputs and a
bounded next-build recipe are retained. Erased JS and original typed source
must be evaluated separately rather than assuming typed-mode optimization.

AssemblyScript v0.28.20 is pinned to source
`b6bda05c29c9eb7d07d7f6cb2703508483b30493`; its isolated npm lock also pins
Binaryen `131.0.0-nightly.20260721` and long 5.3.2. Installation disabled lifecycle
scripts. The installed compiler tree was hashed before and compared after
execution. No root dependency/lock or production runtime changed. The
[upstream status page](https://www.assemblyscript.org/status.html) describes the
language subset; the rows above are local compiler observations.

The security-release clause is unresolved for both candidates. Hermes has a
[Meta reporting policy](https://github.com/facebook/hermes/security/policy), but
that does not bind security fixes to the selected `static_h` revision. The
retained general release listing is not assumed to be its release channel.
AssemblyScript has recent versioned releases; its
[repository policy page](https://github.com/AssemblyScript/assemblyscript/security/policy)
did not expose a SECURITY.md. Absence of that document does not prove absence of
security maintenance. Establish an upstream supported release/update policy
before treating decision 18's security prerequisite as satisfied.

## Host contract and next execution

[coverage.json](../../tools/probes/typescript-aot/coverage.json) enumerates every
current `NativeContext` member and the relevant existing engine test roots. Its
member list is checked against the actual SDK. All AOT host rows remain pending:
revocable typed-array loans, callback expiry, poisoned transactions and rollback,
allocation/deadline refusal, immutable props, persistence, physics/navigation,
input/timers/localization, HUD/audio/animation, haptics and restart. Existing
Wasm parity gaps must not silently shrink the TypeScript SDK requirement.

Build a pinned Static Hermes compiler when a native compiler slot is assigned,
then run the retained inputs and implement the reviewed host boundary. Generated
native code must not bypass allocation/work bounds or expose unchecked ECS
pointers. Run every current SDK contract before a matched Mac shipping comparison.
For AssemblyScript, first demonstrate any proposed lowering/ABI bridge without
changing the source-language contract. Keep QuickJS as the shipped default while
this work is incomplete.

[README and runner](../../tools/probes/typescript-aot/README.md) provide the fixed
commands and benchmark admission plan. The five unchanged adoption conditions
are: 100% SDK conformance; at least 2× bytecode on the matched Mac shipping script
benchmark; no iOS JIT; under 5 MB added mobile binary; maintained upstream security
releases. The separate 2 ms p95 script gate is not replaced by the 2× comparison.
Other-device execution remains deferred under the current policy. No timing or
size of this isolated language control satisfies these conditions.

## Retained evidence

[Manifest](evidence/typescript-aot/manifest.json) and its lossless archive retain
both run generations, raw compiler diagnostics, source copies, generated Hermes
inputs, the Wasm control, tool identities and pinned upstream source/license
copies. Historical two-fixture bytes were reconstructed from the exact small
syntax changes and checked against their original recorded hashes; the other
source copies are unchanged. No historical execution is relabeled.

```sh
python3 tools/probes/typescript-aot/collect.py --verify-only
python3 tools/probes/typescript-aot/collect.py --verify-only --external
python3 -m unittest discover -s tools/probes/typescript-aot -p 'test_*.py'
```

The verifier performs no compiler execution. The external option additionally
checks retained original files; portable replay needs only the repository.
Independent source/final review is pending. This packet changes no production
runtime and approves no phase gate.
