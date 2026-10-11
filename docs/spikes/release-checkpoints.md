# Local verification for release checkpoint CI

The director authorized the cadence change on 2026-10-11. The previous PR82
workflow reached macOS completion after 42m41s; execution took 37m21s. This is the
observed development wait being removed, not a measured speedup of compilation.
[ADR 0014](../adr/0014-ci.md) records the branch and promotion contract.

On macOS, the policy change passed 137 Python tool tests, 17 Node script tests,
536 UI tests, strict SDK compilation, UI production build, generated bridge/SDK
checks, cargo fmt, generated convention checks, actionlint and whitespace checks.
Tests execute the forced full-scope CLI against a real documentation-only Git
history, exercise actual cache-producer shell decisions (including case-sensitive
ref refusals), and check workflow event boundaries and the complete 22-context
release contract. The full engine build/test/probe bodies remain present; changing
triggers does not count as rerunning their correctness or performance proofs.

All feasible scoped engine tests and review are still required before development
merges. The new hosted fast job's duration, branch protections, initial release
baseline and first candidate results must be recorded after deployment. This
local verification does not claim hosted success, physical-device performance,
phase completion, or a signed/published distribution release.

## Integrated PR83 transition

The source transition preserves the previously reviewed SDK, color, runtime, editor
and shipping-proof bodies byte-for-byte. On the old head `b89f518`, twenty hosted
checks succeeded, but general macOS job 114505127324 failed before player execution:
its Rust SDK guest needed `wasm32-unknown-unknown`, which that job had not installed.
The correction installs the exact Rust 1.99.0 target before shipping preparation.
An actual trusted guest then compiles locally (638,223-byte Wasm), and a regression
checks prerequisite ordering. The failure log and both completed raw build streams
are retained; no final shared-session ledger existed after the preparation failure.

The integrated transition passes 138 Python, 12 Node and 536 UI tests, strict
SDK compilation, UI production build, generated bridge/SDK/conventions, Rust
formatting and workflow lint. The policy's earlier 17-Node result was on a private
authored-script union; this PR preserves its separate 12-test source scope.
The root created the active `incant-release-checkpoints` heartbeat at the agreed
three-hour cadence. Repository settings, exact final fast-check success and the
first full candidate result are deployment evidence still to be recorded; neither
the historical failure nor local verification is presented as full hosted success.
[Machine-readable evidence](evidence/release-checkpoint-transition.json).
