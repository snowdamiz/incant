# QuickJS C profile-guided compilation: standalone feasibility

The pinned QuickJS C interpreter successfully trained and consumed an execution
profile on this Mac while preserving the tested behavior. This establishes a
usable compiler path for a later Core Sample experiment. It does not establish
a speedup, the 2 ms script budget, full SDK conformance or engine adoption.
Ordinary builds and the exporter are unchanged.

The private harness compiles the same four QuickJS 0.16.2 translation units used
by the maintained native dependency. Apple Clang and llvm-profdata both report
major version 21. Baseline, instrumented training and profile-use builds use the
same C sources, macros and optimization settings. The C harness itself is not
instrumented. This is a standalone C build, not an assertion that every Cargo
shipping compiler option has been reproduced.

Training uses a separate trusted bytecode workload. Only its owned raw profile
feeds the QuickJS profile merge; held-out execution and compiler canaries have
separate raw outputs. The resulting indexed profile is 408,368 bytes, SHA-256
`d7b2783bb468a225ce86294b1394542999a65031580dd43341e0254e988c6759`.
The held-out baseline, training and profile-use executables each produce the
independently expected result 97,914 and 771 callbacks. Their 317-byte held-out
bytecode is identical. Syntax, range and type errors, detached-buffer behavior,
interruption and memory-limit recovery controls pass in all three executables.
Baseline and profile-use artifacts contain no profile-generation runtime.

The fourth attempt passes all 57 managed commands: 51 expected successes and
six specific expected refusals. Those refusals cover an unknown warning option,
missing and corrupt indexed profiles, stale control-flow data, a new unprofiled
function and a merge containing corrupt raw data. Stale and missing IR profile
diagnostics are explicitly fatal; a warning with exit zero cannot qualify.

Three earlier attempts remain failed:

- The first used an unsupported joined `-j2` argument for llvm-profdata.
- The second expected detailed counters from a covered-functions command that
  actually returned a list of names.
- The third preserved behavior, but the compiler returned success after warning
  about a stale control-flow profile. Its existing warning promotions did not
  cover the emitted backend diagnostic.

The successful successor changes those argument/parser/diagnostic guards without
changing the interpreter, harness workload or acceptance values. Nineteen
focused controls pass. The successful native wrapper exits zero; every managed
child returns and is cleaned up. Root independently rehashes all 114 output
streams and 14 retained artifacts, rechecks the actual held-out reports and
training-only merge input, and confirms all 58 observed process identities absent.
Cumulative time for all four attempts is 79.673 seconds. Retained accounting
peaks are 36,207,880 bytes of work, 3,442,216 bytes of profiles and 17,440,508 bytes
of evidence, below the original bounds. The successful run's scratch is removed;
original failures, logs, profile identities and binaries remain retained.

The [portable evidence](evidence/quickjs-c-pgo-feasibility/manifest.json)
contains 366 inert text records, including every attempt's command output,
the focused controls and independent review. Its archive is 497,897 bytes;
native binaries and profiles remain external with exact hashes. Root separately
replays all records, compares the 33 source inputs to producer
`b3da3b6d9bbb6a8870480ae54d97a2884d20edde`, rehashes the external artifacts,
and passes eight saved-data refusal controls. Replay does not execute archived
code or a compiler:

```sh
python3 -B docs/spikes/evidence/quickjs-c-pgo-feasibility/verify.py
python3 -B docs/spikes/evidence/quickjs-c-pgo-feasibility/test_verify.py
python3 -B docs/spikes/evidence/quickjs-c-pgo-feasibility/verify.py --originals --artifacts --git-source b3da3b6d9bbb6a8870480ae54d97a2884d20edde --repo .
```

The next experiment must train on representative Core Sample execution and
compare matched shipping builds with exact gameplay and audio parity. It must
also preserve profile identity across Cargo's differing generated-source paths,
bind profile/tool identity to script and checkpoint compatibility, and keep
training instrumentation out of the measured baseline and candidate. No such
engine experiment or performance result is claimed here.
