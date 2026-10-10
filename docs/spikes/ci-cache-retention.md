# Shared Rust cache retention

Measured 2026-10-10 after all PR #48 jobs completed, before cleanup. This policy
retains all checks and has no measured warm-hosted speedup yet.

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
