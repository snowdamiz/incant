# Shared Rust cache retention

The capacity baseline was measured 2026-10-10 after PR #48, before cleanup.
The completed main #49 producer rollout is recorded below. All checks remain;
a complete independent warm-PR measurement is still pending.

## Capacity and visibility

| Rust cache family | Entries | Bytes | GiB |
| --- | ---: | ---: | ---: |
| macOS full checks | 1 | 1,541,663,384 | 1.436 |
| Linux/Windows full desktop | 2 | 2,615,240,356 | 2.436 |
| Six platform probes | 6 | 2,966,864,872 | 2.763 |
| Credential persistence | 3 | 1,457,622,393 | 1.357 |
| All Rust dependencies | 12 | 8,581,391,005 | 7.992 |
| Existing main npm/Gradle | 13 | 1,204,127,398 | 1.121 |
| One shared set plus main non-Rust | 25 | 9,785,518,403 | 9.114 |

One shared generation fits the default 10 GiB allowance with only **0.886 GiB**
headroom. A second full PR copy cannot fit; even leaving only the nine auxiliary
jobs saving per PR would add 4.12 GiB per PR. Growth and overlapping dependency
versions can still evict a useful entry. Re-measure after new dependencies and
consider registry-only caches for auxiliary jobs if necessary; do not silently
increase billing or omit checks. The full pre-cleanup inventory was 27 entries /
10,348,422,742 bytes, including two older private Windows npm caches.

GitHub's [cache access rules](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#restrictions-for-accessing-a-cache)
make a PR merge-ref cache reusable only by that PR; it cannot seed main or another
PR. PRs can restore default-branch caches. At the storage limit, least-recently
accessed caches are evicted. Consequently, preserving PR archives alongside the
main set wastes scarce cross-PR capacity after the PR has merged.

## Implemented policy and producers

Every Rust cache step now uses:

```yaml
save-if: ${{ github.ref == format('refs/heads/{0}', github.event.repository.default_branch) }}
```

The [pinned action's documented semantics](https://github.com/Swatinem/rust-cache/blob/6323deb102c322ba6fcbdcafc7e3dddab59af2b6/README.md)
make false restore-only. All PRs, including internal PRs, restore without saving;
successful default-branch jobs can save. Keys, action pin, dependency-only cleanup
and disabled bin caching are unchanged. Cache contents have no signing keys or
provider credentials. There is no privileged PR trigger or new token permission.
Existing same-PR entries remain restorable until expiry/deletion; source-only
repeat PRs can restore main. Unmerged dependency changes may rebuild a missing
delta until a successful main producer publishes it. No full private fallback
copy is created.

- Existing main-push checks replenish the macOS full-check set.
- Existing nightly default-branch platform runs replenish all six platform sets.
- Desktop and credential workflows add main-push filters covering root and nested
  Cargo manifests/locks, root rust-toolchain files, `.cargo/**` and the workflow's
  own YAML. Nested manifests include vendored/probe crates because the action
  hashes Cargo manifests/locks anywhere in the repository. The policy merge itself
  changes each producer YAML and therefore seeds those matrices automatically.
  Later dependency/toolchain/config/workflow changes refill automatically;
  ordinary source-only merges do not add these matrices.
- Existing PR triggers, schedule, dispatch, job names, concurrency and all test,
  probe, shipping and artifact commands remain unchanged. Default branch `main`
  was verified with the repository API. Main push, main dispatch and scheduled
  runs satisfy the save predicate; PR refs and non-default dispatches do not.

The action also supports restoring prior lockfile versions, so dependency changes
can reuse an earlier main prefix while compiling missing dependencies. Main runs
cancelled by subsequent pushes do not publish a success cache; a successful
producer must finish before treating the set as seeded. Toolchain updates without
a repository commit rely on the next applicable producer (nightly for platform
jobs, next filtered change or explicit existing dispatch for desktop/credentials).

## Rollout and verification

Actionlint v1.7.12 passes for all four workflows. Source comparisons confirm every
existing `run`/`uses` line and PR trigger is unchanged by this policy. No Cargo or
application input changes are part of it. After this policy is on main, verify
actual main cache publications; use a single existing default-branch dispatch
only for missing producer families if needed. Do not dispatch PR refs. A later
PR must demonstrate restore origin/size and reduced compilation before claiming
a warm speedup. Cache saves never gate whether tests execute.

The [authorized PR #48 cleanup](ci-pr48-cache-cleanup-2026-10-10.json) retained final
sizes before deleting its 12 exact merged-PR Rust entries. Fresh API checks pinned
merge `56c22d408fa3e85aaf6bc37e00f856c664dfd0c8`, keys and merge refs; all requests
succeeded and all nonselected entries remained. Inventory changed from 27 /
10,348,422,742 bytes to 15 / 1,767,031,737 bytes. This allows main refill without
using private cache eviction as hidden evidence of a workable steady-state quota.
No default-branch/open-PR cache or cache setting was changed.


## Completed main producer rollout

PR #49 merged as `646095e78268c70a3216e5d146412be5abf6ec83` after all 12
current-head checks passed. Its PR jobs all missed Rust caches and all used
`save-if: false`; no private Rust fallback was published. The main push started
[full checks](https://github.com/snowdamiz/incant/actions/runs/38085082368),
[desktop](https://github.com/snowdamiz/incant/actions/runs/38085082417) and
[credentials](https://github.com/snowdamiz/incant/actions/runs/38085082454).
A fresh inventory found no equivalent main platform run: the prior scheduled run
had different lock/workflow inputs. Exactly one existing
[nightly main dispatch](https://github.com/snowdamiz/incant/actions/runs/38085106886)
filled that family. No producer was duplicated or rerun.

All 12 producer jobs passed and each exact cache key was verified under
`refs/heads/main` through the cache API. Queue is each job's creation-to-start
interval; execution is start-to-completion including artifact/cache publication.

| Producer job | Execution | Queue | Rust restore |
| --- | ---: | ---: | --- |
| macOS full checks | 13m43s | 8s | Prior main prefix |
| Windows full desktop | 34m27s | 2s | Miss |
| Linux full desktop | 15m36s | 2s | Miss |
| macOS credentials | 3m53s | 12s | Miss |
| Linux credentials | 2m44s | 2s | Miss |
| Windows credentials | 5m01s | 2s | Miss |
| Web platform | 5m32s | 3s | Miss |
| Windows platform | 6m14s | 2s | Miss |
| Linux platform | 2m09s | 2s | Miss |
| macOS platform | 4m03s | 7s | Miss |
| iOS simulator | 6m47s | 6s | Miss |
| Android platform | 7m05s | 2s | Miss |

The macOS full check restored 1,541,591,502 bytes from the prior main prefix and
published its new exact key. Its release workspace step took 327s, GPU suite 66s,
Khronos validation 35s and combined tool build 55s. The other 11 producers were
cold. These are observed producer times, with runner variation and overlapping
physics PR #50 work; they are not a controlled cache-only benchmark or the next
PR's warm-consumer result. Every existing test, platform, shipping, CRT and probe
step remained enabled.

Final shared Rust storage is **8,383,182,696 bytes (7.807 GiB)** across 12 entries.
The complete retained inventory is **28 entries / 9,587,788,571 bytes (8.929 GiB)**,
leaving about 1.071 GiB below the default 10 GiB limit at this observation.
Detailed run/job IDs, keys, versions, sizes, timestamps and queue times are in
[the rollout evidence](evidence/shared-cache-seed-2026-10-10.json).

Three exact obsolete entries were removed after fresh verification, reclaiming
2,104,495,841 bytes: private Windows npm caches for merged PRs #45/#46
(`8776796913`, `8776838551`), with Windows npm saving still disabled, and the old
main macOS Rust cache (`8778772672`). The replacement (`8779181342`) had a
successful completed upload, the same restore prefix/version and an intact API
entry. At deletion its independent consumer restore was still unverified; no
extra consumer run was forced. Every nonselected entry remained, and the final
inventory has no unexplained disappearance from the initial snapshot.

This establishes publication and point-in-time retention, not permanent cache
availability. Desktop/credential producers run on filtered changes or dispatch;
a source-only PR after long idle/eviction can remain cold until a producer runs.
The current rollout recovers explicit misses without reducing coverage. Future
warm evidence must name actual restored keys and distinguish prefix reuse from
exact hits, changed dependency/features, queue time and execution. Reference-phone,
mobile execution and phase gates are unaffected.
