# Tilemap journal assertions on Windows

Checkpoint56's Windows credential job failed three new agent tilemap tests at
`fs::read(game.journal.jsonl)`. The command bus still owned the journal's exclusive
OS lock; Windows returned error33 before the relevant refusal/no-op assertion.
The failed [hosted job](https://github.com/snowdamiz/incant/actions/runs/39103549011/job/117604553477)
is retained. No credential operation or production journal mutation failed in
that trace.

The test-only fixture now releases its command bus before reading the exact
journal bytes, then reopens the same project and journal. It verifies identical
project, revision, history and redo state after recovery. Every existing no-op,
malformed-request, denied/cancelled-approval and byte-equality assertion remains.
Production locking is unchanged; no platform skip, retry or length-only assertion
is added.

Source `7f5130ded2a8394483d390c11704b612d0eb5d12` passes all ten native agent tilemap
tests and strict Clippy for that test target on this Mac, using pinned Rust1.99,
`ci` profile, offline/locked dependencies and two build/test workers. All workspace
crate artifacts rebuilt. Independent source review reports no blocking finding.
The ten development checks also pass, including 1,040 UI tests.

The first native wrapper exited1 **after** Cargo/test exit0 and ten successful
tests: its post-test audit incorrectly required unchanged vendored library and
build-script artifacts to rebuild too. The failed wrapper remains retained.
A separate data-only audit verifies every workspace crate was rebuilt, all2,560
vendored source files match the prior qualified source inventory, the exact test
binary, and the terminal process absence. It does not rerun or relabel that
wrapper. Separate strict lint and development-check runs actually exit0.
The test/build episode took60.681 seconds, lint6.466 seconds and the full fast
checks77.396 seconds. Every recorded owned group and wrapper is absent afterward.

[Portable evidence](evidence/tilemap-journal-portability.json) includes the original
Windows log, native logs, failed-wrapper result, independent data correction,
source review, lint and development results and terminal closure. Actual Windows
confirmation remains required in the next comprehensive checkpoint; local Mac
success does not establish it. Checkpoint56 stays failed and release is held.
