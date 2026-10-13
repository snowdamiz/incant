# Complete root motion on main160

The accepted native/TypeScript root-motion path and additive Wasm operations
53–55 now share one current-main union. Existing operations 0–52 remain unchanged.
All 119 reviewed producer source, test, tooling and build inputs are byte exact.
The native suite passes 606 CPU cases; the separate actual Wasmtime/wasm2c suite
passes 453 cases in 80 groups plus all-target Clippy and three development
processes. Target, SDK, dependency, generated-contract and tool checks pass.
These suites overlap and their counts are reported separately.

A fresh ordinary shipping build uses the current lock and captured project.
After deleting private authoring/cache/build/tool inputs and debug maps, three
separate 32-tick player processes match all 22,464 typed-native oracle scalars
and 256 public vector checks per run, exact development logs/observations and
byte-identical PCM. Eleven precise development/package refusal cases preserve
empty stdout and absent PCM/save output. Public import, atomic selection,
Undo/Redo and journal reopening are exercised before deletion.

The path covers transform, extract-only, character and kinematic root policies,
exact clock words, LINEAR/STEP root tracks, compound interpolation and ordered
IK. CUBICSPLINE root import and whole-module checkpoints explicitly refuse.
This is local CPU shipping evidence, not GPU appearance, device performance or
phase approval. The separate frozen checkpoint14 remains the release boundary.

The [full source, raw/compressed command logs and package receipts](evidence/native-root-motion-main160.json)
are accepted by the [independent final review](evidence/native-root-motion-main160-review.json). Exact-head fast CI remains required for publication. Later main159–160 contributes
only documentation and the separately reviewed Windows desktop job budget;
reviewed production/build/proof bytes remain unchanged.
