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
merges. The first fast job and deployed branch rules are recorded below; the first successful full
candidate promotion remains pending. This
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
three-hour cadence. Repository settings and exact final fast-check success are recorded below. The
first successful full candidate promotion remains pending; neither
the historical failure nor local verification is presented as full hosted success.
[Machine-readable evidence](evidence/release-checkpoint-transition.json).

## Deployed development gate

[PR83](https://github.com/snowdamiz/incant/pull/83) merged at actual main
`290b69078922db4369bfe2ff8485f82013e0fb63` after exact-head `development-checks`
succeeded. Its queue was **2 seconds**, execution **119 seconds**. This is a
deliberately narrower per-merge gate, not faster full Rust compilation. The new
restore-only npm action reused the existing exact Linux cache key successfully
and registered no PR save hook.

`release` was bootstrapped at the fully checked main82 integration
`cde3d7279ffe09204f853f3ce342175a14035b1e`. Initial API readback confirmed main required
only the GitHub Actions `development-checks` context, with strict up-to-date off;
release required all 22 listed GitHub Actions contexts with strict up-to-date on.
Both require PRs, enforce the rules for administrators, and prohibit force pushes
and deletion. No extra approval count is imposed. The active three-hour thread
heartbeat is `incant-release-checkpoints`. No candidate has yet been promoted;
release remains on its known baseline until the complete frozen suite succeeds.
[Deployment and first-run identities](evidence/release-checkpoint-deployment.json).

## Recent fast main checks

PR89–93 each passed the exact required development check before verified merge:
execution was 126, 133, 109, 129 and 128 seconds, with queues of 2, 2, 1, 2 and 2 seconds.
[Head/job identities and timestamps](evidence/fast-main-checks-2026-10-11.json)
record the observed runs. These jobs retain the fast gate's original coverage;
full native/platform checks remain on frozen release candidates.

The reviewed local-service workflow increased release protection from the initial
22 contexts to 23 for checkpoint 2. Checkpoints 1 and 2 were not promoted after
recorded platform failures. Checkpoint 3 confirmed the Linux private-service and
macOS process-tooling repairs, then exposed a Windows restricted-TypeScript-host
lookup failure; PR91 fixes that on main, with real Windows confirmation assigned
to the next snapshot. Release stays on its fully checked bootstrap until every
required context succeeds.
