# Local service foundation

This scoped PLAN revision 8 implementation starts actual Postgres 17.11,
SeaweedFS 4.48, Zitadel 4.19.4 and Mailpit 1.31.5 containers behind a fixed
HAProxy 3.2.25 TCP gateway. All five image indexes are digest-pinned. No hosted
account or supplied credential is required. Generated local bootstrap values
stay outside projects in private per-user storage. The engine/editor/runtime
source and dependency graph are unchanged.

The [usage and explicit boundaries](../../services/local/README.md) describe
one-command startup and the fresh-instance proof. The exact source hashes,
image platforms and sanitized result are in the
[evidence ledger](evidence/local-service-foundation-2026-10-11.json).

On the recorded macOS ARM64 Docker Desktop host, the final frozen proof passed
14 actual protocol checks before and after container recreation, plus four
persistence checks: private settings, issuer, SQL row and exact S3 bytes. Its
37.94 seconds exclude the earlier image downloads and are a local observation,
not a hosted CI or capacity benchmark. All 100 Python tool tests passed, including
14 state/config refusal tests. Actionlint and generated conventions passed.
All generated proof containers, volumes and private values were removed; no
unrelated container or running user application was changed.

A real startup exposed Docker 29's internal-only port-publishing behavior:
requested mappings were retained in HostConfig but absent from NetworkSettings.
The chosen gateway keeps backend egress isolation and binds only loopback host
ports. [Docker's bridge documentation](https://docs.docker.com/engine/network/drivers/bridge/)
and the [upstream behavior report](https://github.com/moby/moby/discussions/53256)
explain this boundary. The helper verifies both the Compose model and actual
running networks/port mappings. HAProxy health requires every backend, Seaweed
health requires its data node, and Zitadel's readiness configuration matches
its explicit local HTTP mode. Telemetry and service pings are disabled.

The local OIDC proof uses a disposable public native client with S256 PKCE and
a short-lived local login-service identity through the pinned issuer's session
API. It proves real issuance/refusal and provider-validated userinfo; it does not
claim a complete editor/browser login or production token verifier. Upstream
configuration and API definitions are pinned to
[Zitadel v4.19.4](https://github.com/zitadel/zitadel/tree/v4.19.4).
The [upstream license change](https://zitadel.com/blog/apache-to-agpl) matters:
this version is AGPL-3.0-only, not the old Apache edition. Retained application
licenses and a service-specific allowlist cover these unmodified local processes;
a full transitive OS-image SBOM, security and production distribution review
remains open.

The dedicated Linux workflow runs only for a frozen release-candidate push or
explicit dispatch. It adds no PR/main container matrix and does not alter an
existing engine check. Its first hosted execution remains pending publication. No local
protocol test substitutes for real WorkOS/AWS/store/device/region gates. Accounts
service implementation, WAL-to-S3/PITR, the other PLAN §2.7 services and production
operations remain separate work.

## Main84 integration and checkpoint readiness

The reviewed foundation was preserved while integrating frozen authored PR84
`337c93b1fe5723ddcd9f8036ce60da775679e79a` and then actual main84
`b84a9bf318c9fa6005d7a5d3524941da2eb4546e` (tree
`601bd42377ecf81a2bb3fbd102b2e249c51479bf`). Only status and license notices
conflicted; both scoped records were retained. The service delta does not change
engine Rust, simulation, the player dependency graph, schemas or UI appearance.

Two substantive corrections were made during revalidation. A nonprivate POSIX
state-home directory now refuses both initial creation and reuse without changing
existing state. SMTP proof now compares the complete message body after CRLF
normalization instead of accepting a matching substring, and deletes only its
own message even when that assertion fails. Behavioral regressions cover both.
The README now accurately states that ambient `INCANT_LOCAL_*` variables are
stripped from child environments, rather than claiming their presence is rejected.

On frozen source and cached image digests, a fresh macOS ARM64 Docker Desktop
run passed all 28 actual protocol checks and four recreation/persistence controls
in 43.36 seconds. This is a local functional observation, not a capacity or hosted
measurement. A separate fresh `check` preflight passed and removed its private
state without starting containers. All 154 Python tooling tests and 17 Node
compiler tests passed, including 16 local-service refusal/protocol tests and the
five release-policy contract tests. Scoped actionlint, regenerated conventions
and the scoped whitespace check passed. Locked npm dependencies were installed
after the initial broader tooling attempt found the reused worktree had none;
the final run passed. No engine compilation or GPU execution was needed for this
Python/configuration slice. Generated proof containers, volumes and private state
were removed; both pre-existing unrelated containers remained running.
[Exact source/result ledger](evidence/local-service-main84-readiness.json).

Independent Astra reviewer `/root/runtime_review` accepted the exact source
`54e521fd4af4ccb034afe526c96a4c51e44d2494` against integration base `3b3b565`
with no actionable blocker. The read-only review covered the two behavioral
corrections, regressions, workflow/context identity and staged protection rollout;
it ran no containers or builds. The following review-record update changes only
documentation and leaves the validated implementation hashes unchanged.

The workflow job is explicitly named `local-service-protocols`, and its required
context joins the release manifest and tested name contract for a total of 23.
The coordinator must install the additional branch-protection context **only for
the next candidate containing this reviewed workflow**, after the initial frozen
22-context main84 checkpoint. This local source change neither changes that
active candidate nor claims the new hosted job has run. Publication and all
main/release/candidate writes remain with the sole coordinator.

Review against PLAN revision 8 §2.7 confirms this is a bounded foundation:

| Boundary | Locally exercised | Still required |
| --- | --- | --- |
| PostgreSQL | Pinned major 17, TCP commit/rollback, wrong-password refusal and recreated-container persistence | Application services, a matched deployed RDS version, WAL archival/PITR, managed failover/backup gates |
| S3-compatible storage | SigV4 create/write/read/list/delete, exact bytes, negative authentication and persisted bytes | Application object adapters, full required S3 behavior and backup/cloud-sync integration |
| Local OIDC issuer | Discovery/JWKS, native public-client S256 exchange, state/nonce/audience/subject, provider userinfo and refusal controls | Accounts/player service, editor/browser login, a production client token verifier and WorkOS staging behavior |
| Captured SMTP | Local delivery, complete normalized message body and exact fixture deletion | Accounts-service email sign-in flows and production delivery |
| Service isolation | Digest/license pins, private generated values, internal backend network and verified loopback gateway | Full local-stack substitutes, production routes/rate limits, OpenTofu tests, observability, relay/TURN and network shaping |

No new outside service was introduced. The existing explicit business-feature,
license/SBOM, platform and hosted/device/production limitations remain in force.
