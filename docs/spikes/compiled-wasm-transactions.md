# Journal-owned compiled Wasm publication

This source-only foundation allows the ordinary importer to validate host-owned
Wasm bytes and publish the generated `.wasm` source plus its asset document in
one `incant_cmd` transaction. It does not yet resolve module-service candidate
IDs, expose an agent/editor tool, build modules, or activate a scene program.
The private build service and its registered-source adapters remain separate.
All new Rust controls below are authored but not executed; generated contracts
and cross-platform native qualification remain pending.

`PreparedCompiledWasm::prepare` takes a registered module source ID, exact
revision, source version and destination version. The host root must match the
bus's held source root. It derives only the registered entry's `.wasm` sibling;
there is no candidate-path or arbitrary destination-path argument. Ordinary
`CapturedModule::capture/store` validates ABI/metering and builds the same cache
fingerprint as ordinary project-file import. Cache preparation may retain
unreferenced immutable derived data, as existing imports do.

The raw candidate and any existing destination are each bounded to 1 MiB, matching
`incant_wasm_cook::MAX_MODULE_BYTES`. Preparation owns a byte copy and captures the
bus session, full document/revision, source version, destination hash, exact named
file identity and parent chain. Publication rechecks these, including the source
hash at the durable file boundary. A same-ID/revision reopened bus, external
source edit, same-content destination replacement, alias or unrelated collision
refuses. A prior destination may be replaced only through its existing ordinary
Wasm asset identity; unregistered files are never adopted implicitly.
The nonserialized preparation retains the observed destination and every parent
handle until publication finishes, preventing their identities from being
recycled during that interval. Unix tests unlink the file and parents while the
held nodes remain alive; the Windows control checks that parent replacement is
denied until preparation is dropped.

`BinaryEdit` is a separate optional transaction field; old text source fields and
journal versions retain their encodings. Version-5 binary Prepare/Commit/Abort
records use the existing hash chain and causal document snapshots. The bus
reserves space for Prepare plus the larger terminal record before writing a
pending marker or installing project bytes. The 128 MiB project-record ceiling
is not permission to enlarge binary input. Only one binary asset is published
per transaction, with exactly one UpsertAsset and no activation command.

Before Prepare, candidate bytes are copied to a create-only private transaction
slot. A receipt binds its exact file identity/hash/length, source hash, derived
path, root/parent identities and canonical journal binding. After durable Prepare,
the existing destination is moved without replacement into its retained slot,
then the candidate node is moved into the vacant destination. Undo/Redo reuse
these exact nodes; they do not re-read ephemeral build storage, compile, or trust
an arbitrary supplied path. No owned node is recursively deleted. Unexpected
nodes/bytes preserve the pending marker and require recovery rather than being
clobbered. Arbitrary external writers are not locked: an ambiguous concurrent
change fails closed and may leave retained nodes for reconciliation.

Completed asset/file history survives save/reopen and private candidate removal.
The dependency on exact filesystem identities is intentional: copied/moved
projects do not inherit old binary or first-package Undo authority. These
receipts are not silently rebound. Ordinary same-path reopen is supported.
Persisted receipts check identities and bytes but cannot pin filesystem IDs across
process exit: they do not establish security against an external writer who can
delete retained state and recreate matching data with reused IDs. In-process
preparation pinning and durable recovery are distinct guarantees.
Private retained nodes must not be treated as disposable cache while their
history remains live.

Authored controls cover prepare-without-publication, exact ordinary import
fingerprints, invalid ABI/raw bounds/root refusal, source/destination/revision and
bus-session conflicts, same-content file/parent replacement, links, slot corruption,
terminal journal capacity, prior-byte restoration, cancellation by dropping a
preparation, save/reopen/Undo/Redo without the private candidate, and the combined
first-package/install/Undo-both/Redo-both sequence. A real subprocess fixture kills
apply, Undo and Redo at seven durable boundaries, including torn Prepare/Commit.
Another kills publication between displacement of the previous file and install
of its replacement. Mixed pending protocols refuse before moving either file.
A mixed TypeScript/binary history control preserves the version-1/2 journal prefix
byte for byte and checks both Undo/Redo orders after reopen.
These process controls have not been executed in this source-only work.

The future candidate adapter must additionally resolve a successful candidate ID
inside the service generation, require registered source authority, verify the
entire package digest/recovery epoch and worker receipt, re-read/verify candidate
bytes, and apply user/agent approval before this publication call. The entry-file
CAS here does not replace complete package-snapshot validation. Cancellation
before publication drops preparation; cancellation after the durable protocol
starts must let it finish or recover. No shell authority or runtime dependency is
introduced. Source copying from an unrelated private package and multi-file
installation remain outside this foundation.
