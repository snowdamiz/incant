# Release checkpoint51

[PR383](https://github.com/snowdamiz/incant/pull/383) promoted selected main382
`9f11bbca54c3a3012733fdc4fad348d9abb92426` after all 23 required contexts and all
eight push workflows reported success on frozen candidate
`3da5f0451c800f2d6936e536a9bb5ce24b979058`.

The October 17 release merge is `e354bae15da4b9f41fdc08f59e0ecd2a45eb097a`.
Its tree `87ee2a4162e178bf4b5edbc8f36d3d3bc2fe77f0` equals both the tested
candidate and selected main. Git independently confirms those three trees, the
previous-release parent `7e878a411897a769ac7f85dfd96c15029a23e3d7`, and the merge's
`Built-by: astra` trailer. [Raw checks, workflows and promotion receipt](evidence/release-checkpoint-51.json).

The receipt preserves the pre-promotion PR snapshot as `OPEN`, followed by the
actual release merge identity and promotion timestamp. Its required contexts are
all completed `SUCCESS`; all eight workflows completed successfully on the exact
candidate. The website deployment job was skipped outside the required contexts;
its build and workflow passed. No requirement was removed or weakened.

This baseline adds reviewed PR377–382 evidence, including the ordinary authored
Wasm cold-restart export and native material/hierarchy workflows, plus default-off
wake/acquisition observation. The prior [checkpoint50](release-checkpoint-50.md)
and all retained failures remain historical. Later main384+ work, including
explicit Wasm persistence, its ordinary authored save/load proof, later live
pacing evidence and reconstruction cost results, remains outside this tested
release tree with separate local qualification.

No distribution was published and no phase gate was approved. Complete rendered
game quality/performance, the script-time requirement and other open plan
obligations remain separate.
