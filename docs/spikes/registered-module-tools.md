# Registered Rust module tools

This source slice adds the PLAN Appendix A registered-ID authoring workflow to
`incant_agent` and the existing shared `incant_module_build::Service`. It uses
`CommandBus::read_module_source` and `Command::WriteModuleSource`; source edits
retain the normal atomic transaction, actor provenance, persistent source journal,
revision checks and Undo/Redo. It adds no installation or module-selection path.
Rust behavioral controls and strict lint are authored but not yet executed for
this source. Formatting and source checks do not qualify runtime behavior.

The agent's preferred workflow is:

1. `module_read {module_id}` returns registered source/version/revision under
   `untrusted_source_data`. The ID is registered project content; it supplies no
   arbitrary path or compiler authority. A saved source store is required, but a
   compiler host is not. Missing registered content is readable as `Missing`.
2. `module_write` takes that module ID, source, `expected_source`,
   `expected_revision` and description. It replaces at most 64 KiB of Rust source
   through the existing command transaction. With the first-package source
   foundation, missing Rust parent directories are privately staged and published
   as one owned subtree through that same transaction; Undo/Redo retain and
   restore its exact identities. The adapter creates no separate filesystem path.
3. `module_inspect` takes the ID and observed source/revision. The engine derives
   the package/entry from the registration and captures the existing bounded
   package. Missing entry content must first be authored. The immutable snapshot
   binds the module ID, registration path, entry version, document revision and
   ephemeral command-bus session identity.
4. `module_build_registered` takes those observations, the returned snapshot ID
   and description. It uses the same bounded worker, fixed recipes, normal module
   validator, owned-child cleanup and private candidate service as legacy builds.
   Diagnostics remain untrusted data. It does not import, install, select, play
   or export a candidate.

Write/build use the existing destructive/always approval classification; Auto
keeps its existing policy. Before approval, the adapter captures the actual bus
session, registration, revision and source version. After approval/provider
fencing it rechecks them inside the dispatch guard. Cancellation, stale source,
retargeting, removal, recovery-required state and same-ID/revision project reopen
refuse without applying the pending source edit or starting the pending build.
The bus session ID is fresh per bus construction/reopen, nonserialized and stable
through ordinary edits and Undo/Redo. Registered builds revalidate actual
registration and entry bytes after worker execution too.

These are additive registered protocol v1 tools. Existing `module_source_read`,
`module_build`, status/cancel and editor IPC request schemas retain their legacy
package-diagnostic contracts. The legacy source/build response omits new
registered metadata. Legacy build cannot consume a registered snapshot, and the
registered build route cannot consume a legacy snapshot. Editor callers can use
`Service::inspect_registered`/`build_registered` with engine-supplied
`RegisteredSource` authority and an equivalent bus/source validation callback;
no editor UI or IPC contract is changed here.

A completed build is still a historical result for its explicit snapshot, not a
current-document installation capability. No bus lock is held across compilation
or final private receipt publication. Any future installation must independently
revalidate session, registration, source, revision and candidate authority through
its own shared command transaction. Status/cancel continue to use the snapshot
ID and the service's existing project-session binding.

The focused test sources cover registered read/write without a compiler host,
approval modes and denial, journal/actor/Undo/Redo, unknown IDs and unknown fields,
source/revision CAS, source bounds and creation, recovery refusal, external edits,
retarget/remove, same-ID reopen before approval dispatch and during a build,
cancellation before/during build, private-candidate refusal, cross-protocol
snapshot refusal and legacy response omission. They use deterministic mock
providers/backends and synthesized filesystem edits; they make no native sandbox,
network, device, window, rendering, audio, shipping or performance claim. The
existing qualified worker policy and native build evidence are separate in
[module-build-host-jobs.md](module-build-host-jobs.md).
