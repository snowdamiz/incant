# Release checkpoint47

PR352 promoted selected main359 (`f7ffa52728496fea4f4d7dc06f14681af2c26c4b`)
after all 23 required contexts and all eight candidate workflows reported actual
success on frozen candidate `1146b8e1613c522396e2182954709e7fea6d216d`.

The release merge is `4b81e4a8a7c301306b2bb849c208d094f392eecb`. Its tree
`ff642d589d01dfdeac0198b40e5fd9f76c21b02f` exactly equals the tested candidate
and selected main tree. The merge includes `Built-by: astra`.
[Exact checks, workflow runs and identities](evidence/release-checkpoint-47.json).

The Windows desktop starter controls passed in this complete suite. Checkpoint46's
earlier eight-second cook timeout remains historical; neither this success nor the
separate passing diagnostic establishes the timeout's cause or a specific repair.
No failed checkpoint was promoted and no requirement was removed.

This baseline includes the reviewed cooked MetalFX boundary, native Metal archives,
material refresh, compiler-loader observations and bounded/local Responses work
through main359. Main360 and later changes remain outside this tested release tree.
No distribution was published, and rendering quality, full-game performance and
phase gates remain open.
