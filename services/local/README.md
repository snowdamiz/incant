# Local service foundation

This is the first executable slice of PLAN §2.7: PostgreSQL, an S3-compatible
object store, a local Zitadel OIDC issuer and captured SMTP. It needs Docker
Compose and Python 3.12 or newer. It needs no hosted account or supplied
credential. The first start downloads the exact public image digests.

```sh
python3 tools/local_services.py up
python3 tools/local_services.py probe
python3 tools/local_services.py down
```

`python3 tools/local_services.py check` prepares private instance configuration
and validates pinned images, retained licenses, the local Docker endpoint and
the Compose boundary without starting containers.

`up` waits for all five containers and checks local OIDC discovery. `probe`
creates uniquely named disposable protocol fixtures, then removes them. `down`
stops only this Compose project and retains its data volumes. To run a complete
fresh-instance and restart test without touching the development instance:

```sh
python3 tools/local_services_smoke.py
```

The smoke command owns a newly generated `incant-local-proof-*` project, removes
its own containers/volumes in `finally`, and prints only a sanitized result.
An interrupted process can leave that named test project; inspect its exact
name before manually removing it. The normal development volumes are never
removed by these commands.

| Protocol | Default host endpoint |
| --- | --- |
| PostgreSQL | `127.0.0.1:15432`, database/user `incant` |
| S3, path-style, region `us-east-1` | `http://127.0.0.1:18333` |
| OIDC issuer/discovery | `http://127.0.0.1:18080` |
| SMTP capture | `127.0.0.1:11025` |
| Mailpit capture API | `http://127.0.0.1:18025` |

Use the same `--instance NAME --port-offset NUMBER` on each command to select a
separate project. Port collisions fail startup. The helper refuses remote Docker
contexts, symlinked/incomplete state and
non-private POSIX state. No Docker socket enters a container.
Ambient `INCANT_LOCAL_*` variables are stripped before invoking Docker; only the
validated private instance configuration controls the stack.

Generated database passwords, S3 keys, the local identity bootstrap token and
master key stay in `~/.incant-local-services/NAME/` outside the checkout and authored projects, private to the local
OS user. They are disposable local values, not WorkOS/AWS/provider credentials.
Do not copy them into a project, commit, report or log. Bootstrap PAT lifetime is
seven days; this foundation does not implement admin-token rotation. A fresh
named instance provides a fresh bootstrap for protocol testing. The generated
admin password is local bootstrap configuration, not editor sign-in integration.

Backends join only an internal Docker network. A non-root, read-only HAProxy
container joins that network and a separate edge network, publishing only IPv4
loopback ports and forwarding to five fixed backend destinations. It is not a
forward proxy. This accommodates Docker 29, which does not publish ports for
containers attached only to an internal network. The gateway's health check
requires every backend port, and the helper additionally verifies discovery.
Zitadel telemetry/service-ping exporters and Mailpit version checks are disabled.
No service needs external network access after image pulls; only the fixed TCP
gateway joins the edge network. Local HTTP is intentional; this is not a
production TLS configuration. CPU, memory, process, connection and object-volume
limits keep this fixture bounded; it is not a capacity configuration.

## What the proof covers

The actual containers must pass TCP SQL commit/rollback and wrong-password
refusal; SigV4 S3 bucket/object creation, exact-byte retrieval, listing, deletion,
missing-object, wrong-signature and anonymous-write refusal; SMTP capture and
exact body; OIDC discovery/JWKS, public native authorization-code + S256 PKCE,
state/nonce/issuer/audience/subject, successful provider userinfo, wrong password,
wrong verifier and code replay. The smoke proof repeats all protocols after
container recreation and verifies persistent SQL and object bytes, stable issuer
and unchanged private settings.

The OIDC probe uses Zitadel's session API as a local login client. It does not
claim a browser login, editor account flow, production token-verifier library or
cryptographic ID-token verification. The real provider validates the issued
access token through its userinfo endpoint; the probe checks ID-token claims and
the advertised signing-key identity separately. Test users/clients and the
short-lived login client are removed afterward. No hosted gate passes here.

## Pins and licenses

`images.lock.json` records immutable image-index digests, selected versions,
application licenses and source URLs; retained upstream license texts are hashed.
The index manifests include Linux amd64 and arm64. Execution evidence currently
covers the recorded host only; an index entry is not a platform test.

The narrowly scoped local-container allowlist accepts PostgreSQL, Apache-2.0,
MIT, GPL-2.0-only (HAProxy's retained OpenSSL exemption), and AGPL-3.0-only for
Zitadel. Zitadel v3+ uses AGPL; this does not mislabel it as the older Apache
edition. These unmodified services are not linked into or bundled with engine
players. This is not the plan's complete transitive dependency/OS-image SBOM,
vulnerability audit or production distribution/legal review. License or digest
changes require review of the retained source and notices.

## Explicit remaining work

The dedicated Linux `local-service-protocols` job runs on frozen release-candidate
pushes or explicit dispatch. It adds no per-PR or main container matrix. Its
required context is introduced only when the next candidate contains this
workflow; the initial active 22-context checkpoint remains unchanged. The local
source revalidation is recorded in
[integration evidence](../../docs/spikes/local-service-foundation.md#main84-integration-and-checkpoint-readiness).

No Incant accounts/player service, object adapter, CRDT collaboration service,
relay/TURN, payment fake, platform sign-in fake, store/cloud-save abstraction,
metrics/logging stack or local device executor is implemented by this slice.
Postgres is pinned to major 17; no deployed RDS configuration is asserted.
WAL-to-object-store archival and a PITR drill remain unimplemented. Production TLS,
provider migration, real sandbox/device/region tests, secrets rotation, backups,
load/capacity and security review remain separate work. Compatible OCI runtimes
and native Windows hosts have not yet been executed in local evidence.
