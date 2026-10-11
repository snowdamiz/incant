# Shared fresh shipping session

The macOS workflow now selects one initially absent temporary Cargo target for
all four existing shipping proofs, including the complete flipbook/SDK controls.
Each player still compiles freshly and preserves its own symbol package and
refusal checks. The hosted result for this integration remains pending.
Standalone commands and Linux/Windows shipping jobs remain unchanged.

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
tick boundaries. Its default two-proof mode leaves audio/game proofs separate;
the CI-selected four-proof mode includes their unchanged verification bodies.
The GPU player is explicitly compiled with its optional feature and exercised
after source deletion; default workspace checks alone cannot certify it.

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
authoring-tool preparation stayed outside that measurement. At that checkpoint the four-proof extension was not enabled in CI. It has no measured speed
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
That checkpoint required current flipbook/SDK integration before CI adoption;
the final integrated proof below supplies that correctness evidence. The older two-proof numbers below apply only to their recorded source
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
The corrected integrated graph passes its fresh four-player proof below. The 16.8% sample above
does not measure this newer graph. At that checkpoint shared storage remained
off CI and the inherited SDK CI command ran its full standalone proof.

The first fresh SDK preparation correctly refused a repeated QuickJS target unit
before any player launch. Cargo identified a change in `CC_ENABLE_DEBUG_OUTPUT`:
the static SDK caller needs this C-command diagnostic, while the following script
caller omitted it. The shared build wrapper now enables this diagnostic uniformly
on a copy of each caller environment and records it. Compiler arguments, the
standalone commands, the strict duplicate-unit refusal and the deletion barrier
are unchanged. All 120 tooling tests pass, including diagnostic consistency and
caller isolation. [Failed-run evidence](evidence/shipping-sdk-ledger-refusal.json)
retains all raw build hashes. The corrected frozen `ef780b9` then passed all four
complete probe bodies, including 32 baseline game repeats, 32 additional
QuickJS/wasm2c SDK repeats, 43 runtime refusals, spatial/observation/log controls
and four animated/static comparisons. Every player compiled freshly and retained
its independent symbol identity; all registered authoring inputs and shared Cargo
storage were deleted before any launch. Audio reused 82 target units, GPU game
85 and script/physics 98; the strict ledger records 231 distinct freshly compiled
target units. The three QuickJS libraries were proven fresh in the game build
and unchanged/reused in the later script build.
[Current SDK proof](evidence/shipping-sdk-shared-proof.json) records exact source,
commands, profile/feature evidence, tool/player identities and all report hashes.
Elapsed time includes a coordinated wait and unrelated CPU work, so no speedup
is inferred. That checkpoint left the generated public Rust SDK and color
correction for the final integration below.

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
results, not a demonstrated material hosted improvement. Those two-proof
measurements alone did not justify CI adoption.
All55 tools tests pass, including wrong-source/revision/compiler, ambient override,
ambiguous/missing unit, changed feature/bytes and deletion-barrier refusal cases.
Standalone runtime and script/physics commands passed again after the refactor.
[Exact final evidence](evidence/shipping-fresh-session-2026-10-11.json).

## Final SDK/color integration and macOS workflow

Frozen `61d41be7566d6139d11b2e0f5686529f71022297` preserves the generated
public Rust SDK, accepted Claude0043 color correction and corrected native-agent
PR82 ancestry. It passes 739 workspace ordinary tests/doctests, all 75 serialized
ignored/GPU cases, workspace Clippy, 536 UI tests/build and 125 Python tool tests.
The same frozen source then passes the complete fresh four-player proof: 32
baseline game repeats, 32 SDK repeats, all 43 runtime refusals, four spatial
refusals, four animated/static comparisons and retained observation/log controls.
The game player is 9,145,040 bytes; its matching symbols, static C flags, generated
SDK provenance and compiler exclusion are recorded independently.

The ledger proves 232 distinct freshly compiled target units. Runtime starts with
101 fresh units; audio adds 31 and reuses 82, game adds 85 and reuses 85, and script
adds 15 and reuses 98. Each player is fresh. All shared build storage and all four
authoring input directories are removed before any packaged player is launched.
[Final proof](evidence/shipping-sdk-color-shared-proof.json) retains exact source,
raw ledger/build/report hashes, compiler/profile and player identities. Its
460.52-second elapsed time includes other CPU activity and no matched standalone
run, so it establishes no new speedup.

The reviewed workflow replaces the four macOS invocations with
`shipping_suite.py --four-proofs --flipbook --flipbook-sdk`. Original artifact
names point to the corresponding per-player outputs. A separate diagnostic
artifact retains the ledger and raw compiler logs even when a proof fails.
Classifier failure still selects the full job, and all other macOS commands,
required check identities, Linux/Windows shipping and device gates are unchanged.
The actual hosted CLI is exercised through the suite parser/phase dispatch in a
regression test. Hosted correctness and latency remain to be measured.
