# Preserve operation failures through cleanup

A bounded shipping build hit its 8 GiB process-memory ceiling during final linking.
The process owner stopped its group, but its immediate absence check observed a
briefly surviving descendant. The secondary cleanup exception hid the original
resource refusal. The failed build remains failed; it produced no qualified package.

`native_process` now carries both original exceptions when operation and cleanup
fail together. A cleanup failure after a successful operation still fails. The
probe owner observes asynchronous group retirement for up to five seconds before
refusing; the final process snapshot has its own five-second timeout. It never
signals a reaped or reused identity. Process termination and Windows Job Object
rules are unchanged.

At source `8a705821`, the focused process suite enumerated 56 tests: 52 executed
successfully and four Windows-only controls were skipped on this Mac. Four owner
controls also passed, including a real parent/grandchild cancellation and pure
transient, persistent/reused-identity and observer-failure cases. These check
resource refusal, timeout and caller interruption alongside cleanup failures.
Independent source review found no blocking issue.

All ten local development checks passed in 77.529 seconds, including 1,040 UI
tests. The actual outer tool returned zero and a fresh process table confirmed the
wrapper and all ten owned PID/groups absent. No computer use, screen capture,
native window or physical audio was used. Windows execution remains a hosted
checkpoint responsibility; this change claims no runtime performance improvement.

The [portable manifest](evidence/native-process-cleanup.json) identifies the
bounded [raw evidence archive](evidence/native-process-cleanup.json.gz), containing
commands, complete logs, review, source identities and process closeout.

## Separate fixture startup from execution deadlines

The registered-source integration fast run (actual `77244`) failed four controls
and errored in one among 738 Python tests. The harmless Python fixtures had not
reached readiness before their short execution deadlines; the required output
prefixes or active descendant were absent. That failed run remains retained.

Test-only source `0d185664` separates bounded startup from the execution interval.
Worker handshakes and EOF get five seconds; hung requests retain 0.15 seconds and
the intentional startup hang retains 0.2 seconds. The locale and sequence fixtures
own the actual process tree during a five-second readiness allowance, then arm
their original short execution deadlines. Sequence tests still use real SIGALRM.
The RPC fixture's test clock starts only after the owner consumes both actual
output prefixes, with a five-second startup refusal. It preserves exact captured
stdout/stderr assertions. Production deadlines and cleanup code are unchanged.

The four focused modules passed once: 6 worker, 12 locale, 15 editor-protocol and
15 sequence tests (48 total), including new missing-readiness and alarm-arming
error controls that prove descendant cleanup. Actual `39086` returned zero in
12.907 seconds; all four managed calls confirmed cleanup, followed by fresh
absence of nine observed PID identities and four groups. The source stayed clean.
This is focused Mac Python-process evidence; the integrated full suite remains a
separate result, and no engine, Cargo target, device, window or capture was used.

The [fixture manifest](evidence/lifecycle-test-startup.json) binds the original
failure, exact source patches, complete focused logs and terminal closeout in a
small [raw archive](evidence/lifecycle-test-startup.json.gz). Data-only roundtrip,
all external-original hashes and the 48 named successful results were checked.
