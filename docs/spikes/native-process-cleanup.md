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
