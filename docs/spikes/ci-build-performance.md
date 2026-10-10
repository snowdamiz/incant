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
Windows npm archives used 8.91 GiB despite an observed 13-second install. No
caches were deleted or repository settings changed in this work.

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
