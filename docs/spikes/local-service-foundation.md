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

The dedicated Linux workflow runs only for this service stack, its helpers/tests
or its own workflow. It does not widen unrelated PR matrices or alter an existing
engine check. Its first hosted execution remains pending publication. No local
protocol test substitutes for real WorkOS/AWS/store/device/region gates. Accounts
service implementation, WAL-to-S3/PITR, the other PLAN §2.7 services and production
operations remain separate work.
