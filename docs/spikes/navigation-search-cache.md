# Immutable navigation search graph candidate

Source-only candidate after the correlated diagnostic in
[core-script-attribution.md](core-script-attribution.md). This does not establish
a performance benefit. The controller callback's navigation-refresh correlation
motivates the work; the diagnostic did not isolate adjacency construction time.

`NavigationMesh` now owns an `Arc<PreparedSearch>` containing boxed directed states
and compact per-polygon adjacency ranges. `rebuild_with_links` prepares it only
after new geometry and resolved links succeed, then publishes the entire mesh.
The existing unchanged-generation return preserves the old graph. Cloned meshes
and retained `NavigationSnapshot` handles share their generation's graph; there
is no global generation-number cache, cross-world key or mutable query state.

Portal midpoint and link endpoint/cost arithmetic is unchanged. Portal order,
then stable link order and forward/reverse direction order retain the original
one-based state indices. Index zero is still a fresh per-query start. Heap ties,
cost/parent arrays, visit charging, corridor limits, nearest-point scans, funnel
and optional shortcut work are unchanged. Costs/parents/heap remain query-owned.
No query endpoint, snapped point, route or previous refusal is cached.

More than 262,144 directed portals produces only a graph sentinel. The existing
refusal still occurs at search entry **after** public request validation,
endpoint lookup and the same-polygon shortcut. Cache preparation does not make
that previously query-specific refusal a failed rebuild. Invalid enabled links
still fail at the same staged resolution boundary before any publication.

For P directed portals, L directed links and N polygons, requested retained array
storage is exactly `(P+L)*(size_of(State)+size_of(usize)) +
(N+1)*size_of(usize)`. P<=262144, L<=256 and N<=32768 come from existing admission;
a compile-time assertion bounds those arrays below 24 MiB per admissible mesh.
Arc/object and allocator metadata are additional. Construction temporarily uses
N cursor indices; the boxed arrays originate from exact-count vectors. An
over-limit sentinel retains no state or adjacency arrays. Old and staged new
graphs may coexist, and callers retaining old snapshots retain their graphs as
well as their existing geometry. This is additional persistent storage in
exchange for removing repeated query allocation/construction; it is not a new
global memory ceiling or a zero-memory optimization. The runtime still admits
at most eight active navmeshes with its existing aggregate polygon/detail bounds.
The conservative array bound for eight distinct active graphs is therefore
192 MiB, not 24 MiB in total. A complete staged replacement can coexist with the
eight old graphs (384 MiB conservative arrays across both generations, plus
metadata and construction cursors). Arbitrarily retained public snapshots can
keep additional old generations alive; the bound is per distinct graph, and
shared clones do not multiply its arrays. No total live-snapshot memory cap is
introduced or claimed here.

The original main281 search is retained under `cfg(test)` as
`search/uncached.rs`; only its shared Edge/Route definitions, explanatory header
and method name differ. The candidate adds 144 complete path comparisons
(enabled/disabled/one-way/bidirectional links, endpoints/snap distances and visit
budgets) and 300 exact directed-search comparisons including equal-cost ties,
unreachable goals and successive failed/successful queries. Existing independent
Dijkstra/geometry controls remain. New generation tests include changed tiles,
failed enabled-link resolution, link-only replacement, immutable old clones and
the over-limit same-polygon/invalid-request/missing-endpoint order. Two existing
native runtime navigation tests now retain snapshots across tile and link edits.
Two manually assembled private precision fixtures explicitly prepare their graph.

No native test/build, navigation microprobe or shipping comparison has executed
for this candidate at the initial source freeze. Its proposed focused qualification
is serial Rust 1.99, arm64, CI, locked/offline, jobs 2/Rayon 2: full incant_nav
tests; native runtime navigation tests in deterministic and ordinary fast4 modes;
strict incant_nav all-target and runtime navigation Clippy; formatting/diff. Any
later microprobe is diagnostic only. Full-Core shipping evaluation must keep all
2,156 ticks, authored script cadence/enemy counts, numeric validation/rollback and
six publication-spike frames under the unchanged whole-tick protocol.

## Completed focused qualification

Implementation `4c1220e8514502f6dc7249481270905a5e00948c` and the subsequent
docs-only retention clarification `f2ff2841119e125ca3bcac19f3e1edbd51c88549`
are preserved. Frozen source002 SHA256
`9490477aa358917a9c6032588746ba01f1d56f61070b156bb99dbeb99faf27ec`
binds eight changed paths and 4,356 tracked compiler/source inputs. The retained
old search method is text-identical to the parent after only method-name,
visibility and whitespace normalization.

The six authorized serial commands passed without a repair: 35 incant_nav tests,
14 native runtime navigation tests in deterministic mode, 14 in ordinary fast4
mode, incant_nav all-target strict Clippy, and the runtime navigation target's
strict Clippy in both modes. There were zero failed or ignored cases. Actual
compiler records show enhanced-determinism only in the deterministic arm and
no SIMD8 in the ordinary fast arm. All used Rust 1.99, arm64, CI, locked/offline,
jobs 2/Rayon 2 through the pinned repository Cargo wrapper.

[The qualification manifest](evidence/navigation-search-cache/manifest.json)
binds 32 external records and ten immediate test-executable copies. Supervisor
68523 exited zero and was reaped; source and full input hashes still matched.
The target and slot were released. Target allocation grew 320,049,152 bytes;
free space remained above 12.96 GB, within the 2 GiB observed growth / 6 GiB free
guards. Those periodic cancellation checks are not an OS storage quota.
No player, shipping build, navigation microprobe or timing comparison ran.
The cache's memory/performance tradeoff therefore remains unmeasured.
