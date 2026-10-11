# Auxiliary registry caching

The bounded follow-up to the [capacity comparison](ci-cache-layout-proposal.md)
uses the pinned rust-cache action's existing inputs. Three credential jobs and
five platform jobs (three desktop, web, Android) cache registry/git downloads
without compiled targets. Native engine and iOS target caching remains unchanged.
No test, assertion, command, target, feature, timeout, trigger or permission is
removed or changed. Pinned wasm-bindgen/cargo-ndk caches remain independent.

## Exact local verification

The change replaces `key` with `shared-key: auxiliary-registry-v1-<runner label>`
and adds `cache-targets: 'false'` to four input maps, expanding to eight jobs.
`cache-bin: 'false'` and the default-branch-only `save-if` are retained. PRs only
restore. No packaging, publisher, admission service or new workflow is introduced.

An independent parsed-YAML comparison against reviewed `961b1d3` restores only
those input changes and proves complete workflow equality. All 36 Python tool
tests, actionlint, generated conventions and whitespace checks pass. Rust source,
manifests, lockfile, compiler flags and profiles are unchanged; no broad local
Rust rebuild is needed for this inputs-only change.

[Preflight measurement output](evidence/ci-auxiliary-registry-preflight-2026-10-10.json)
records all eight observed keys from completed PR56 jobs and log hashes. Applying
the [pinned action's key construction](https://github.com/Swatinem/rust-cache/blob/6323deb102c322ba6fcbdcafc7e3dddab59af2b6/src/config.ts)
produces three expected new namespaces: macOS/arm64, Windows/x64 and Linux/x64.
Compiler/environment and manifest/lock suffixes are still present. The namespace
contains the runner label; the action adds OS and architecture. Runner labels
can move to newer images, so this does not claim an immutable image pin. These
archives contain checksummed source/download data; compiled native/iOS archives
keep their existing separate keys.

The preflight derives new keys from actual observed inputs; hosted publication
and consumption of the new namespaces are still pending. The first PR59 head's
completed credential jobs now confirm all three exact expected keys, cold misses
and no PR writes: macOS 2m07s plus 1m34s queue, Linux 2m22s plus 1s, Windows 4m18s
plus 2s. These measurements do not prove new archive sizes or warm restores.
Different future environment inputs may produce additional generations. An immutable shared key
can be populated first by a job with only a subset of downloads; normal locked
Cargo downloads remain the fallback. A hit does not imply every crate is present.

The integration with corrected, reviewed PR57 `aecee499` preserves its runtime, physics,
audio, editor, Cargo graph and tools exactly. Parsed-YAML comparison against that
base again finds only the four approved cache input changes; 40 tool tests and
workflow/convention checks pass. The manifest/lock suffix changes for this union,
so current-head hosted checks must complete before merge. PR59 is stacked on
`impl/native-physics-runtime`; no contentless PR56 push is required.
The inherited corrections decode Cargo machine output as UTF-8 on Windows and
canonicalize physics report paths; neither changes runtime code or build flags.

## Capacity and migration limits

The selected old combined archives total 4,252,113,850 bytes, including the already
missing 566,356,126-byte Linux credential family. Retiring currently present
selected entries from that snapshot would free 3,685,757,724 bytes. This is the
exact old-archive total, not a measured compressed target-only saving.

At the conservative planning estimate of 600 MB for all three new registries,
the reconstructed complete steady inventory would be 6,944,452,056 bytes. Add the
observed macOS/Windows native replacement pair and a 256 MiB reserve: the total
is 10,368,274,438 bytes, 369,143,802 bytes below 10 GiB. Actual new archive sizes
remain pending. Three simultaneous native replacements do not fit this estimate;
the change is not an unconditional capacity guarantee.

The estimate assumes the selected old auxiliary archives have been retired.
Keeping every legacy archive while adding new registries is a different, unsafe
overlap case. No old cache is deleted by this patch. Before the first trusted
rollout, inventory any still-running old producers and wait for them to finish.
Prepare a freshly verified list of only the eight obsolete default-branch
auxiliary families for explicit migration review. Intentional retirement of
their target caches must be distinguished from the earlier same-family/version
replacement cleanup. Preserve current native/iOS, active PR, npm, Gradle and tool
caches. Do not begin that retirement from the preflight IDs alone.

After the approved migration window, capture new successful save keys, cache API
compressed bytes and a subsequent PR's actual restores. Existing action output
and read-only job/cache APIs provide these measurements; this patch adds no
packaging or measurement build. Confirm the three expected namespaces and record
normal download misses, every job's queue and execution, all check identities and
the longest merge-gating job. Historical cold auxiliary jobs were shorter than
the native critical path, but current graph growth and queue pressure require
measurement. No extra recovery dispatch is scheduled by this change.

## Evidence correction

The retained main53 report's `restored_bytes` fields for macOS engine, web and
Android are corrected to scope extraction to the Swatinem restore step. The old
generic parser could select npm/Gradle sizes instead. The Rust keys were correct;
cache API inventory sizes and every capacity calculation were unaffected. No
published source history is rewritten.

## Natural PR61 consumers

Corrected editor head c620868 restored all three exact registry-only seed keys.
Completed credential jobs passed the unchanged package tests and platform storage
probes: Linux 1m45s with 2s queue, Windows 2m55s with 2s queue, and macOS 2m28s with 10s
queue. The actual archives are 151,846,679/151,304,098/151,575,645 bytes respectively,
454,726,422 bytes total. These are whole-job timings; runner/setup variation and
normal Cargo work also contribute, so the total difference is not attributed
solely to caching. PRs performed no cache writes or extra seed dispatch.

The contemporaneous 17-entry inventory retains 6,968,145,208 bytes. Some compiled
families remain absent; this is not a claim that every target cache is warm.
The corrected native critical path is still pending; the initial desktop runs
stopped at a UI test deadline before Rust compilation.
[Exact consumer keys, steps, queue/execution and log hashes](evidence/ci-registry-consumer-pr61-2026-10-11.json).
