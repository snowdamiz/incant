# Inert pair persistence on main205

Accepted producer `77910a65` is composed with actual main205
`7bfeecfdfaa4a52ec198d1caee0fcb4ff9399fd9`. The source/test/manifest bytes of all
39 crate inputs equal the producer. STATUS was the only cherry-pick conflict.
The current lock retains all 968 package identities and adds only a direct
already-pinned dev-only serde edge to the inert snapshot crate.

All 88 CPU cases pass in 14 nonempty groups (16 summary rows), with zero failures
or ignored cases. These include the prior 79 manifold cases and are not additive.
All-target Clippy, wasm32 library compilation, the std-only normal dependency
graph, runtime graph guards, formatting, conventions and diff checks pass.
All 2,895 frozen inputs stayed byte-identical. The main205 ancestry merge preserves
the exact tested tree; no redundant build is claimed for that merge.

The collector rehashes all 130 producer raw/retained artifact records and 17
pinned source/copy pairs. It binds the eight actual integration command results,
full frozen inputs, lock comparison and ancestry to the
[integration manifest](evidence/physics-pair-main205.json).
[Producer final acceptance](evidence/physics-pair-main205/producer-final-review.json)
is retained. Current-main final integration review is pending.

This remains an inert format. It preserves pair event/color/recycle fields and
ordered composite-workspace detector keys, timestamps and manifold bijections.
Unknown workspace types refuse. Complete owner authentication, reciprocal
island/solver ownership, previous-cluster scratch reconstruction and live
construction remain open. Physics-save admission is unchanged and refused.
No shipping, GPU, phone, performance or phase gate is claimed.
