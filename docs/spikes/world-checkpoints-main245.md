# Native world checkpoint composition on main245

The grouped intake applies the accepted borrowed configuration views (`6252f4ee`),
aggregate world checkpoint (`9f335231`), compatible staging API (`6f782a26`) and
portable producer packet (`5f25c7dd`) to actual main245
`27e666be45433fe82228639708f7914e0caa8196`. No private producer ancestry is merged.

All 3,651 tracked implementation, build and tooling inputs are byte-identical to
the root's executed source004 prepared tree. The comparison covers both complete
path sets under `crates`, `third_party`, `tools`, `.cargo`, and the root manifests
and toolchain file. Three current and prepared host dependency graphs are exact
after replacing only their workspace root: ordinary, native deterministic and
native fast8. The ordinary graph excludes the optional checked restore/snapshot
edge. These are read-only graph queries, not new compilation.

Independent final packet inspection decoded all 342 retained records and compared
each with its original bytes, read the producer verifier, and verified all 142
immediately copied executable identities. The producer's source004 runtime
suites pass 261 cases per mode, with both strict Clippy modes and wasm checks.
The 205 ordinary physics/script consumer cases remain source003 results. The
source004 delta is confined to feature-gated exports and checkpoint code/tests;
no source004 consumer rerun is claimed. Earlier failed producer runs remain in
the original archive. Source003/source004 independent reviews are retained.

This default-off API coordinates opaque in-memory ECS and primitive physics
owners. It does not enable persisted physics, caller-supplied byte restoration,
script/host/session checkpointing, GPU or device gates. Budget limits and retained
native refusal boundaries remain as documented in the
[producer contract](native-world-checkpoints.md).

Run `python3 docs/spikes/evidence/world-checkpoints-main245/verify.py --verify-only`
for the portable input, dependency graph and producer-receipt qualification.
The [independent audit](evidence/world-checkpoints-main245/independent-audit.json)
records local executable verification; binaries themselves remain local.
