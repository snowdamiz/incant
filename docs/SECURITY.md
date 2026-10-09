# Phase 0 trust boundaries

Project JSON, asset paths, script source, entity names, model output and tool-returned
content are untrusted. Project mutations enter incant_cmd, validate atomically,
retain provenance and can be undone. Revision checks reject stale writes. Incoming
CRDT bytes merge on a fork and must satisfy semantic validation before publication.
No remote collaboration listener is exposed by this spike.

QuickJS has bounded memory, stack, execution time, input/state/output size and command
count. There is no installed module loader, filesystem, network, shell or host process
API. Scripts use a disposable runtime command bus during play. Native extensions are
not loaded. These boundaries require independent adversarial review before release;
the current tests are engineering evidence, not a security certification.

Windows/Linux provider credentials use Incant's OS credential-store namespace.
At the director's request, macOS uses private 0700/0600 local credential files to
avoid repeated Keychain prompts across development builds. These files rely on
OS file permissions, with no additional encryption. Metadata files
contain account identifiers, registration/client identifiers and connection selection,
not tokens or API keys. OAuth callbacks check path/method, state, duplicate parameters,
PKCE, nonce and verified token claims. Credentials are never included in provider
errors, projects, eval reports, console fixtures or CI artifacts. No telemetry runs.
CI has no coding-agent credentials; the live eval workflow uses a separate protected
secret and only runs from the default branch. It must not run unreviewed code.

Agent system instructions label project content untrusted, but prompts are not a
security boundary by themselves. The finite tool registry and command validation
constrain actions. Approval mode controls whether patches run automatically. The
live eval explicitly uses automatic mode in disposable fixtures. No engine-agent
shell tool exists. Successful output text alone does not count as a completed turn:
the Responses stream must contain a successful terminal response and usage.

Durable journal entries are hash-chained and sync before publishing edits. A partial
last line can be truncated after a crash; corrupted complete lines fail closed. Hashes
are integrity checks against accidents, not protection against a malicious local user
who can rewrite the entire journal. Journal compaction, complete resource-exhaustion
hardening and a production security review remain required.
