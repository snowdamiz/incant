# Native TypeScript cook deadline observation

Checkpoint32's Windows desktop job `115682817775` failed the unchanged authored
fallback test at its eight-second Node cook deadline. The immediately preceding
native-host tests had cooked authored sources successfully. The previous
checkpoint's Windows fallback harness passed with the same compiler/test source.
The failed job had finished Rust compilation before this harness ran. These facts
do not identify the slow compiler phase or establish a host-contention cause.
The complete failure and previous passing log remain in the portable packet.

The authoring compiler now writes eight fixed cumulative milestones to a private
sidecar: driver entry, compiler dependencies ready, input catalog ready, source
snapshot ready, type checking done, transformation done, artifact ready, and
publication done. Driver entry precedes dynamic loading of the pinned TypeScript
and SWC dependencies. Project source, paths, diagnostics and inherited environment
values are excluded from these observations. The environment selecting the private
sidecar is constructed by the trusted Rust host after clearing its environment;
project data cannot select flags, executables, a deadline or the observation path.

The parent permits at most 4 KiB and eight rows, validates names/order/integer
microseconds, discards arbitrary content, and retains only complete rows after a
partial final write. It kills/reaps the child before reading the sidecar. An empty
sidecar is explicit evidence when no first milestone was reached. Timeout still
returns the existing error and the production deadline remains eight seconds.
Successful observations are printed only when the trusted host sets
`INCANT_NATIVE_COOK_DIAGNOSTIC=1`; deadline observations are always retained.
Artifact validation, source-recovery checks, strict type checking, confinement,
publication and fallback assertions are unchanged. The few synchronous sidecar
writes are diagnostic overhead, not a performance optimization.

Child microseconds start at recorder initialization after builtin driver imports;
parent microseconds start when process spawning returns and include cleanup.
Their different epochs must not be subtracted to infer Node startup duration.
`exited_success` describes the Node child, before Rust validates/compiles its
artifact; it does not claim the whole game or authoring operation succeeded.

Frozen source `9d506ec9` passed ten local commands on this Mac: all 23 Node
compiler controls, three Rust observation controls, two real TypeScript
integration cases, the one exact authored-fallback case, strict all-target cook
Clippy, formatting, six release-policy controls, actionlint, conventions and diff
validation. The Node control compares bundle/map/manifest bytes with observation
on/off. A real sleeping child exercises timeout before its first phase, the same
kill/wait method, an empty observation and absent output. Parser negatives cover
unknown fields/names, ordering, invalid times, excess bytes and partial writes.
All 3,909 recorded build inputs remained exact, and three executables were copied
immediately after their commands. The fallback cook emitted all eight phases at
0.640 seconds parent elapsed, then the deleted-source control correctly refused.
This single Mac observation does not repair or reproduce the Windows failure.

Root reviewed the exact source patch and protocol before qualification; its
received review message is retained verbatim. The [portable verifier](evidence/typescript-cook-phases/verify.py)
checks 50 lossless records, the committed input identities, actual command/test
counts, three copied binary identities when requested, and the original Windows
failure. Run it with `--current --external` only when this source and the original
local artifacts are available; its default mode uses committed inputs and the
portable archive without running compilers or tests.

After the reviewed PR's exact required fast check and merge, dispatch
`TypeScript cook deadline diagnostic` once against that exact merge SHA. It has
no inputs, no deadline override, no retry and no cache writer. It runs the same
workspace feature selection with only the original fallback integration target
and exact test, then retains the actual exit/log and fixed-phase observations.
The 35-minute workflow ceiling includes compilation; the cook deadline stays
eight seconds. A missing or incomplete receipt cannot turn a failure into success.
A passing invocation is one diagnostic observation, not a release substitute or
proof that the historical timeout is fixed. Windows execution remains pending.
Checkpoint32 stays frozen until its original workflows finish; release remains
unchanged and all 23 required checks still require actual SUCCESS for promotion.


## Actual Windows observation on main270

The single authorized workflow run [38516305407](https://github.com/snowdamiz/incant/actions/runs/38516305407)
completed successfully on exact merge `ed3999b02d2a747dee93d278a2c1c50d43f275b7`.
The selected test passed once with zero failures/ignored cases. The successful
Node cook's parent observation was 2.442570 seconds; its eight fixed child phases
ended at 1.982913 seconds in the child's separate epoch. Compiler-ready was
1.035031 seconds and typecheck-done 1.941074 seconds. The test's expected
deleted-source refusal also executed (0.783991 parent seconds, three phases).
Do not subtract the different parent/child epochs as a startup measurement.

The workflow built in 19m13s and the job ran 05:22:47–05:43:40 UTC on 2026-10-14.
These are this diagnostic's durations, not a performance comparison. The first
dispatch API request rejected a raw commit SHA as an unsupported ref before any
run existed; the corrected branch addressing created exactly one run and its
actual source was checked. There was no test or workflow retry.

[Portable raw API, job log, artifact and verifier](evidence/typescript-cook-windows/manifest.json)
retain the observation. Replay with `python3 docs/spikes/evidence/typescript-cook-windows/verify.py`
(and optionally `--originals`). Checkpoint32's original eight-second timeout remains
failed and unexplained. This passing observation is neither a timeout repair nor
a replacement for any of the 23 release requirements.
