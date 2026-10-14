# Native structural operation fence

This default-off component supports the pending persisted rigid save route. It
adds no byte importer and does not relax public physics-save refusal.

`CheckpointOwner::preflight_structure` takes a strictly sorted unique body or
collider removal slice and peak body/collider insertion counts. It checks actual
live handles, attached-collider deletion and overlap, exact global arena
removal-generation headroom, native doubling capacity and the sum of persistent
island removal counters. A manually supplied zero-capacity native arena refuses insertion: the
native slow path would otherwise reserve zero. The inert decoder already
requires nonzero slot storage; this is an additional operation-time defense. Joint/soft history remains refused.

The returned non-Clone `StructuralPlan` borrows only the operation slice, retaining
an Arc to this owner's original dispatcher and its checked mutation revision. It
can survive an all-scenes preflight without borrowing any scene. Reads preserve
it; any mutable access or step invalidates it. `structural_permit(plan)` binds
exclusive execution without another work-budget charge. Execute sorted removals,
then the exact insertion counts, then `finish`. Unexpected operations refuse.
Dropping a partly consumed permit does not roll back requested ordinary edits;
outer callers must stage their input validation before consuming plans. Native
allocation failure behavior is unchanged; capacity admission is not a reservation
or a claim that host OOM is recoverable.

Six focused actual native controls passed in each dimension at source generation
structure-002, including cumulative generation refusal before the first removal,
attached collider generation, island sum, capacity, owner mismatch, stale plans,
zero remaining work after successful preflight, exact reuse and mutation revision
exhaustion. The prior four-test generation is preserved. Logs and immediate
executable copies are in
`~/.codex/incant-evidence/physics-native-persisted-001`. Root source review
`physics-structure-002-root-20261013.json` SHA
`d5c6c6420f0e53726590f6fba2d984b9271957b36ade4af45b610d49b6e3d70c`
accepts this operation fence only. Hostile numerical/domain admission and
next-step counter safeguards remain separate work.
