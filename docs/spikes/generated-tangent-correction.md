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
- All 101 scoped asset/import/renderer ordinary tests pass (61 GPU cases were
  run separately): `/tmp/incant-fbx-tangent-scoped.log`.
- Asset/renderer and import/headless release all-target Clippy pass:
  `/tmp/incant-fbx-tangent-clippy.log` and
  `/tmp/incant-fbx-tangent-host-clippy.log`.
- New ignored GPU test compiles and compares generated geometry with analytic
  explicit frames across four UV bases and three object placements. Opposite
  handedness is a non-vacuity control; retained original versions are redrawn
  after replacements. All 61 numerical renderer tests pass (14 unit and 47 integration):
  `/tmp/incant-fbx-tangent-gpu.log`. This includes the 12 UV/object comparisons
  inside the new test, without treating them as 12 separate Rust tests.

The original 0040 ledger contains two late material captures at 04:34:47Z after
the gate was withdrawn at 04:34:14Z; they overlapped the start of union GPU tests.
This was discovered after all processes exited and disclosed to the coordinator.
The union's 69 numerical tests passed; no elapsed time is a performance claim.
The delta uses a candidate-specific gate rechecked immediately before every child
launch, with CPU tests proving withdrawal prevents the next process. Release
requires all recorded children to be reaped and their executable absent from the
process inventory.


## Frozen appearance delta

[Claude0040's separate delta](../../handoffs/0040-static-fbx-import/tangent-delta-result.md)
accepts the generated-tangent correction on source `35f3f0d19f0b495a9337e49025ef3f57a16f6475`,
tree `e2c09ac1f25c6e43b0b652f6ecf3d9b2eb198e4f`. The 39,470,912-byte
headless candidate has SHA-256
`b2aed48f61fc310737c05afd945475124f68680aac2021838a510c758ff47506`.
ACP verified Opus 5.5 and Max again before the resumed capture prompt.

The bounded delta executed 12 captures and 34 child processes, all successful and
hash-checked on an Apple M5 Pro / Metal, macOS 26.6. Claude found the generated
control identical to the supplied-tangent reference and all five FBX frames
identical to glTF twins carrying Maya's supplied frame. Supplied-tangent objects
and colour controls remain unchanged; the twin project also contains a generated
quad, whose intended differences are explicitly separated in the report.
With sources deleted, both repeat frames are byte-identical. The original static,
reimport and UI rows were not rerun; their original frozen evidence is preserved.

Astra re-ran all 17 CPU-only helper tests, verified all 10 committed image hashes
without inspecting pixels, and checked the report against its process ledger.
The candidate grant was removed at 05:13:32Z; independent process inventory
confirmed all 34 recorded PIDs absent before release. No capture child ran after
that release. These are scoped desktop correctness/appearance results, not
phone, performance, shipped-player or phase-gate evidence. The corrected full
hosted matrix remains pending after foundation integration.

Claude0041 separately found alpha-weighted Color mips darkening OPAQUE material
RGB under minification. Emissive and Color environment consumers share the same
filter-policy boundary. That existing defect is outside this generated-tangent
change and remains open; this acceptance does not claim all texture filtering is
correct. A separate follow-up must preserve coverage filtering for BLEND/MASK and
sprites while providing alpha-independent color variants.
