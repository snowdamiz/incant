# CI build performance

Measured on 2026-10-10. This changes build reuse, not runtime performance or
phase-gate evidence. The first cold hosted results and a subsequent local
feature-graph refinement are recorded below. The completed warm PR #51
measurement is recorded at the end; it is an observed comparison with an inherited
dependency/feature delta, not a controlled unchanged-source benchmark.

## Baseline

Twelve successful PR `Incant checks` runs in the latest hundred Actions runs
had a median elapsed time of **38m15s** (35m24s–45m03s).
[PR #44](https://github.com/snowdamiz/incant/actions/runs/38075442645),
head `53a6d9e0`, spent nine seconds queued and 37m04s executing:

| Operation | Step time | Compilation reported by Cargo |
| --- | ---: | ---: |
| Workspace Clippy | 3m17s | 3m16s |
| Audio device feature check | 39s | 38s |
| Workspace release tests | 11m47s | 11m05s |
| Ignored GPU tests | 5m54s | 2m31s + 2m38s |
| Khronos KTX validation | 1m37s | 1m34s |
| Editor custom-protocol build | 2m34s | 2m33s |
| Standalone headless/schema command | 5m44s | 5m43s |
| Collaboration example | 2m34s | 2m34s |

Actual workspace test execution took about 41 seconds; GPU test execution about
45 seconds. The package-scoped GPU commands compiled 203 crates after the
workspace build. The separate headless command compiled 173. Different package
selections change Cargo's unified dependency features and cause real rebuilds.

The same-revision
[Windows job](https://github.com/snowdamiz/incant/actions/runs/38075442655)
finished in 25m43s. A slower
[Windows run](https://github.com/snowdamiz/incant/actions/runs/38066379671)
spent 41m31s executing, including 20m08s initial compilation and another
5m12s + 5m51s compiling the scoped GPU graphs.

Queue delay is separate. The
[structural-buffer macOS smoke](https://github.com/snowdamiz/incant/actions/runs/38075667061)
waited 23m58s for 3m21s of work; a
[checks run](https://github.com/snowdamiz/incant/actions/runs/38058862334)
waited 12m35s before 32m27s execution. More concurrent macOS jobs can worsen this.

## Changes

- Run ignored tests with the same workspace/release/locked selection as the
  normal test pass. Every current ignored test is a GPU test. Test processes
  and device lifetimes remain serialized with `--test-threads=1`; shader,
  renderer, editor and headless cases remain included. Future ignored tests
  will also execute instead of silently being omitted.
- Build editor and headless together on macOS, as the desktop matrix already
  does. Execute the built headless binary for schemas and the evaluation
  fixture check so package selection cannot cause another build.
- Add pinned Rust dependency caches to the main, desktop, credential and
  platform workflows. Keys separate workflow/job, OS/architecture, compiler,
  compiler environment and dependency manifests. Cargo validates fingerprints
  and all checks still run on hits and misses. Workspace artifacts and
  incremental builds are excluded by the cache action's default cleanup.
- Stop creating a Windows npm cache for each PR. Keep `npm ci` unchanged.

The cache API reported 47 entries using 10.03 GiB, all npm/Gradle. Thirty-four
Windows npm archives used 8.91 GiB despite an observed 13-second install. A later
authorized cleanup of exact merged-PR entries is recorded below; repository
settings remain unchanged.

The [pinned Rust cache action](https://github.com/Swatinem/rust-cache/tree/6323deb102c322ba6fcbdcafc7e3dddab59af2b6)
uses only Cargo registry/git and target paths with bin caching disabled; it does
not include Cargo credentials, project/auth artifacts or evaluation outputs.
The initial rollout allowed internal PR saves in their isolated merge-ref scope;
those could not populate main or another PR. The measured retention refinement
below now makes every PR restore-only, with main producers publishing the shared
set. No privileged PR trigger or additional token permission is introduced.
Filtered desktop/credential main pushes automatically refill dependency changes;
existing main checks and nightly platform runs supply the other jobs. A first PR
remains cold until its successful main producer publishes. No dispatch has yet
been performed during this investigation.

## Verification and remaining work

- Four edited workflows pass actionlint v1.7.12; the downloaded release binary
  was verified against its published SHA-256. `git diff --check` passes.
- On base `0bb42df`, a cold local release workspace run passed all 318 Rust
  tests in 488.54 seconds. The immediately following workspace ignored pass
  passed all **44 exact named GPU tests** from the hosted baseline in 23.55
  seconds. Cargo finished in 2.90 seconds and rebuilt only `incant_editor`;
  no external dependency was rebuilt. This verifies test preservation and
  graph reuse on the local Mac, not a matched hosted speedup.
- All 387 UI tests, 15 tool tests and generated conventions/bridge/SDK checks
  pass. On integrated main `c1add7c`, the combined custom-protocol editor and
  headless build passed in 130.86 seconds. Direct schema generation, eval-check
  and all 15 existing gameplay probes passed; schemas remain unchanged. The
  final workspace pass on structural main `23637ed` passed all 339 tests. Its
  following ignored pass again matched all 44 baseline GPU names in 22.66
  seconds, with **zero compile events** and Cargo finishing in 0.49 seconds.
  Formatting, the real runtime dependency guard and workflow lint pass.
- Initial hosted validation was pending. Record queue versus execution,
  cache hit/restore/save size, compilation versus test time and test counts.
  Component timings suggest roughly five minutes of avoidable macOS GPU
  compilation and eleven minutes on the slower Windows sample. These are
  baseline costs, not a measured post-change improvement.

Disabling the Windows npm cache can increase package downloads; measure that
step separately from Rust cache gains. No cache hit is allowed to skip a test.

No job names, triggers, cancellation policies, required coverage, build profiles
or artifact license notices are removed. There was no feature-branch push/PR
double build to remove: pushes only trigger full checks on main. Retain that
integration verification. Cache capacity/eviction may limit reuse; measure it
before adding more cached outputs. The first rollout is cold; repeat successful
PRs can reuse their own cache, while shared main reuse requires a successful
main job. Rapid merges can cancel main before its success-only cache save. Do
not count cancelled runs as cache producers. Batch tightly related work in a
reviewable PR when sensible to reduce repeated full matrices for dependent
stacks, and avoid adding more independent macOS jobs while capacity is scarce.

Follow-ups after hosted measurement: build texture/collaboration examples with
a consistent graph, cache the pinned platform CLI tools, and move lightweight
platform-independent work off the scarce macOS runners while retaining explicit
aggregate failure propagation. Those examples alone recompile for about four
minutes in the baseline. No new scheduled workflow is added here.

PLAN.md's dependency-license allowlisting and reference-phone performance gates
remain unmet in the inspected workflows; this optimization neither completes
nor waives them. Simulator/desktop test passes are not reference-device results.

## Integration with Windows doctest repair

The reviewed build-reuse change is included in the numeric-column PR together
with its Windows CRT repair, so a single hosted matrix validates both rather
than launching another duplicate full matrix. Shipping-profile builds, source-free
execution, symbol validation and uploaded artifacts from PR #47 remain intact.
The static CRT policy and PE import checks are documented in
[Windows CRT evidence](windows-crt-policy.md). Its local 352 Rust and 25 tool
tests passed before this workflow-only integration; hosted Windows behavior and
cold/warm timings were still unverified at that integration; the cold results
are recorded below.

## Combine the required authoring tools and examples

The next workflow change builds macOS's `incant_editor`, `incant_headless`,
`cook_scene` and `collaboration_spike` in one Cargo invocation, with explicit
binary/example selectors and `incant_editor/custom-protocol`. Schema generation,
collaboration and evaluation execute the resulting binaries directly. Desktop
combines only its editor, headless and cook targets; it does not build an unused
collaboration example. The separate cook build is removed on both workflows.
Cargo metadata confirms each target name. Shipping still uses its own fresh
target directory and independently verifies codegen, execution and symbols.

This avoids the remaining standalone cook/collaboration feature-graph changes.
It does not combine the Khronos texture example yet. The normal workspace tests,
all ignored GPU tests, every gameplay/render probe, shipping execution and
artifact uploads remain. A source comparison confirms the existing
test/probe/dependency/shipping commands in each workflow are unchanged.

Local macOS validation at base `588b3ab`, using `./tools/cargo -j2` and existing
dependency outputs, produced:

| Operation | Wall time | Result |
| --- | ---: | --- |
| Workspace release tests | 176.00s | 352 pass; 16 workspace compile events |
| Serialized ignored workspace tests | 19.50s | Exact 44 baseline GPU names; zero compile events; Cargo 0.22s |
| Combined four-target build | 56.75s | 14 workspace compile events; zero external dependency rebuilds |
| Desktop three-target selection immediately afterward | 0.26s | Zero compile events; Cargo 0.21s |
| Schema, evaluation, collaboration and 15 gameplay probes | 20.79s | All 18 commands pass; schema files unchanged |
| Fresh shipping package and source-free execution | 30.46s | Mach-O/dSYM match, abort and source-free checks pass |

The shipping Mach-O/dSYM UUID is `23815991-967c-395d-afb3-5628752edc2d`.
All 25 Python tool tests, generated conventions, runtime dependency guard,
actionlint v1.7.12 and `git diff --check` pass. These are local reuse/correctness
measurements, not a matched hosted speedup or reference-device performance gate.

Desktop's deadline increases from 45 to 60 minutes as bounded cold-run
resilience, without a speedup claim. The old
[PR #47 Windows job](https://github.com/snowdamiz/incant/actions/runs/38076760529)
timed out at 45m06s during saved steering replay, after shipping and its artifact
uploads had passed. It spent 21m55s on normal workspace tests, 13m49s on the old
GPU calls (11m30s compilation), 1m32s on editor/headless, 1m30s on cook_scene and
1m42s on shipping. Keeping shipping provenance intact is inexpensive relative
to repeated development-graph compilation. Its macOS check spent 40m57s executing;
the standalone cook and collaboration steps took 54s and 2m05s respectively.
The first cold hosted results are recorded below; warm measurements remain open.

## Authorized cache cleanup

On 2026-10-10 at 19:30 UTC, exact IDs from the reviewed Windows npm inventory
were rechecked against current keys, PR merge refs and freshly fetched PR states.
All 29 remaining candidates belonged to merged PRs and had keys matching
`node-cache-Windows-x64-npm-[0-9a-f]{64}`; five original IDs were already absent.
After explicit authorization, the 29 exact DELETE requests succeeded between
19:32:10 and 19:32:22 UTC, removing **8,162,117,941 bytes (7.60 GiB)**.
The [complete per-ID evidence](ci-cache-cleanup-2026-10-10.json) records keys,
refs, PR state, individual sizes, request outcomes and before/after verification.

Immediate re-listing confirmed every deleted ID absent and every nonselected
preexisting ID still present. Open-PR/default-branch/Rust/Gradle caches were
preserved. Inventory moved from 48 entries / 11,633,972,224 bytes to 23 entries /
5,719,550,745 bytes while four new Rust caches arrived concurrently. The aggregate
usage endpoint lagged at 45 entries / 10,494,390,193 bytes both times, so its delta
is not treated as deletion evidence. The 5.33 GiB inventory leaves about 4.67 GiB
of initial headroom under the 10 GiB cap; future cache publication can still
cause eviction. No broad cache patterns, settings, dispatches or cancellations
were used.

## Preserve the workspace graph when selecting tools

The first cold [PR #48 macOS check](https://github.com/snowdamiz/incant/actions/runs/38079748085)
passed in **33m58s execution**, with 2m30s queued. Normal release tests took
13m04s, the exact 44 baseline GPU tests 1m35s, and shipping 2m09s. The preceding
PR #47 check executed for 40m57s. These are adjacent revisions on separate hosted
runners, not a controlled speedup measurement. Rust cache restore missed; the
successful job published a 1,541,663,384-byte dependency archive in 41 seconds.

The same log shows remaining shifted compilation: editor/headless took 4m28s
with 69 compile events, cook_scene 1m33s with 63, and collaboration 2m17s with 87.
The GPU call rebuilt only the editor (37.64s), so its reduction cannot be counted
as an equal reduction in total time. Khronos validation still took 2m49s and
compiled 143 crates; that separate graph remains a possible later improvement.

A same-head local experiment at `0b501e9` compared the combined package selection
with `--workspace`, keeping every explicit binary/example selector and the
required `incant_editor/custom-protocol` feature. Cargo's JSON artifact messages
were compared against `cargo test --workspace --release --locked --no-run` by
exact output filenames, retaining multiple host/target variants of a dependency.

| Selection | External library/proc-macro artifacts | Exact test-artifact matches | Variants absent from test graph |
| --- | ---: | ---: | ---: |
| Four selected packages | 656 | 611 | 45 |
| Workspace, same four targets | 658 | 656 | 2 |

The two necessary differences are `tauri` and `tauri_macros` with
`custom-protocol`. Package selection additionally changes cryptography, Loro,
Rapier, image and Tauri dependency fingerprints. Workspace selection emits only
the four requested executables: editor, headless, cook_scene and collaboration.
Desktop's three explicit targets likewise produce only its requested tools.
It does not build every workspace binary/example or execute fewer tests.

In the existing local target directory, the first workspace-selected build took
24.46 seconds and rebuilt only Tauri and the editor. The package-selected graph
was already warm (0.37s); these wall times do not establish a cold speedup.
The subsequent test no-run reference completed in 0.31s without rebuilding;
desktop's workspace-selected three-target build took 0.24s with no rebuild.
The two workflow build commands now select the workspace to retain this reuse.

After the refinement, all 18 direct schema/evaluation/collaboration/gameplay
commands passed in 20.32s, with schemas unchanged. Fresh shipping verification
passed in 29.37s, including source-free execution, abort behavior and matched
symbols. The preceding unchanged-source suite passed 352 normal and all 44 exact
GPU tests; the artifact comparison confirms their graph remains reusable.
Actionlint v1.7.12 and `git diff --check` pass. Hosted Windows/Linux/macOS results
for this final selector refinement remain to be measured. Shipping still has an
independent fresh target directory and unchanged provenance checks.

## Completed first cold hosted matrix

At checked head `588b3ab`, all 17 PR #48 checks passed. The coordinator verified
merge `56c22d4` at 20:06:24 UTC with the identical checked tree. The later combined
explicit-target/workspace-selector refinements above were not in this run.

| Job | Queue | Execution | Normal tests | GPU tests | Tool build | Separate cook | Shipping | Cache save |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| macOS checks | 2m30s | 33m58s | 13m04s | 1m35s | 4m28s | 1m33s | 2m09s | 41s |
| Linux desktop | 2s | 17m56s | 9m09s | 1m39s | 1m42s | 54s | 45s | 21s |
| Windows desktop | 2s | 40m21s | 20m48s | 2m18s | 6m21s | 2m15s | 1m59s | 1m18s |

[Desktop run](https://github.com/snowdamiz/incant/actions/runs/38079748032)
and [macOS run](https://github.com/snowdamiz/incant/actions/runs/38079748085)
all missed their Rust cache. All three passed the exact 44 named baseline GPU
tests; normal workspace test totals were 352, 350 and 347 respectively, reflecting
platform coverage. Windows's GPU call compiled nothing (Cargo 0.88s); Linux and
macOS each rebuilt only the editor. Windows still compiled 72 crates in the tool
build and 88 in the cook call. Its uncached npm install took 18 seconds.

The preceding PR #47 jobs took 40m57s on macOS, 24m16s on Linux and timed out at
45m06s on Windows during saved steering replay after shipping passed. These
uncontrolled adjacent-revision results do not establish a matched speedup;
especially, removing GPU rebuilds shifted some compilation to subsequent tools.
The workspace-selector experiment addresses that remaining graph mismatch.
Warm hosted verification remains open.

## Reuse the texture fixture generator

The same macOS cold log's 2m49s Khronos validation step compiled 143 crates because
`validate_ktx.py` ran the asset package's `texture_probe` through a separate
package-selected Cargo graph. The shared workspace build now includes that exact
example as its fifth target, and validation runs its executable directly through
`--probe-binary`. The existing standalone build-and-run behavior remains available
when that argument is omitted. The pinned Khronos 4.4.2 package/checksum and all
color, linear and HDR validation checks are unchanged; validation follows the
combined tool build instead of preceding it. Desktop workflows remain unchanged.

On the existing local target directory, the five-target build emitted exactly
`incant_editor`, `incant_headless`, `cook_scene`, `collaboration_spike` and
`texture_probe`, with every artifact fresh (Cargo reported 0.27 seconds).
Comparing exact external library/proc-macro filenames to a workspace release test
no-run yielded 658 artifacts, of which 656 match; only the required Tauri and
Tauri macros custom-protocol variants differ. Thus the fixture example introduces
no additional external-library graph beyond the previously reviewed tools.
These are artifact-reuse observations, not a cold-hosted speedup measurement.

The real prebuilt probe produced all three fixtures and pinned Khronos 4.4.2
accepted each with no stdout/stderr diagnostics. An explicit missing probe path
fails before validation and never falls back to Cargo. Actionlint and diff checks
pass. Logs are `/tmp/incant-ci-ktx-build-artifacts.jsonl`,
`/tmp/incant-ci-ktx-test-artifacts.jsonl` and
`/tmp/incant-ci-ktx-validation.log`; the validation report is in ignored
`artifacts/ci-ktx-reuse/validation.json`. No runtime source or dependency changed,
and no test, shipping check, validator or format was dropped.

Final cache sizes and the implemented shared-cache producer policy are recorded
in [cache retention](ci-cache-retention.md). After preserving those measurements
and independently rechecking PR #48's merged state and every candidate's key/ref,
12 exact private Rust cache IDs were removed at 20:07 UTC to make room for shared
main producers. This freed 8,581,391,005 bytes; all 15 other entries were preserved.
[Per-ID cleanup evidence](ci-pr48-cache-cleanup-2026-10-10.json) records the verified
merge, sizes and before/after IDs. Open PRs, default-branch caches, quota and
billing settings were unchanged.


## Completed warm PR #51

[PR #51](https://github.com/snowdamiz/incant/pull/51), pinned head
`22a81fc92b07eec0ac6a5285a72d3b7e3a8889a5`, passed all nine applicable checks and
merged as `a7a6faf3022aebf6c9967c6b62b6d5840ffc9ae4`. Actual-main, local merge-tree
and GitHub merge-ref trees matched the tested head exactly; parents and the
`Built-by: astra` trailer were verified. The existing credentials/physics path
filters excluded unchanged source scopes; no workflow or check was disabled.

All nine jobs restored a shared main49 prefix cache and saved no PR cache. The
inherited main50 Rayon/crossbeam dependency/feature delta means this is not an
unchanged-dependency experiment. The [complete machine evidence](evidence/shared-cache-warm-pr51-2026-10-10.json)
records actual requested/restored keys, bytes, queue, steps, compile events and
all 44 exact GPU names per native job.

| Check | Cold PR49 execution | Warm PR51 execution | PR51 queue |
| --- | ---: | ---: | ---: |
| Windows engine | 34m21s | **21m19s** | **1s** |
| Linux engine | 18m53s | 12m54s | 1s |
| macOS general | 20m42s | 12m14s | 5s |
| Windows platform | 4m56s | 3m34s | 2s |
| Linux platform | 2m27s | 1m51s | 2s |
| macOS platform | 3m42s | 1m46s | 6s |
| Web | 4m48s | 2m52s | 2s |
| Android | 7m40s | 5m49s | 1s |
| iOS simulator | 5m57s | 9m24s | 9s |

The longest check improved by **13m02s (38%)**. Simulator execution alone took
383s in the iOS job, so a cache hit did not make every job faster. The three
native jobs passed 377 Windows, 380 Linux and 382 macOS ordinary tests (including
nine new native saves each), plus all 44 exact baseline GPU test names each.
Platform-specific test totals differ intentionally.

### Remaining build and execution cost

Times below are observed wall time, not CPU attribution. Cargo's reported build
phase includes its compilation/linking work; ordinary logs cannot separate LLVM
from linking or quantify overlapping units.

| Operation | Windows | Linux | macOS |
| --- | ---: | ---: | ---: |
| Ordinary test step | 681s | 338s | 378s |
| Cargo build phase within it | 596s | 274s | 328s |
| Approximate remaining ordinary-test execution | 85s | 64s | 50s |
| GPU step | 147s | 119s | 53s |
| Rebuild within GPU step | none | editor 30.53s | editor 5.51s |
| Explicit custom-protocol tool build step | 88s | 38s | 41s |
| Fresh shipping profile/symbol/process proof | 99s | 51s | 65s |
| Shared cache restoration | 34s | 19s | 30s |

Windows still compiled 20 workspace packages and 22 other packages in the
ordinary pass, versus 569 other compile events on cold PR49. The 22 include
vendored `rerecast`, four packages newly added by main50 (`rayon`, `rayon-core`,
`crossbeam-deque`, `crossbeam-epoch`) and 17 existing registry packages in the
changed dependency graph. The action intentionally removes workspace artifacts;
source-only cache reuse must not be assumed. Do not enable workspace artifact
caching blindly: timestamp invalidation and archive size can erase its benefit.
The exact invalidation reason for each existing unit and compiler/linker split
need Cargo timing/fingerprint evidence from a compatible-cache follow-up.

The tool pass compiled the editor and one Tauri variant; GPU preserved the same
workspace feature graph and rebuilt no external dependency. The fresh shipping
proof uses a different profile and verifies actual compiler flags, symbol
identity, panic behavior and source-free execution. Its 99s/51s/65s costs are a
required measured floor for this proof, not a redundant ordinary-release build.

The reviewed [pinned CLI cache](ci-pinned-platform-tools.md) removes a separate
79s web/30s Android installation opportunity while retaining normal fallback.
It does not shorten this Windows path by itself. The next native investigation
should capture Cargo per-unit timings before changing test linkage/layout or
compiler configuration. The [path-selection proposal](ci-path-selection-proposal.md)
keeps runtime coverage and existing check names intact, beginning with strict
prose-only PRs; no coverage reduction has been implemented.
