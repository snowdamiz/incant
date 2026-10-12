# Grouped animation pose sampling

Scope: the retained CPU model player can crossfade two complete local poses while
preserving each state’s interpolation rule. This is a prerequisite for graph
transitions involving flat 1D blends and nested weighted trees; graph/runtime,
save, SDK and renderer integration remains separate work.

`PoseSource` distinguishes one clip, a spherical pair, and a weighted quaternion
mean. The player samples each source first, then linearly blends translation and
scale and uses shortest-arc SLERP for the outer rotation. Flattening every clip
into one weighted list would change rotations even at transition endpoints.
Exact outer weights zero and one reuse the selected source path bit for bit.
Both complete sources validate, including inactive endpoints and zero-weight
entries. Hierarchy and skin palettes publish once after complete success.

One additional retained local-TRS array is allocated at construction and charged
by `retained_bytes`; accepted updates allocate nothing. Invalid input uses the
existing allocated asset diagnostic and may allocate. The primitive admits up to
sixteen contributions in each source; the graph connection must impose its own
stricter eight-per-state and sixteen-combined bounds.

The full affected assets/runtime suites pass 113 tests across eighteen groups,
with no failures or ignored tests. All-target Clippy, the supported wasm32 reader,
lean dependency guards, formatting and diff checks pass. Independent source and
recorded-log review accepted the seven frozen files without findings.

Coverage includes an independent quaternion-angle/skin-position oracle that
separates inner SLERP from a normalized weighted mean, all nine source-kind pair
endpoints, ordinary crossfade parity, translation/scale and fixed-matrix nodes,
invalid inactive inputs, composition-overflow rollback in both source directions,
and 1024 allocation-free accepted updates. The first allocation measurement also
included allocated rejection diagnostics and failed. That log is retained; the
claim and measurement were narrowed to accepted updates without changing the
numeric algorithm to hide a failure.

[Hashes, exact logs and independent review](evidence/grouped-animation-pose.json)
retain the measured evidence. This proves no rendered result, device performance,
complete animation feature or phase gate.
