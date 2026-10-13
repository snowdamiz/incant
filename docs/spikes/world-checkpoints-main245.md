# Native world checkpoint composition on main245

The grouped intake applies the accepted borrowed configuration views (`6252f4ee`),
aggregate world checkpoint (`9f335231`), compatible staging API (`6f782a26`) and
portable producer packet (`5f25c7dd`) to actual main245
`27e666be45433fe82228639708f7914e0caa8196`. No private producer ancestry is merged.

## Original composition and retained evidence

At reviewed `cc3e1f3f`, all 3,651 tracked implementation, build and tooling inputs
were byte-identical to the root's executed source004 prepared tree. Both complete
path sets under `crates`, `third_party`, `tools`, `.cargo`, and root manifests and
toolchain were compared. Three host dependency graphs matched after replacing
only their workspace root: ordinary, native deterministic and native fast8.
Independent root composition review `3b4dfff1` is retained.

Independent packet inspection decoded all 342 retained records and compared
each with its original bytes, read the producer verifier, and verified all 142
immediately copied executable identities. The producer's source004 runtime
suites pass 261 cases per mode, with both strict Clippy modes and wasm checks.
The 205 ordinary physics/script consumer cases remain source003 results. The
source004 delta is confined to feature-gated exports and checkpoint code/tests;
no source004 consumer rerun is claimed. Earlier failed producer runs remain in
the original archive. Source003/source004 independent reviews are retained.

## Manifest forwarding repair before publication

A subsequent all-path manifest audit on the held PR head reported twelve real
policy omissions across the optional snapshot edges of both physics runtimes
and `incant_runtime`. The selected normal graphs had passed their five-solver
mode checks; no mixed solver mode was observed. The passing old PR fast gate was
retained without merging that head.

Reviewed repair `78fa7e8b` changes exactly those three manifests: optional snapshot
dependencies disable defaults, and numerical features weakly forward the selected
snapshot mode. Algorithms, Rust source and lock bytes are unchanged. The new
3651-input freeze permits precisely these three old/new identities relative to
the original composition; its graph strings are a separate evidence generation.

All seventeen repaired checks pass, including 261 runtime cases per mode,
strict all-target Clippy in both modes, wasm, runtime/script manifest audits,
existing policy tests, formatting and conventions. Three fresh selected normal
graphs pass the actual five-solver audit. The ordinary graph excludes snapshot;
enabled graphs select construction plus exactly their chosen snapshot mode.
Sixty-four executed test binaries were copied immediately before target reuse.
The initial qualification verifier rejected unrelated multi-version dependency
names; that verifier-only failure is retained and its identity check now scopes
the five solver packages and snapshot, matching the existing audited tool.

The [repair manifest](evidence/world-checkpoints-main245/feature-repair.json)
retains raw failure, passing command records, frozen sources and binary identities.
Run `python3 docs/spikes/evidence/world-checkpoints-main245/verify-repaired.py --verify-only`
for current input and historical evidence verification; `--local-binaries` also
hashes the repaired executable copies. The original `verify.py` describes the
pre-repair generation and remains byte-identical. The repaired verifier checks
the unchanged original producer verifier against its retained source copies,
without presenting historical source004 as the new source.

This default-off API coordinates opaque in-memory ECS and primitive physics
owners. It does not enable persisted physics, caller-supplied byte restoration,
script/host/session checkpointing, GPU or device gates. Budget limits and retained
native refusals remain as documented in the
[producer contract](native-world-checkpoints.md). Final independent repaired evidence review is accepted (`03f84b12`), with its
receipt retained. A new exact-head required fast gate precedes publication.
