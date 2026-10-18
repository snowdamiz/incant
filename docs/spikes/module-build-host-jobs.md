# Project-bound Rust module build jobs

This source slice connects the bounded Rust module worker to one shared
`incant_module_build::Service`. The editor IPC and engine-agent adapters use the
same service instance. It inspects existing source and builds private candidates;
installation and source-write command transactions remain separate work.

The source slice has scoped macOS CPU qualification. The service/agent/worker
producer is `a0dd8eb7`; the editor guard correction and fresh schema producer is
`c9733e5c`. This is not a new sandbox-isolation, shipping, runtime, device or visual
acceptance. The worker's prior isolation and ordinary-import evidence remains in
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
The portable service uses held filesystem identities: Unix device/inode or
Windows volume/file index. Windows directory opens use backup semantics and
no-reparse, no-DELETE-sharing handles; replacement is refused at the filesystem
boundary. Native compilation itself remains unavailable off macOS.

Limits are 32 Rust files, 64 KiB/file, 256 KiB source total and depth eight; no
Cargo manifests, build scripts, non-Rust files or source links. Diagnostics are
bounded by UTF-8 bytes (64 KiB public, 4 MiB per child stream, 16 MiB combined).
Each service operation has a 180 s aggregate bound and 90 s child bound, with
512 MiB global retained host storage, 8 GiB sampled owned RSS and a 6 GiB free-disk
floor. Retained jobs count toward that global budget; this slice provides no
automatic deletion or cleanup credit.

## Qualification

The 42 pure Python mailbox and existing worker controls pass. They start no
native children and do not establish OS isolation. The managed native episode
retains these distinct attempts:

| Attempt / producer | Actual result | Scope |
| --- | --- | --- |
| CPU001 / `9984d54b` | session 81328, terminal 1, 62.416758 s | Eleven shared controls and default ignored inventory pass; agent approval fixture returns `Budget` with its insufficient 100-token test budget. Later stages unrun. |
| CPU002 / `a0dd8eb7` | session 52495, terminal 1, 269.008551 s | Thirteen shared controls, two agent controls, both ignored inventories, five actual lifecycle/worker controls and service/agent strict all-target Clippy pass. Editor lint refuses two `collapsible_if` guards; schema stages unrun. |
| CPU003 / `c9733e5c` | session 79787, terminal 0, 171.767952 s | Only pending editor strict lint, fresh headless build and complete schema comparison pass. Service/agent/worker source and earlier passing evidence are carried unchanged. |

The first correction also adds portable held-root identity and opened-handle
regular/single-link checks. The shared mock tests remain enabled on Windows;
these macOS results do not execute that platform's API path. The second correction
only collapses the editor callback's equivalent error guards. Both failed attempts
remain failures in the packet. Total elapsed is 503.193261 s of the original
900 s; no retry budget or storage history was reset.

The selected Rust checks cover source/identity bounds, stale/rebound projects,
busy/cancel behavior, revision/epoch validation, candidate retirement, cleanup
poisoning, exact child recipes/environment and agent approval with zero document,
history or project-file mutation. The actual five controls additionally exercise:

- A real source inspect, SDK/Wasm compile, normal validation, private candidate
  return, then cancellation after the delegated compiler starts.
- Terminal and cancelled direct-child cleanup.
- An exited parent with a pipe-holding descendant.
- A crashed bridge while a separately owned child still requires cleanup.
- An exit-observer refusal that retains the unreaped owner without blocking wait.

The configured bridge test requires the default-off
`incant_module_build/host-qualification` feature. Actual default `--list --ignored`
excludes it; the explicit feature inventory includes it. The four self-contained
process controls remain in the ordinary ignored inventory. Each direct-process
control asserts its own group closure; the outer packet independently records and
rechecks every observed identity. Fresh post-terminal checks found 50, 623 and
110 recorded PIDs respectively absent (including wrappers), and all recorded
groups absent. These observations are separate from broad OS isolation claims.

All Cargo stages use pinned Rust 1.99, `--locked --offline --profile ci -j2`,
Rayon 2, renderer opt-level 2 and sha2 opt-level 3. The exact commands, environments,
source/tool bindings, actual raw stdout/stderr and owned-process receipts are in
[the portable manifest](evidence/module-build-host-jobs/manifest.json) and its
bounded, data-only [replay](evidence/module-build-host-jobs/verify.py). Run:

```sh
python3 docs/spikes/evidence/module-build-host-jobs/verify.py --self-test
```

It replays 152 inert records and six meaningful refusal controls; it executes no
archived code. Local-only `--originals --artifacts` additionally rehashes the
retained originals and eight external executable/Wasm identities. Executable bytes
are excluded from the archive. The unchanged standalone isolation canaries and
ordinary import/Undo/Redo proof are retained separately and were not repeated.

The fresh headless copy is 63,697,952 bytes, SHA-256
`5709e749e813eb3c39627de54baa0d589950d49501c416655202c5fcaf280ead`,
Cargo `fresh:false` at `c9733e5c`. Its export matched every published schema file
byte for byte, including the four new tool schemas and all prior entries.
Final shared target allocation/retained peak was 13,524,672,512 bytes under the
existing 18 GiB cap. The 99 protected artifact identities and prior global charges
were retained; global totals became 5,187.800573 s post-cleanup and 5,646.805217 s
including deleted history. The target was released after actual terminal cleanup.

No UI interaction, native window, screen capture, physical input/audio device,
GPU, candidate installation, source write or source-free export was exercised.
Installation and stable-ID source-write adapters remain a distinct command-bus
integration task.
