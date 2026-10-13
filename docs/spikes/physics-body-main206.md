# Inert regular body state on main206

Accepted producer `c250c23e` is composed with actual main206. The 44 crate
source/test/manifest inputs are byte-identical to the accepted producer; STATUS
was the only cherry-pick conflict. Cargo.lock and all dependency identities equal
main206. The normal graph remains the single std-only inert crate.

All 101 CPU cases pass in 16 nonempty groups (18 summary rows), with no failures
or ignored cases. Counts include the preceding 88 pair cases and are not additive.
All-target Clippy, wasm32 library compilation, normal/runtime dependency guards,
formatting, conventions and full-base diff checks pass. All 2,900 frozen inputs
stayed unchanged. The actual-main ancestry join preserves the tested tree.

The collector binds eight executed checks, full frozen inputs, 99 rehashed
producer raw/retained artifact records, 20 pinned reference pairs and the
[producer acceptance](evidence/physics-body-main206/producer-final-review.json)
to the [integration manifest](evidence/physics-body-main206.json).
[Current-main final acceptance](evidence/physics-body-main206/final-review.json) is retained. Publication requires the exact-head fast check.

The format retains fixed regular-body force/mass/velocity/CCD/activation fields,
exact user bits and island scalar references, bound to checked body/attachment
rows. Unknown body kinds, unsupported soft owners, malformed canonical frames
and out-of-domain values refuse. Derived-state consistency, reciprocal owner
validation, safe live construction and exact continuation remain open. Physics
save admission stays refused. No shipping, GPU, device, performance or phase gate
claim follows from this inert component.
