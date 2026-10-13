# Inert contact manifolds on main201

The reviewed inert manifold component is composed with actual main201
`ebccdfda1f41a0409eb3a566f06bb174d23cfd34`. Source commit `1da75a94`
changes nine crate paths exactly as accepted producer `7ca4dd87`; the only
cherry-pick conflict was STATUS. No source reconciliation or lock change was needed.

All 79 CPU cases pass in 13 nonempty groups, with zero failures or ignored cases.
This includes the prior 65 inert cases and 14 manifold cases; counts are not
additive. All-target Clippy, wasm32 library compilation, the std-only normal graph,
three runtime dependency guards, formatting, conventions and full-base whitespace
check pass. The eight commands ran on the checkout's private target; all 2,878
frozen inputs stayed byte-identical. No GPU or native-window execution occurred.

The collector verifies all 33 crate source/test/manifest inputs against the producer,
all 116 retained producer artifact records against their original bytes, and 21
pinned source/copy pairs. The producer's older lock differs only by already-merged
profiling packages/edges; the integration lock equals main201. New integration
outputs are retained as 19 lossless records with 10 distinct payloads. The complete
commands, source identities and receipts are in the
[integration manifest](evidence/physics-manifolds-main201.json).

[Second source review](evidence/physics-manifolds-main201/independent-source-review.json)
and [producer final acceptance](evidence/physics-manifolds-main201/producer-final-review.json)
are retained. Final current-main integration review is pending.

This remains an inert structural format. Pending-mutation journals explicitly
refuse; caller-supplied phase labels do not authenticate the world. Whole-owner
binding, reciprocal solver/island state, live construction and exact continuation
remain open. Physics save admission is unchanged and refused. No shipping,
performance, phone, appearance or phase gate claim follows from these checks.
