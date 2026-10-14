# Immutable script attachment snapshot cache

`BoundGroup` now retains its frozen instance array and the corresponding
`Arc<[StableId]>` authorization list while attachment membership stays unchanged.
The first tick constructs the pair; later ticks restore the JS root and clone the
Arc handle. This removes repeated array population/freezing and ID-list allocation
at this boundary. It is an implementation change, not a measured speedup or a
passing 10k/performance gate. Workload, VM memory/CPU budgets, profiler intervals
and generated trusted bindings are unchanged.

Every tick still checks **every** original `EntityIdentity` against the native
world. Despawn/reuse permanently retires the attachment, including reuse between
script ticks. Any retirement invalidates both cached views before replacement
construction. Only a fully constructed/frozen pair is published. Source ordering
and surviving instance-object identity are preserved; old arrays remain immutable
historical data. They do not authorize a reused entity. Empty groups remain
skipped and their timers canceled by the existing host path.

The cache contains no state/timer/query/command capability or numeric view. Those
are created and revoked every tick as before. Fresh/checkpoint VMs build fresh
caches from the admitted active membership. Persistent roots drop before the
context/runtime. The SDK already describes these arrays as deeply frozen initial
data; it never promises a fresh array identity each tick. Callers must continue
treating the data as immutable. The cache keeps at most one current array and ID
list per source (existing limits: 64 sources, 32,768 attachments); guest-retained
old arrays remain covered by the unchanged QuickJS memory limit. The full native
lifetime scan remains O(attachments).

## Focused evidence

The source is based on preserved baseline packet `b34beb28`. Its original
`bindings.rs` is byte-identical to measured engine `bff83af6`. Only the BoundGroup
regions, a paired-return callsite and related tests changed; publisher-owned
`bindings_embed` and property-budget extraction are outside this commit.

All Rust checks use `./tools/cargo`, `--offline --locked --profile ci -j2 --target
aarch64-apple-darwin`, `RAYON_NUM_THREADS=2`, package `incant_script`, no default
features and `compiler,physics-deterministic`. Actual commands are retained:

- Four private native controls check shared array/Arc identity, complete lifetime
  invalidation, unchanged old snapshots, failed construction, empty/restored
  groups and inherited-array-accessor hardening.
- The final full `project_bindings` suite passes 17 cases, including five new
  public controls: cached data with fresh revoked capabilities/loans, ID reuse,
  failure after cache hit, fresh-VM checkpoint continuation, and caught retired-ID
  persistence refusal with numeric rollback. Existing hardening now also covers
  cache hits and a retirement rebuild.
- Fourteen existing checkpoint regression cases pass, including retired
  attachment membership and source-scoped persistence/timers across restore.
- Strict all-target Clippy passes before and after the final test addition.
  Scoped pinned rustfmt and `git diff --check` pass.

These are 35 distinct scoped passing cases, not a full workspace suite or both
physics modes. Source002 establishes 4+16+14; source004 retains the same production
and adds one case, with all 17 integration tests rerun. The source003 added test
initially failed because its inline JavaScript fixture omitted a closing brace
(16 pass/1 failure). Its exact source, log and immediate executable copy remain
historical evidence; source004 changes only that fixture brace. An initial
unqualified `rustfmt` PATH lookup failed; the absolute pinned 1.99.0 formatter
then passed. No runtime/parser behavior was relaxed to make the fixture pass.

[Portable manifest](evidence/script-instance-cache/manifest.json) retains all four
source freezes, exact commands/logs, and hashes of immediate executable copies.
The verifier checks source generations and distinguishes executed source002
library binaries from source004 integration binaries. Binaries remain explicit
local external files, not duplicates in Git. No GPU, player shipping run or
matched timing experiment was performed in this slice.

```sh
python3 docs/spikes/evidence/script-instance-cache/verify.py
python3 docs/spikes/evidence/script-instance-cache/verify.py --external
```

Final current-main composition and any actual performance comparison belong to a
separate reviewed integration and matched shipping experiment.
