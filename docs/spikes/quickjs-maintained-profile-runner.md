# Maintained heap observation runner — source controls

The explicit measurement seam and bounded sequence runner implement the
[approved shipping protocol](quickjs-maintained-shipping-protocol.md). The
adjacent plan remains unchanged at SHA-256
`8c14c925f8044d02681b06cbda1857937201ba09a1e722d218edb4425828caa9`.
This work does not build or run a native player, measure performance, or approve
a functional, performance or phase gate.

`tools/mac_performance.py --heap-mode maintained|detailed` records the requested
mode and actual argv, and accepts only raw profile v3 with that actual heap mode.
Detailed adds `--profile-heap-detail`; maintained uses `--profile`. Omitting the
argument preserves default argv and historical v2 ingestion. Actual raw report
hashes, instrumentation identity, complete histogram derivation and the existing
2 ms/5% policies remain bound together. An explicit mode never admits a v2 report.

The sequence entry point is `tools/probes/quickjs-maintained-profile/measure.py`.
It takes one existing source-free shipping package and a separately qualified
fresh unprofiled report/PCM control. It checks the approved plan and input receipt
hashes, freezes input bytes into its new output directory, and uses the helper
for exactly D0, M0, M1, D1, D2, M2. The same executable/package identity, machine,
input, ordered script sources, logical report after removing only `profile`, and
exact PCM must survive every arm. It refuses incomplete heap/source observations.

Before the first invocation, `sequence.json` fixes maintained's anchor to M0 and
detailed's anchor to D0. A failing M0 2 ms result cannot select another baseline.
M1/M0, M2/M0, D1/D0 and D2/D0 use the unchanged comparator as within-mode stability
context. Cross-mode output is only the signed difference between independent
quantile intervals. Callback totals divided by calls remain rational means; no
subtraction or guest-loop improvement claim is produced.

Each child has a 45 second execution timeout. The sequence has a 300 second
execution deadline. TERM, INT and ALRM latch the first cancellation; explicit
checkpoints raise before a new arm or inside `native_process.run`'s owned wait.
Repeated signals cannot interrupt process creation, kill/reap or reader cleanup.
The default helper callers retain their existing behavior. Mandatory cleanup and
an already-blocked filesystem operation or existing bounded host probe may finish
after the deadline; 300 seconds is not a hard wall-clock termination guarantee.
Failures stop without retry and
retain raw output, the command receipt and the incomplete attempt.

After independently reviewed current-main source/export closure, one fresh cook
and export, full unprofiled/detailed/maintained functional parity against the
historical unprofiled anchor, the Core-win/stress oracle, and actual all-lane quiet
acknowledgments, the command is:

```sh
python3 -B tools/probes/quickjs-maintained-profile/measure.py \
  "$candidate_package" "$new_sequence_output" \
  --functional-report "$fresh_unprofiled_report" \
  --functional-pcm "$fresh_unprofiled_pcm"
```

Those prerequisites require separate evidence; the runner does not infer them
from a matching project hash or a supplied control file. No current package or
timing output is selected by this source-only change.

Light Python controls exercise actual argv and raw identity ingestion, default
legacy compatibility, wrong/legacy explicit-mode refusal, the fixed order and
preselected failing M0, same-mode anchors, logical/PCM/source/heap refusal, retained
failure state, and protocol byte binding. Harmless Python process trees exercise
child timeout, total deadline, TERM during spawn and wait, repeated signals during
cleanup, and deadline arrival during child-timeout cleanup. Synthetic profile
fixtures and Python lifecycle controls do not count as player or timing evidence.

The following local controls exited 0: `test_mac_performance.py` (21 tests),
`test_quickjs_profile_sequence.py` (13 tests), and `test_native_process.py`
(48 tests, including four Windows-only skips; 44 passed). Each used
`python3 -B -m unittest discover -s tools/tests -p <file> -v`. Exact command output
and source hashes are retained in the external `quickjs-maintained-runner-001`
evidence packet. Windows cancellation execution was not performed.
