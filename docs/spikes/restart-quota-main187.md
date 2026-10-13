# Restart quota regression isolation

Checkpoint17 at `7a4ad870` failed the same restart quota assertion on Linux and
Windows baseline conformance. Both logs report 127 passing sibling library
cases and one failure. The assertion did not print the returned error, so the
exact hosted error variant is unknown.

This test makes 65,536 restart calls plus one caught overflow using the default
50ms thread-CPU deadline. Adjacent full-buffer structural quota tests use an
explicit five-second test budget. The repair adopts that same test-only budget
and retains exact `InputLimit` failure. It adds the exact maximum positive
control, mixed velocity/restart quota sharing, numeric/queued-write rollback
and typed terminal-session rejection. Production defaults and limit handling
are byte-identical to main187.

A local diagnostic version at 50ms passed once and 40 additional parallel
processes. A deliberate one-millisecond test budget produced the exact
`ExecutionDeadline` error and failed the original quota assertion (exit101).
This control demonstrates budget competition but does not establish which
error occurred on either hosted runner. All raw results are retained.

The repaired four restart tests pass; the full UI/audio script library passes
127 cases, with no failures or ignored tests. All-target Clippy, formatting and
diff checks pass. Independent source/evidence review is accepted. The exact-head fast check and
next corrected full checkpoint remain required; the running checkpoint17 is unchanged.

[Lossless evidence and source identities](evidence/restart-quota-main187.json).
