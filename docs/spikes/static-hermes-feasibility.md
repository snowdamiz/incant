# Static Hermes native feasibility

The pinned `facebook/hermes@1d2fc48c021fbda8eb14478379112ed67bed8f0f` compiler
built, and seven unchanged JavaScript fixtures matched exact Node values through
generated C units with eval/JIT disabled. The one negative deadline probe needed
external termination: `watchTimeLimit(10)` did not stop its native loop before
the two-second supervisor deadline. This does not establish safe engine
interruption or rollback. QuickJS remains the shipped TypeScript default;
decision 18 is unchanged.

The run used Incant `489d2977ed28015d58abc2f48ef62493dc2f8751`, Mac17,9 / Apple
M5 Pro / arm64 macOS 27.0.1 (26A434), Apple Clang 21.0.0, privately unpacked
CMake 3.31.8 and Ninja 1.13.2. All 12,761 source file/symlink identities matched
the pinned Git inventory; source bytes were reverified after build. Official
archive hashes, exact toolchain/runtime/host identities and recorded commands
are bound by the [evidence manifest](evidence/static-hermes-feasibility/manifest.json).

One network-denied configure and `shermes-dep --parallel 2` build succeeded.
The initial monitor counted only the original process group, omitting Ninja's
separate job groups. Its recorded RSS is not aggregate build RSS, so historical
resource compliance remains unqualified. The build was not repeated. A corrected
tracker uses session membership and PID/start-time identities, seals session
admission before leader reap, and passed actual Ninja, escaping-grandchild,
cross-group RSS and stale-identity controls. This is trusted-tool supervision,
not a guarantee against an unobserved hostile fork/setsid/reparent escape.

The first CLI fixture compiled but aborted during upstream console initialization:
the lean VM could not compile `ConsoleBindings.js.inc`. That failed attempt is
retained. The corrected route used `-exported-unit=incant_aot_probe`, the reviewed
C++ JSI owner and a print callback, with no console library. SDK preflight and
all seven fixed fixtures then matched Node. The plain SDK harness owns its arrays
and uses a small stride-3 context; it is not an actual stride-10 ECS loan or full
NativeContext conformance. No Rust exception/longjmp boundary was exercised.

All seven original TypeScript fixtures and both current SDK declarations returned
exit 1 under `-typed -parse-ts -O -emit-c`. Diagnostics include readonly syntax,
remaining TS-to-Flow annotations and mapped types; three cases failed in Hermes's
own TypedLib, and one emitted no specific error despite failing. These are exact
compiler-mode observations, not proof that every source language feature fails.
No source syntax was removed to secure acceptance.

The deadline host armed 10 ms, flushed its setup marker, and called a nonallocating
native loop. It did not return or throw internally. The independent supervisor
terminated the single process with SIGTERM after two seconds; total including
cleanup was 2.067 seconds. The emitted C is a self-targeting `goto` without a poll.
No deadline retry, instrumentation or compiler fork was performed.

Public [timeout APIs](https://github.com/facebook/hermes/blob/1d2fc48c021fbda8eb14478379112ed67bed8f0f/API/jsi/jsi/hermes-interfaces.h#L176)
require externally prepared code to contain async-break checks. Although actual
`shermes -help` advertises `-emit-async-break-check`, its
[context setter is commented out](https://github.com/facebook/hermes/blob/1d2fc48c021fbda8eb14478379112ed67bed8f0f/tools/shermes/shermes.cpp#L721),
and inspected SH emission has no corresponding insertion/helper. HBC supports
entry/backedge checks consumed by the interpreter. No explanation for the
commented setter was found; do not infer a deliberate performance tradeoff.
Uncommenting it alone has no demonstrated SH effect. The flag was not separately
executed in this experiment.

Public JSI can retain a `MutableBuffer` owner and query detached state, but no
public detach mutator was found at this pin. Internal detach and Test262 helpers
exist. Allocation lifetime does not revoke retained ECS aliases when a loan ends.
Expiry, aliases/subarrays, rollback and bounded/recoverable allocation still need
a reviewed bridge and actual conformance evidence.

| Decision 18 requirement | Status after this experiment |
| --- | --- |
| 100% current SDK conformance | Untested as a complete suite; native deadline control fails on the tested route. Loans, rollback, allocation/work limits and current command families remain unqualified. |
| At least 2x matched Mac shipping bytecode performance | Untested; no shipping pair or valid performance comparison. |
| iOS no-JIT build/inspection | Untested; Mac configuration does not establish iOS. |
| Less than 5 MB added mobile binary | Untested; desktop artifacts do not establish mobile delta. |
| Upstream security-fix release line | Unresolved for this static_h pin. |
| Separate complete ScriptTick 2 ms p95 gate | Not established; no matched Mac shipping-profile measurement. |

Verify data without executing archived code:

```sh
python3 docs/spikes/evidence/static-hermes-feasibility/verify.py
```

The compressed data retains finite values, diagnostics, source/host/compiler
bindings, the initial bootstrap failure, supervision limitation and single
deadline outcome. Native tools/libraries, emitted C and binaries remain in the
external hash-bound packet named by the manifest; optional `--external-packet`
only hashes that archive. All native resources were released. Ordinary deletion
removed owned staging after an unnecessary chmod of CMake's signature directory
was denied; both cleanup outcomes are retained. No checkout, global installation
or engine selection was changed.
