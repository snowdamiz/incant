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
