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

The local follow-up adds `--four-proofs` to include the complete scene-audio and
native-game `--gpu` commands, on the frozen PR80 source graph. Preparation follows
the current macOS CI order: runtime, scene audio, GPU game, script/physics. Each
generator prepares its own fixtures, compiles its own player, audits its resolved
target units and packages its own matching symbols. Only then does the shared
session remove all four authoring directories and the whole Cargo target before
any packaged player is launched. Standalone command defaults and all existing
execution/refusal assertions remain. The GPU graph's same-name dependency
versions are joined by exact rustc artifact suffix as well as package, target,
features, profile and file hashes; ambiguous joins still fail.

The extension passes 107 tool tests. In its first exclusive local interval, all
four standalone proofs passed in 529.76s. The shared command then failed before
any Cargo build or player launch because dynamic loading did not supply the GPU
probe's sibling-module search path. The loader correction temporarily supplies
that path and restores it afterward; an isolated-process regression imports all
three actual probe modules. [Initial evidence](evidence/shipping-four-proof-initial-2026-10-11.json)
retains the failure without claiming a speedup. Both chains were then repeated on
the same corrected frozen revision in a newly coordinated interval. Public
authoring-tool preparation stayed outside that measurement. The four-proof extension is not enabled in CI, has no measured speed
claim for hosted CI or devices and does not use a persistent shipping target cache.

The corrected frozen revision `023c8bb` completed both four-proof chains in one
exclusive agent CPU/GPU interval: standalone **526.43s**, then shared **437.96s**,
a local saving of **88.47s (16.80%)**. Exact resolved target graphs, compiler and
code-generation flags, numerical results, negative-control identities, logs and
observations match. The shared ledger records 101 fresh runtime units; audio
reuses 82 of 113 target units, GPU game 85 of 168, and script/physics 98 of 113.
Each player is freshly compiled and packaged independently; all build/authoring
storage is deleted before the first packaged-player execution.
[Matched evidence](evidence/shipping-four-proof-matched-2026-10-11.json) retains
the report hashes, source/tool identities, timings and comparison scope.

This is one local macOS sample, measured standalone then shared on the PR80
graph; it is not randomized, a persistent-cache result, a Windows projection or
a phone-performance gate. Root independently verified all eight report hashes.
The current flipbook/SDK controls must be integrated and reviewed before any CI
adoption. The older two-proof numbers below apply only to their recorded source
revision.

The local renderer/import union `ef2897af60d74efdd193288108206d4858ef75ae`
is now integrated into a separate experiment branch. `--four-proofs --flipbook`
forwards the existing animated/static-frame numerical controls without changing
their fixtures or assertions. The original four-proof mode remains available;
`--flipbook` without all four proofs is refused before output creation. All 110
Python tooling tests pass, including option forwarding and global barrier order.
The immutable SDK revision `2ee26d510a6bfa6ea2bcb381b9f3bd0296d9680f`
is now integrated locally with exact PR81. `--flipbook-sdk` requires `--flipbook`
and retains all original QuickJS/static-module frame, state, symbol, C-codegen,
compiler-exclusion and refusal controls. The generated Wasm/C bundle must live
inside registered disposable inputs; its complete file set and hashes are checked
before and after its fresh target build. The global deletion barrier removes that
bundle, guest source, module and translator before any packaged player launch.
The trusted host cooker remains a separate authoring tool, outside the target
shipping ledger. The suite forwards no new default behavior to standalone probes.
This integrated graph still needs its fresh player proof. The 16.8% sample above
does not measure this newer graph; shared storage remains off CI. The inherited
SDK CI command continues to run its full standalone proof.

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
