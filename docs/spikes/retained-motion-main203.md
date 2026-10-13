# Retained motion on main203

The reviewed opt-in opaque/MASK motion/depth owner is composed with actual
main203 `78e18433d23bbaf4ae09b8e651d841660fac171c`. Source commit
`814717935462e4ee5dbdb1fdae2738d17477f932` and numerical evidence tip
`f40452012c96b9bca9904edc95402d0e4a724d8a` are preserved as ancestry.
All 22 producer inputs and 13 unchanged baselines match exactly. There are no
production integration seams. The lock change adds only the existing pinned
`naga` package as a renderer dev dependency for shader validation.

The tested private head `c2e3239b0a82a0c56f844991269167cf4bc65979` and actual-main
ancestry head `95e223e0dfa26313b7f38e7988e214a3fa4a4386` have identical trees.
All 2,889 frozen implementation/build files remain unchanged. Ten commands exit
zero: ordinary render/host tests (91 passed, 92 GPU cases ignored), overlapping
all-cooked motion-refusal tests (12 passed, 9 ignored), all-target Clippy, wasm32,
normal runtime graph, native-window all-target compile, formatting, conventions
and diff checks. Conventions ran twice; the identical empty logs share the same
paths, so this is ten executions and nine unique command labels.

The collector rehashes 16 original CPU artifacts, nine retained producer binaries,
and the full 15-record numerical packet against raw originals. The two accepted
numerical GPU tests remain producer-scoped: exact accepted-pose buffer/history
swaps and deformed/skinned/IK previous geometry, using the recorded binary. This
integration lane did not run GPU commands, inspect images, or accept appearance.
The old producer contract's GPU-pending prose is historical; the separate numeric
packet supplies its later scoped acceptance.

No color resolve, TAA, temporal upscaling, MetalFX, native cooked-motion path,
phone or performance gate is claimed. The timestamp candidate is separate; its
future composition must time the new motion pass independently or explicitly
exclude it from coverage. Existing default output remains unchanged.

[Machine-readable evidence](evidence/retained-motion-main203.json) retains the
freeze, commands, lossless logs, source identities and prior review receipts.
[Assertion-only collector](evidence/retained-motion-main203/collect.py) accepts
`--verify-only`. Final current-main integration review is pending.
