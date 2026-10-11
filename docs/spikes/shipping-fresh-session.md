# Shared fresh shipping session

This opt-in macOS experiment shares one initially absent temporary Cargo target
between the existing standalone Transform/Velocity player and embedded script
player with its physics cases. It does not change any CI command. Controlled
comparison and independent review remain required before CI adoption.

`python3 tools/shipping_suite.py <new-output-directory>` prepares and packages
both players, removes all fixture authoring inputs and the complete shared build
storage, then executes every existing assertion. The original `shipping.py` and
`probes/cooked-script.py --physics` commands still use independent fresh storage.
No persistent target cache, new runner or cache quota is introduced.

The ledger records actual target rustc commands alongside Cargo package, target,
features, profile and artifact filenames/hashes. A library reported fresh by a
later Cargo invocation is accepted only when its exact artifact identity and
bytes match a freshly compiled unit from this same session. Each player must
compile freshly. Host compiler/build-script artifacts are not target-runtime
proof. The existing per-player codegen and compiler-symbol checks remain.

Tracked source revision/hashes and pinned compiler identity are checked around builds and
before execution. Ambient compiler/flag/wrapper overrides are refused. Explicit
script compile inputs are hashed around their build.
Preexisting storage, missing or ambiguous evidence, changed features/bytes,
retained authoring storage and changed packaged players fail before execution.

The suite preserves runtime abort/no-unwind and corrupted-scene checks, each
symbol package/archive, all script structural/input cases and four native physics
tick boundaries. It does not replace native-scene-audio or native-game proofs.
Optional `incant_game_player/render-gpu` needs separate feature compilation and
the actual source-deleted GPU proof; default workspace checks cannot certify it.

The original commands passed locally on the same runtime sources in 78.82 and
121.71 seconds, respectively. This is a local baseline with concurrent unrelated
development builds, not a hosted or device performance result. Shared-session
measurements are now complete: the final standalone commands took80.59s and
120.84s (201.43s total); the shared command took189.25s including its outer
process. All27 observable/guard comparisons and38 player launches pass. The first
build recorded93 fresh target units; the second recorded107 units,77 proven reused
dependencies. Owned libraries compiled again as distinct dependency/feature units.
Each player compiled freshly and independently passed its symbol identity/audit.

The roughly12s difference in this sample and8s in an earlier sample are noisy local
results, not a demonstrated material hosted improvement. CI adoption stays off.
All55 tools tests pass, including wrong-source/revision/compiler, ambient override,
ambiguous/missing unit, changed feature/bytes and deletion-barrier refusal cases.
Standalone runtime and script/physics commands passed again after the refactor.
[Exact final evidence](evidence/shipping-fresh-session-2026-10-11.json).
