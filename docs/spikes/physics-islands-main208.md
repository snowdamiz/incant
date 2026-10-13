# Inert island ownership on main208

The accepted island producer `b09beb36` is composed directly on actual main208
(`d15b0bd2`) at `aa917a4f`. Only STATUS conflicted; all 52 crate source/test/manifest
inputs remain byte-identical to the accepted producer. Cargo.lock equals main208,
with the same 968 package identities. No runtime consumer or physics-save
admission is added.

All eight scoped commands pass against 2,914 unchanged build/source inputs:
115 CPU cases across 17 nonempty groups (19 summaries including empty groups),
all-target Clippy, wasm32, the std-only normal dependency tree, runtime dependency
guards, formatting, conventions and diff checks. The 115 cases include earlier
body/pair coverage and are not additive to those prior counts. No GPU, shipping
package, restored solver or device execution is claimed.

The integration collector also rehashes 120 retained producer records against
raw originals and all 24 pinned native source references. Actual prior native
observations cover active/persistent membership, sleeping and empty awake
containers, retained fixed-body membership, free-slot reuse and deferred splits.
The original failures and corrected fixture/observer expectations remain in the
producer packet. Independent producer source and final evidence acceptance is
retained; final current-main integration review is pending.

Stamp-sensitive scratch, cooldown overflow during execution, solver buckets and
caches, complete collider/owner authentication and exact live continuation remain
open. The public record remains inert and `PhysicsUnsupported` stays closed.

See the [component contract](physics-inert-islands.md),
[machine-readable packet](evidence/physics-islands-main208.json), and
[assertion-only collector](evidence/physics-islands-main208/collect.py).
