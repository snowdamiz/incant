# Outer compound and tile checkpoint joins

This private implementation extends the Incant-owned joins around opaque native
checkpoints. It does not admit physics from saved bytes. Ordinary `save_bytes`
and scene-load capability checks still return `PhysicsUnsupported`.

The 3D join compares the exact ordered primitive children of the live compound
against the retained stable part IDs, f32 dimensions, local translation, and the
same f32-normalized rotation used at structural installation. IDs must remain in
canonical order. The outer layer never reconstructs a compound or its BVH.
The native compound boundary separately owns exact child/root AABBs, acceleration
state and private pseudo-normal admission. Nonempty compound flags refuse here.

The planar join retains automatic tile rectangles in their original prepared
order. It requires a fixed collider with the default tile material and groups,
identity child rotation, and exact native box dimensions and centers. An additive
borrowed geometry iterator exposes both primitives and tiles without granting
completed-step authority. The previous primitive-only iterator still refuses
when a tile row is present.

The ECS join requires exact stable entity membership, automatic-tile binding and
prepared-map membership, and reproduces the existing rectangle-to-center/half
formula without allocating or recooking geometry. Pending ECS numeric inputs
remain separate from the last committed native pose. Reconstructing a tile scene
requires its captured planar owner. Supplied native geometry is never installed
again through the structural authoring path.

Compound part vectors and every stable-ID string, and tile rectangle vectors,
are fallibly deep-copied under the same cumulative payload budget as native
capture/restoration. Visits are charged before deep joins. The additional outer
compound quota is at most 262,144 parts across scenes, with 1–64 parts per
collider and 26-byte IDs; planar shapes retain their existing aggregate 262,144
limit. These are logical payload/work bounds, not allocator overhead or RSS.

Source001 is based on `42e13e75c3768dcc88783543ec573ce875850e00`. Focused native
geometry joins (9), native-feature copy/budget controls (2), and the exact ECS
tile binding/map-refusal control (1) passed: 12 unique tests. The first ECS run
passed with an unnecessary-mut warning; source002 removes only that test qualifier
and the exact test passes again without warnings. All 15 production/test source
hashes are retained, and all production bytes are identical between freezes.
[Portable receipts](evidence/outer-compound-checkpoints/manifest.json) bind the
four terminal commands and both source freezes. These are focused default-mode
checks, not full-suite qualification. The new
full compound/tile checkpoint, sleep/wake, sensor/removal/reuse, query and ECS
continuation tests await the agreed lower Parry/Rapier compound implementation.
There is no claim that those tests or complete native compound restoration have
passed on this base. Both mode qualification and independent source review remain
pending. No GPU, appearance, phone, or performance evidence is involved.
