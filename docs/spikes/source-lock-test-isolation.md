# Source-store lock test isolation

Checkpoint29's Linux desktop job failed the immediate drop/reopen assertion in
`root_lock_is_exclusive_across_different_journals` on candidate `74e77456`
(run38469082532/job115543193346). Nine other source-store tests passed, including
seven real crash-recovery subprocesses. The original assertion did not print its
underlying error. Its complete raw log and job metadata are retained; the hosted
cause remains inferred and successor hosted execution is still required.

A controlled local diagnostic held a forked child before exec while it inherited
a real SourceStore descriptor. Dropping the parent owner left the lock held;
reopening returned `SourceError::Io` containing the typed
`std::fs::TryLockError::WouldBlock`. Closing the child's release pipe and reaping
it made the identical reopen succeed. Fifty natural concurrent suite repetitions
with diagnostic error printing passed without reproducing the failure. The
controlled mechanism is evidence of a possible race, not a recorded hosted error.

The one-file test repair runs the existing exclusivity case in its own exact
filtered subprocess before it opens the store. Sibling recovery tests cannot fork
inside that child's lock lifetime. The parent requires successful exit and one
passed, zero failed, zero ignored tests, preventing a missing filter from passing.
The original held-owner refusal and drop/reopen assertions remain; reopen errors
now print explicitly. Production source, locking and recovery are unchanged.
There is no lock retry, explicit production unlock or shipped unsafe diagnostic.

On main250, all ten source-store cases and strict all-target Clippy pass. Ten
additional concurrent ten-case repetitions pass. All fifteen qualification
commands exited zero, with both diagnostic and repaired executable copies bound
by hashes. The source freeze verifies 4,136 implementation/build inputs and only
one changed test file. The diagnostic and its eleven-case run are separate from
the repaired ten-case suite, and all repetition counts overlap those cases.

The [portable manifest](evidence/source-lock-main250/manifest.json) and
[assertion-only verifier](evidence/source-lock-main250/verify.py) preserve 151 raw
records and 79 lossless payloads, including the original hosted failure. Optional
`--local-originals --local-binaries` verifies local retained copies too.
Independent source and final evidence reviews are accepted; publication awaits
the required exact-head fast check. No release or repaired hosted-success claim is made.
