# Generated glTF/FBX normal-map tangent correction

Claude0040 accepted the bounded static FBX geometry, hierarchy, material partitions,
colour-map and reimport paths, but rejected the shared generated normal-map basis.
The same inversion occurs on glTF without supplied `TANGENT`; it is an existing
shared importer defect, not an FBX parser or shader regression. The original
[0040 result](../../handoffs/0040-static-fbx-import/result.md), binary and captures
remain immutable. Prewarm candidates and PR80 do not include this correction and
must treat that appearance as the existing baseline.

Mikk encodes a bitangent sign following increasing texture V. glTF's image origin
is top-left, and its normal-map +Y uses the opposite bitangent direction. The
shared generated-only callback now negates Mikk `w`. Supplied glTF tangent vectors,
UVs, normals, FBX UV conversion, shaders and runtime geometry readers are unchanged.
This matches the primary [Mikk integration guidance for glTF](https://github.com/donmccurdy/mikktspace-wasm/blob/main/README.md)
and the [glTF normal/tangent convention](https://github.com/KhronosGroup/glTF/blob/main/specification/2.0/Specification.adoc).

The independently authored CPU triangle failed before this change, reconstructing
`(0,-1,0)` for the expected `(0,1,0)` bitangent. Four analytic UV-mirror bases now
pass, as does the real Maya cube's +Z face after cooking. A separate import test
checks that authored +Y tangent directions with both signs survive cooking
unchanged, even though the UV-derived tangent would point along +X.

Both glTF and FBX cook recipe salts advance. A regression seeds valid entries with
the exact old recipes, proves unchanged-source reimport misses those entries and
then hits the new ones, removes sources, and loads both generations. Existing
immutable cooked versions retain their old appearance until source is restored
and reimport/recook runs through the shared command path. Old versions remain
available for Undo and pinned scenes. There is no runtime repair, forced cache
deletion, format change, or source parser in the shipped reader.

## Scoped verification

- Before-fix numeric failure: `/tmp/incant-fbx-tangent-before.log`.
- All 52 asset tests pass, including the actual previous-recipe cache and real
  exporter controls: `/tmp/incant-fbx-tangent-assets.log`.
- All 101 scoped asset/import/renderer ordinary tests pass (61 GPU cases remain
  explicitly ignored): `/tmp/incant-fbx-tangent-scoped.log`.
- Asset/renderer and import/headless release all-target Clippy pass:
  `/tmp/incant-fbx-tangent-clippy.log` and
  `/tmp/incant-fbx-tangent-host-clippy.log`.
- New ignored GPU test compiles and compares generated geometry with analytic
  explicit frames across four UV bases and three object placements. Opposite
  handedness is a non-vacuity control; retained original versions are redrawn
  after replacements. Numerical execution and separate Claude appearance delta
  remain pending; neither compilation nor CPU tests are visual acceptance.

The original 0040 ledger contains two late material captures at 04:34:47Z after
the gate was withdrawn at 04:34:14Z; they overlapped the start of union GPU tests.
This was discovered after all processes exited and disclosed to the coordinator.
The union's 69 numerical tests passed; no elapsed time is a performance claim.
The delta uses a candidate-specific gate rechecked immediately before every child
launch, with CPU tests proving withdrawal prevents the next process. Release
requires all recorded children to be reaped and their executable absent from the
process inventory.
