# Non-reaping child observation on older macOS Python

Frozen checkpoint2 main88 `c3ec3e8` failed before Rust compilation because the
Python3.12 macOS interpreter lacks `os.waitid`. Python documents macOS support
[beginning with3.13](https://docs.python.org/3/library/os.html#os.waitid). The
process helper correctly refused to weaken its process-group lifetime rule, but
the local Python3.14 validation did not expose this supported-workflow mismatch.

The compatibility path uses Darwin's stdlib `kqueue` with
[`EVFILT_PROC` / `NOTE_EXIT`](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/kevent.2.html).
It observes the direct child without consuming its wait status. The unreaped
leader continues to reserve its PID and process-group identity until group
termination and final `wait()`. A child that exits before registration remains
waitable; a registration refusal closes the observer and terminates/reaps the
child. Completed observation is latched, and cleanup closes the queue. The
existing `waitid` path remains selected where available; the Windows Job Object
implementation and execution/argument boundaries are unchanged.

All35 applicable process tests pass on actual system Python3.9.6 with no
`os.waitid`, and on Python3.14.8 exercising both native and forced-fallback paths
(four Windows-only tests skipped in each39-case run). Real process tests cover
pipe-holding descendants, no-pipe descendants, success, nonzero, timeout, invalid
output, caller interruption and setup failures. Additional cases establish that
exit17 is still recoverable only by final wait, and an already-zombie child
retains exit19 across late observer registration. The complete tooling suite
passes199 applicable tests with four Windows-only skips.

Root independently reviewed the final implementation and regressions. The
original hosted failure remains preserved; actual hosted Python3.12 confirmation
is pending the next corrected checkpoint. No failed release is promoted.
[Exact source and log identities](evidence/darwin-process-observation.json).
