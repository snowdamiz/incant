# Project-bound Rust module build jobs

This source slice connects the bounded Rust module worker to one shared
`incant_module_build::Service`. The editor IPC and engine-agent adapters use the
same service instance. It inspects existing source and builds private candidates;
installation and source-write command transactions remain separate work.

The source and mock controls are implemented. Native compilation, actual bridge
execution and fresh schema-export comparison are pending CPU qualification. This
is not a new sandbox-isolation, shipping, runtime, device or visual acceptance.
The worker's prior isolation and ordinary-import evidence remains in
[untrusted-rust-module-build.md](untrusted-rust-module-build.md).

## Contract

- `module_source_read` / `engine_module_source_read` accept a project-relative
  package, relative Rust entry and required expected revision. They return bounded
  untrusted source data and an opaque source snapshot ID. There is no source write.
- `module_build` / `engine_module_build` accept that snapshot ID, expected revision
  and description. A successful build returns opaque build/candidate IDs, bounded
  untrusted diagnostics and engine-supplied actor/source/tool provenance. It does
  not import, select, install, execute or export the candidate.
- `module_build_status` and `module_build_cancel` (with corresponding
  `engine_` IPC names) accept only the observed snapshot ID. Cancellation keeps
  the job busy until the owned process cleanup returns. Cleanup uncertainty
  poisons new work and remains visible as `cleanup_unproven`.

`complete` is a historical build result for its explicit immutable snapshot and
observed revision. The pre/post checks reject observed changes; they do not hold
the authoring bus lock across receipt publication. Completion grants no current
document installation authority. Any future installation must revalidate project
session/revision/source identity in its own command transaction; provenance marks
this result `snapshot_only` and `installation_requires_revalidation`.

Snapshots are bound to a saved project's held root identity and a fresh session
ULID. Closing/rebinding even the same project ID invalidates prior handles and
cancels active work. Source bytes, recovery epoch and document/revision are
checked before and after a build. A new build retires the preceding candidate's
eligibility even when the new build fails. No project revision or history entry
is created by this build-only service.

Agent builds use the existing write-tool approval classification: Always and
Destructive ask through the existing approval route; Auto keeps its existing
policy. Read/status/cancel are not new write capabilities. Missing host setup,
unsupported platforms and unavailable projects fail explicitly; tools do not
provide shell access or model-controlled compiler/configuration paths.

## Optional trusted development host

`INCANT_MODULE_BUILD_CONFIG` names an external, owner-controlled JSON file. Its
fields are `python`, `worker`, `rustc`, `validator`, and `storage`, all absolute
canonical host paths. `worker` is the fixed engine `tools/rust_guest_job.py`;
`validator` is the normal qualified module validator executable. Storage must
already exist outside the project. Every trusted tool/config/SDK input is kept
outside project authority. Missing configuration leaves the editor usable and
reports module builds unavailable. macOS is the only implemented backend; other
platforms fail closed.

Configured executable/worker/SDK/config bytes are hashed and rechecked around
execution; private receipts retain their exact identities. Storage root identity
is held. System libraries and the installed toolchain remain trusted platform
inputs; this does not claim a hermetic toolchain or protection from a malicious
host administrator. The inherited environment is cleared. The existing deprecated
`sandbox-exec` policy remains unchanged and retains its documented limitations.

The fixed Python bridge captures source and reuses the normal worker/validator.
Rust independently admits the ordered compiler/SDK/sandbox/validator argv and
minimal environment, then directly owns each child process group. Private
mailbox messages never grant project or model execution authority. Only completed
JSON messages become visible. Successful results require native exit and owned
group cleanup; an exited/crashed bridge cannot waive a delegated child's cleanup.
On macOS, buffered libproc membership queries check errno and PID/group identity;
NULL-buffer capacity estimates are not absence evidence. Exit observation precedes
reaping, and uncertain cleanup retains the owner instead of blocking indefinitely
or silently discarding it.

Limits are 32 Rust files, 64 KiB/file, 256 KiB source total and depth eight; no
Cargo manifests, build scripts, non-Rust files or source links. Diagnostics are
bounded by UTF-8 bytes (64 KiB public, 4 MiB per child stream, 16 MiB combined).
Each service operation has a 180 s aggregate bound and 90 s child bound, with
512 MiB global retained host storage, 8 GiB sampled owned RSS and a 6 GiB free-disk
floor. Retained jobs count toward that global budget; this slice provides no
automatic deletion or cleanup credit.

## Verification boundary

The pure Python mailbox and existing worker controls pass: 42 tests. These start
no native children and do not establish OS isolation. Source tests cover shared
service source/identity bounds, stale/rebound projects, busy/cancel behavior,
revision/epoch validation, candidate retirement, cleanup poisoning, exact child
recipes/environment and agent approval with no document/history mutation.

Ignored macOS test sources additionally cover terminal/cancel cleanup, an exited
parent with a pipe-holding descendant, a crashed bridge with a separately owned
child, retained ownership after exit-observer refusal, and a real shared worker
build followed by cancellation after delegated compilation begins. These tests
are unexecuted until separately bounded CPU qualification.
The configured bridge test additionally requires the default-off
`incant_module_build/host-qualification` feature. Ordinary workspace ignored tests
never require a configured external worker or launch its compiler.

The four agent schemas are source-derived additions preserving every prior tool
value. A newly built matching headless exporter must verify the complete registry
before publication. Editor IPC compilation is required; no UI, native-window or
hardware claim follows from it.
