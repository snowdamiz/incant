# Compatible animated vertex refresh

Native Play now accepts replacement positions, normals, UVs and tangents for an
animated model whose indexed topology and pose bindings remain unchanged. This
lets an artist revise a mesh or regenerate tangent data without restarting its
running scripts or animation. A complete cold-decoded asset replaces the prior
asset atomically through the existing import/command/history path.

Vertex counts and indices remain exact. Scene/primitive bindings, skin weights,
joint order, inverse binds, rest transforms, clip identity, channel targets,
interpolation and duration still require compatibility. Root-motion catalogs
and late publication checks remain authoritative. Animated geometry does not
become eligible for baked navigation through this change. Skeleton and structural
edits still need their own admission rules.

Source `4d0b4f5a26cfd9b64f8cbf4283aade3752795964` passes 42 behavior controls across native rendering and Play,
plus both packages' all-target strict Clippy checks. The actual import/Undo/Redo
controls exercise geometry and generated-tangent replacements while retaining
the world, guest counter and animation clock. A staged candidate uses the pose
at publication even if playback advances during preparation. Publication remains
allocation/deallocation-free, root motion continues, and retained assets remain
usable after the private cache is deleted. The isolated index-only refusal proves
identical vertex and entire animation data before rejecting changed indices;
other controls preserve the old catalog on incompatible or late changes.

Qualification used Rust 1.99 on this Mac, the CI profile, locked/offline Cargo,
two compiler jobs and two Rayon threads. These CPU results do not establish GPU
pixels, actual editor-window behavior, shipping performance or phase approval.
The subsequent same-clock Metal checks and actual editor-window qualification
are described below.

Four attempts are retained in the [raw packet](evidence/animated-vertex-refresh.json.gz)
and [manifest](evidence/animated-vertex-refresh.json). CPU001 was deliberately
stopped before test execution to correct the fixture's cooked-vertex index;
flat-normal expansion maps raw vertex 0 to cooked vertex 1. CPU002 stopped because
the disk-usage monitor raced temporary compiler-file deletion. CPU003 added only
bounded retries for that exact meter error and passed 41 controls plus both lints.
CPU004 added the isolated index-only refusal and passed all 42 plus both lints.
Production admission code did not change between these attempts. The earlier
failures and their resource charges are not reset or relabeled.

Total retained attempt time is 221.824 seconds. The separate
target grew from 2,370,228,224 to 3,496,972,288 allocated bytes, below its
6,665,195,520-byte cap. The original headless executable stayed byte-identical.
Owned child groups were reaped and independently checked absent before source
changes or each successor. The source manifests and independent reviews are
included as inert records; they do not execute on replay.

## Same-clock Metal qualification

Source `d053542e69c7ce33ea64226bbd41d591caa8fe10` adds three fixture controls,
three actual Metal controls and a passing all-target strict renderer lint. Its
fresh CI-profile test executable has SHA-256
`d9f979ad8a8d92ca5e4d9eee0760a707c2b78fcbe57055b94c42be178ee34f6c`.
All three Metal controls pass. Position-only replacement narrows the animated
triangle from 276 to 204 red pixels, removing 72 and adding none; its complete
RGBA equals an independently loaded replacement at the same animation clock.
Tangent-only replacement changes all 36 sampled interior pixels while preserving
the geometry, images, materials and animation; its complete RGBA also equals the
fresh reference. Invalid and stale replacements retain the last accepted pixels.
The original clip-key regression still moves the 276-pixel triangle by 12 pixels.

Independent replay of all 18 retained captures confirms those comparisons.
The shared world and guest state continue, and replacement uses the current pose
rather than the pose when preparation began. The private cache is deleted before
publication. These are fixed-state offscreen correctness controls, not editor
appearance acceptance, temporal reconstruction or shipping performance results.

The additional CPU and GPU stages take 193.154 and 8.999 seconds respectively;
cumulative retained qualification time is 423.978 seconds. The target's allocated
peak is 4,214,726,656 bytes under the original 6,665,195,520-byte cap. All owned
process groups are reaped and independently absent. The [portable Metal packet](evidence/animated-vertex-metal/README.md)
preserves 91 inert records, all 18 captures and the original producer receipts.
Independent data replay and seven verifier tests pass; no native execution is
implied by replaying the archive.

## Actual editor workflow

Source `b3da3b6d9bbb6a8870480ae54d97a2884d20edde` passes the actual Mac editor
workflow after a fresh locked CI-profile editor/headless build, eight native
probe controls and both packages' all-target strict Clippy checks. The separate
18 Python fixture/receipt controls also pass. The generated frontend and pinned
TypeScript/SWC implementations are included in the producer identity.

The real import/history handlers and source watcher publish the compatible
position edit, Undo restores the original asset and Redo restores the replacement.
An index-only incompatible edit commits to authoring but retains the last accepted
running asset with the specific topology refusal. Restoring compatible source
recovers through the watcher. The same world and script counter survive every
stage; paused tick 2 and animation time 0.533333 seconds stay fixed during edits.
Step advances to tick 3, adds the expected script counter delta and advances the
animation by 1/60 second. Stale generations refuse and Stop releases the world.
A fresh headless process reopens journal revision 8 with unrelated authored bytes
unchanged.

All six managed app/headless calls complete successfully. The window wrapper
finishes in 18.763 seconds, with every observed child and wrapper identity checked
absent. The preceding build/test/lint sequence takes 561.614 seconds; cumulative
retained qualification cost is 1,004.355 seconds. Target allocation peaks at
8,039,895,040 bytes, within the explicitly expanded 9,886,420,992-byte editor-build
cap. The earlier CPU failures remain recorded. An unexecuted preparation packet
and metadata-only preflight parser correction are not native execution failures.

The [portable editor packet](evidence/animated-vertex-editor/README.md) preserves
83 inert records, the original build and window receipts, and independent root
review. Its saved-data verifier and five negative-control tests pass. Native
executables and dependency caches are explicitly excluded; replay does not
re-execute the editor.

This checks actual functional editor behavior and presentation on Apple M5 Pro.
It captures no pixels and does not qualify appearance, frame rate, temporal
reconstruction, shipping performance or a phase gate. The separate Metal checks
above establish the fixed-state vertex/tangent pixels.
