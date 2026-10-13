# macOS conformance timeout correction

Checkpoint27's macOS Wasm conformance job was cancelled at its configured
40-minute maximum. The combined static/development conformance step completed
successfully after 32m21s; the fresh compiler-free shipping build had run for
6m39s when the job expired. The composed shipping proof and later dependency
checks did not run. [Exact job and step evidence](evidence/macos-conformance-timeout-checkpoint27.json)
retains that failure.

The macOS job now has a 90-minute total budget for its cold development graph and
two separate fresh shipping proofs. Linux keeps its existing 40-minute budget.
This prevents the known premature cancellation; it is not a measured speedup or
a claim that the remaining proofs pass. Test commands, release contexts, source
and target isolation, artifacts and admission rules are unchanged. Development
PRs continue to use the fast main check independently of this full checkpoint.

Local verification parsed both YAML files and confirmed that the timeout is the
only semantic change, and all nine existing workflow/profile guard tests passed.
The next exact-head hosted checkpoint must still complete every required check
successfully before promotion. No old cancelled or skipped job counts as a pass.

[Independent source and evidence review](evidence/macos-conformance-timeout-review.json)
accepted the exact main242 composition and independently reran the nine guards.
Hosted success remains pending a later exact-head checkpoint.
