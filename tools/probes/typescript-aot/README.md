# TypeScript AOT controls

This directory contains fixed, trusted fixtures for decision 18. It does not
execute project scripts or install an AOT engine host. Its result is not an SDK
conformance percentage or a performance result.

The isolated tool lock pins AssemblyScript and Binaryen. Install it outside the
checkout, without lifecycle scripts, then run into a fresh output directory:

```sh
mkdir -p /tmp/incant-aot-tools
cp tools/probes/typescript-aot/toolchain/package*.json /tmp/incant-aot-tools/
npm ci --prefix /tmp/incant-aot-tools --ignore-scripts --no-audit --no-fund
node tools/probes/typescript-aot/run.mjs /tmp/incant-aot-run /tmp/incant-aot-tools
```

The repository's existing locked Node dependencies must be installed. Each
fixture first passes strict TypeScript and runs through SWC/Node. The runner
then attempts the exact same source with AssemblyScript. Compiler refusals are
retained observations. Any compiled case must match its Node values exactly;
the arithmetic control must compile and match. Compiler invocations have a
60-second deadline, bounded captured output and no project configuration.
Inputs and the installed compiler tree are hashed; the latter is checked again
after execution. `result.json` is written even on failure. Existing output
directories are refused.

`sdk-behavior.ts` separately passes the actual Incant native strict frontend.
It is cooked, not executed by an engine. The arithmetic kernel writes 9,999
triples at four selected ticks; it has no host imports, ECS, controller, physics,
audio, renderer or shipping profile. Node's WebAssembly engine is not the
shipped wasm2c path. No elapsed time from this runner is an adoption benchmark.

The runner also prepares `*.hermes.js` from the same fixtures. They have a tiny
CommonJS export wrapper and print their outputs. They are inputs for a later
pinned Static Hermes compiler run, not evidence that it compiled or executed.
The original `.ts` inputs remain separate: SWC-erased JavaScript does not
establish Static Hermes typed-mode optimization.

## Next Static Hermes execution

Use the exact `static_h` commit in `upstream.json`, with its recorded license and
source hashes. Build tools are intentionally not downloaded or executed by
this probe. After a native compiler slot is assigned, configure a separate
Release CMake/Ninja build and build the upstream `shermes-dep` target. Record
CMake, Ninja, C/C++ compiler, SDK and complete resolved-source identities. The
pinned upstream target provides `shermes`, VM libraries and console bindings.
Use the compiler's `-emit-c` mode on the prepared fixed `.hermes.js` files,
retain generated C and diagnostics, then separately compile/link and execute
under bounded processes. Record compiler help/version with the actual build
before resolving invocation details. The unexecuted recipe is not a build pass.

Before adoption, implement a reviewed sandbox host adapter and turn every row
in `coverage.json` into actual tests against the same native conformance
fixtures. Match current QuickJS behavior including rollback, deadline and
allocation failure, detached numeric views, capability refusal and persisted
state. Do not replace unsupported cases with a narrower SDK.

Then compare bytecode and the candidate on the same immutable 10,000-entity
shipping workload on this Mac, with identical physics, controller, seed, full
functional win, PCM and readback checks, warmed startup and fixed paired run
order. Keep p95/uncertainty and all failed runs. Decision 18 requires at least
2× bytecode performance, independently of the 2 ms script gate. The iOS route
must be inspected/built with no runtime JIT and under 5 MB added binary size;
other-device execution is deferred, not passed. A maintained upstream security
release line must also be established. None of these five adoption clauses is
resolved by a successful language probe.
