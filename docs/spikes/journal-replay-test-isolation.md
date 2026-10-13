# Journal authority replay process isolation

Checkpoint23's macOS `checks` job failed the no-op budget replay assertion on
candidate `3381ccef` (run38394441579/job115286144925). The original assertion did
not print its error. The full original log and annotations remain retained.
No hosted diagnosis or repaired hosted execution is inferred from local success.

The production replay path validates the transaction base and actor, then rejects
non-user budget commands before committing the projection. A controlled local
fork held a child before exec while it inherited the journal descriptor. Dropping
the parent bus did not release the shared file lock: reopening returned `Io`
wrapping `std::fs::TryLockError::WouldBlock`. After the child exited, the identical
tampered journal returned `AgentBudgetUserOnly`. This demonstrates a mechanism
that can interrupt the assertion before authorization; it remains an inferred
cause of the hosted failure, whose actual error was not recorded.

The one-file test change executes each existing authority replay case in an
exact-filtered test child before opening its journal. Sibling process-recovery
tests cannot fork inside that child's ownership interval. The parent requires
successful exit and exactly one executed test, so an empty filter cannot pass.
All actor/command/projection checks remain, errors now print explicitly, and the
no-op case also checks unchanged journal bytes. Production replay, authorization,
exclusive locking and error mapping are unchanged. There is no lock retry or
explicit production unlock, and no unsafe diagnostic code is retained in source.

On actual main224, all 54 command-crate tests in 18 groups and strict all-target
Clippy pass. Ten additional six-case concurrent library runs pass; these overlap
the main suite. All 15 qualification commands exit zero, with a copied executable
and 3,143 unchanged source inputs. The [manifest](evidence/journal-replay-main224.json)
and [read-only verifier](evidence/journal-replay-main224/verify.py) bind the packet.

The diagnostic history is preserved: 63 initial complete library repetitions and
two 1,000-replay diagnostics passed without naturally reproducing the failure.
The initial repetition controller was stopped only after its current child exited;
one final uncollected output is not counted. Controlled diagnostics004 and005
correctly exposed the early lock error but failed their own assumptions about the
outer error kind and Display text. Diagnostic006 uses the actual inner typed
`TryLockError` and passes both before/after assertions. Historical diagnostic
executables were not retained; their source/log evidence is distinct from the
copied final executable. Successor hosted execution and release remain pending.
