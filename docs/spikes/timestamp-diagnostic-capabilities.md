# Timestamp diagnostic capability policy

Checkpoint22 remains frozen at `a894b7ca` (selected main216). Its macOS `checks`
job 115255920239 passed the production timestamp test's typed Unsupported and
completed ordinary-frame branch on Apple Paravirtual Metal, then failed the
separate direct backend diagnostic's unconditional timestamp-feature assertion.
The original complete log is retained in this packet. Other checkpoint jobs may
continue; no failed result is replaced or treated as a passed release gate.

The direct diagnostic now validates `INCANT_EXPECT_GPU_BACKEND` before considering
optional features. Basic timestamp queries run both descriptor cases; inside-
encoder support additionally runs the two encoder cases. Missing basic queries
still execute both bounded ordinary integer-compute/readback cases, and report
Unsupported with null timing fields. No query or sample is fabricated. Actual
integer values, bounded waits, and original same/later-submission coverage remain.
The original diagnostic still reports intervals rather than requiring positivity;
the separate production timing test keeps its positive supported-path assertions.

Two CPU tests cover the feature matrix and mandatory backend selection. Exact
main218 source passes the focused test build (2 passed, 1 GPU test ignored), strict
Clippy, formatting, conventions and diff checks. The isolated warm target was
cloned read-only and workspace/patched products cleaned before compiling. No GPU
was launched for this repair; CPU branching is not actual hosted execution. A
successor checkpoint must exercise the updated diagnostic on its real adapter.

A preliminary direct cargo fmt invocation could not find cargo-fmt on PATH; the
normal tools/cargo command formatted the reviewed source. The packaging helper's
initial expected command count was corrected from eight to the actual seven
(metadata, scoped cleanup, tests, Clippy, fmt, conventions, diff); no check result
or source changed.

[Manifest](evidence/timestamp-diagnostic-capabilities.json),
[read-only verifier](evidence/timestamp-diagnostic-capabilities/verify.py).
Final independent evidence review is accepted; required fast checks precede merge.
