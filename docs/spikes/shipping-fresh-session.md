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

Tracked source hashes and pinned compiler identity are checked around builds and
before execution. Explicit script compile inputs are hashed around their build.
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
measurements and matched observable-output comparison are pending.
