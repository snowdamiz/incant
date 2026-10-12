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
merges. The first fast job and deployed branch rules are recorded below. The historical
local policy verification is separate from the first successful full checkpoint
promotion, now recorded below; neither establishes physical-device performance,
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
original transition receipt records what was pending at that point; checkpoint 5
closes full promotion below. Neither the historical failure nor local verification
is presented as full hosted success.
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
heartbeat is `incant-release-checkpoints`. That baseline was retained through four failed checkpoints; checkpoint 5 was
promoted only after the complete frozen suite succeeded.
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
to the next snapshot. Checkpoint 4 confirmed that Windows regression, but found isolated physics-lock
and PE symbol-audit failures. Each failed checkpoint remained unpromoted.

## Checkpoint 5 — first verified promotion

[PR85](https://github.com/snowdamiz/incant/pull/85) merged into `release` at
**2026-10-11 20:33:01 UTC** after all **23 required contexts** succeeded on the
exact candidate push across eight comprehensive workflows. The candidate
`04f8d9b18a4b29893c63a346841fffd41ce437c7` equals selected main97. Release merge
`c603ca07bbe1e6e1ad46b2fd3aa2f70aaec07f50` has the bootstrap release and that
candidate as parents; its tree `438e65c9165372e45681d40c910d6bb05b056832` exactly
matches the tested tree. Strict required checks, GitHub Actions app identity,
administrator enforcement and the `Built-by: astra` merge trailer were verified.
[Complete check identities, timestamps, audits and failure history](evidence/release-checkpoint-5.json).

| Required job | Execution | Wait from workflow creation |
| --- | --- | --- |
| Windows desktop | 54m06s | 3s |
| macOS general | 44m48s | 4m13s |
| Linux desktop | 16m42s | 3s |

The Windows critical path includes 10m52s workspace tests/build, 5m08s optional
native-window tests/build, 12m09s authored export, 7m28s standalone shipping proof
and 5m27s cache cleanup/packaging/upload. These changed-graph observations do not
establish a controlled cache or profile speedup. Main development checks remain
a separate roughly two-minute gate; full checkpoint cost has not disappeared.

All three native physics jobs and their strict comparison now pass with the
repaired isolated lock. Actual Windows authored export passed three source/build-
deleted 20-tick runs. Its 9,251,328-byte player, matching PE/PDB identity, fresh
link-map bytecode/parser audit, shipping flags, cooked data and PCM hashes were
independently rechecked from the hosted artifact. macOS passed the four-player
shared fresh session in 1,401.486s: 282 unique fresh target units and 371 reuse
events within that session. All eight raw build-log hashes and 3,417 frozen source
hashes match; source/build deletion preceded every player launch.

The verified Windows cache replacement uploaded 2,011,726,110 bytes in full.
Only obsolete main cache `8800194134` was retired after confirming replacement
`8806499881`, exact family/version and successful producer completion. All other
entries were preserved: 12 entries total 8,701,264,445 bytes afterward. Future
consumption of that exact Windows cache is still unmeasured. No failed earlier
batch was promoted, and no test context, shipping assertion or gate was removed.

## Checkpoint 7 — failed, repaired on main

Frozen main113 candidate `7b376924` completed all eight workflows with 22 of 23
required contexts successful. Windows desktop passed in 33m06s and Windows Wasm
conformance in 34m12s, confirming the earlier journal fixture repair. General
macOS failed reopening a ledger with Busy. PR115 repairs explicit acquired-lock
release, preserving exclusive ownership and conservative usage accounting; 64
local agent tests and Clippy pass. The precise hosted descriptor interleaving
was not observed. This failed candidate was never promoted, and its historical
failure remains recorded in the [terminal receipt](evidence/release-checkpoint-7.json).

PR116 subsequently merged reviewed native animation after its 114-second fast
check and scoped native acceptance. The next deliberately frozen repair-validation
checkpoint will select actual main after Console integration; later development
merges will not move that candidate. Full hosted confirmation of the lock repair
remains pending, without waiving any of the 23 contexts.

## Checkpoint 14 — main160 promoted

The repaired frozen candidate passed all 23 required contexts and eight exact-head
push workflows. PR133 promoted release `2444d81f` at 2026-10-12 20:49:50 UTC, with
exact selected/tested tree `c0137be6`. Hosted logs confirm the AppKit lifecycle
regression and numerical UI checks pass on identified adapters. Earlier failed
batches remain historical, and main161+ integrations are excluded.
[Full receipt, durations and limitations](release-checkpoint-14.md).
