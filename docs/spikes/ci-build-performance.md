# CI build performance

Measured on 2026-10-10. This changes build reuse, not runtime performance or
phase-gate evidence. Hosted results for the revised workflows remain pending.

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
Fork PRs only restore. Internal PR saves remain in their GitHub PR merge-ref
scope; they cannot populate main or another PR. No privileged PR trigger or
additional token permission is introduced. Main builds seed a trusted cache.
The first Windows/Linux desktop PR remains cold until a successful default-branch
desktop run seeds that workflow; same-PR updates can reuse its cache. After this
change lands, one manual dispatch of the existing desktop workflow on main can
seed both Windows/Linux caches without adding permanent push or scheduled jobs.
No such dispatch was performed during this investigation.

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
- Hosted cold/warm validation remains required. Record queue versus execution,
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
cold/warm timings remain unverified.

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
New hosted cold/warm measurements remain pending.

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
