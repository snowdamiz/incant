# Navigation graph cache: retained shipping experiment, not adopted

The immutable navigation graph cache produced small observed ScriptTick p95
reductions in all three fixed pairs, but every candidate run still failed the
unchanged 2 ms budget. The final pair had identical frame-p95 buckets. This
bounded experiment does **not** justify adopting the additional persistent graph
storage for this budget effort. The implementation remains private; this packet
is documentation and evidence only. No further profiling or timing is planned
for this optimization.

## Exact scope

Candidate source `21e3177abad4737969f96d1b5487d9296c839a99` contains implementation
`4c1220e8514502f6dc7249481270905a5e00948c` and qualified source
`f2ff2841119e125ca3bcac19f3e1edbd51c88549`. The baseline is retained main278
`0b57cc37db409cd228a7332aa42e2c9dad02ac2e`. The source inventory and closure audit
are included in the archive: the selected algorithm delta is the three
`incant_nav` build/search files. Diagnostic-only attribution hooks compile out;
normal feature rows and codegen were checked against actual compiler records.
The unrelated rejected ordered-query optimization is absent.

The candidate prepares immutable directed states and CSR adjacency once per
successfully published mesh generation. It retains exact state/link enumeration,
heap ties, request validation/refusal order, endpoint/funnel arithmetic and
query-owned scratch. Old snapshots share their generation's graph. Focused
qualification passed 35 nav tests plus 14 runtime navigation tests in each of
deterministic and fast4 modes, with three strict Clippy commands. The retained
original algorithm supports 144 full-path and 300 directed-search differential
cases; changed tiles/links, failed staged rebuilds and old snapshots are covered.
The original qualification manifest and all 32 records are embedded in this
packet, so landing these docs does not require the candidate code commits.

The cache adds bounded arrays per graph, not a global retention limit: less than
24 MiB per admissible mesh, conservatively 192 MiB across eight active graphs or
384 MiB across old/new full replacements, plus metadata/construction storage.
Callers retaining older snapshots can retain additional generations. The actual
Core workload uses much less, as measured below.

## Build and functional correspondence

One fresh public exporter build completed in 275.38 seconds, with minimum sampled
free storage 7,660,601,344 bytes. Its temporary targets were removed after immediate
proof copies. Rust 1.99, aarch64-apple-darwin, fast4, shipping opt3/fat LTO/abort/one
codegen unit and the baseline's exact normal profile feature sets were retained.
Neither native-checkpoints nor diagnostic-script-phases nor a runtime compiler
was selected. Player SHA256:
`24961ec729fc0363057857611dae3c379be2dbf0879cff41ff137ebdf2120837`.
Baseline player SHA256:
`21c23c9f8aae75a4cccb506af02925ceded760bf75b8449e25fcc17df8f02a02`.

All 74 authored inputs and all 28 cooked data files matched. Script bytecode and
other payloads matched; the candidate honestly carries new source-derived build
ID `9589e3c563c1f8c9dbe5e46da7bd9adbaaafc5ffdbc9abe26c420f4c13334cf1`, rather
than copying the baseline ID. Three separate unprofiled/detailed/maintained
2,156-tick functional children passed the original Core and binding oracles,
exact logical JSON equality and exact PCM equality. Those profiles are functional
controls only. The earlier preparation assertion about a feature-line comment is
retained as a failed setup attempt; no native build or timing retry was made.

The completed successful build has a lifecycle qualification: unchanged
`shipping_symbols.py` runs symbol tools with synchronous `check_output`, outside
the added cooperative cancellation callback. Those calls returned and were
reaped on this successful attempt. This packet does not claim a general timeout
or process-group cleanup guarantee for those symbol tools. Native player stages
used the reviewed managed cancellation/cleanup path.

## One fixed six-process comparison

The exact order was B0,C0,C1,B1,B2,C2, with C0/B0, C1/B1 and C2/B2 fixed before
execution. Every process used maintained normal-v3 profiling and the same full
2,156-tick, 10,000-attachment/two-source workload, input recording, observations,
640×480 UI viewport and audio route. No warmup, publication-spike frame, sample
or pair was excluded; no quantiles were averaged. There were no retries.
All six gameplay reports and 13,798,400-byte PCM outputs matched exactly.
The child/sequence execution limits were 45/300 seconds, with mandatory bounded
probes and cleanup outside the sequence execution deadline.

Actual machine: Mac17,9, Apple M5 Pro, 15 physical CPUs, 51,539,607,552 bytes RAM,
macOS 27.0.1 build 26A434, arm64, AC power. Low-power mode was unavailable rather
than assumed false. Other coordinated agent lanes were quiet. Background host
load remains visible in every receipt; this is not a controlled laboratory.

Raw histogram p95 intervals are in nanoseconds (10,000 ns buckets):

| Pair | Baseline ScriptTick | Candidate ScriptTick | Baseline frame | Candidate frame |
|---|---:|---:|---:|---:|
| C0/B0 | 2,220,001–2,230,000 | 2,150,001–2,160,000 | 3,260,001–3,270,000 | 2,980,001–2,990,000 |
| C1/B1 | 2,150,001–2,160,000 | 2,110,001–2,120,000 | 3,010,001–3,020,000 | 2,930,001–2,940,000 |
| C2/B2 | 2,130,001–2,140,000 | 2,110,001–2,120,000 | 2,960,001–2,970,000 | 2,960,001–2,970,000 |

All three interval-based 5% frame/script/requested-memory comparisons passed.
All three independent candidate ScriptTick 2 ms outcomes failed. Conservative
observed script-p95 reductions were 2.70–3.59%, 1.40–2.31% and 0.47–1.40%.
These are bounds from each pair's histogram quantization, **not** confidence
intervals or a universal speedup. Baseline p95 declined through the fixed
sequence, especially the first frame result; all samples remain included.
The source0/source1 observations and maxima are preserved without attributing
those individual delays to the cache or to a guessed background cause.

Rust process-peak requested bytes rose from 76,881,242 in every baseline to
76,951,830 / 76,951,862 / 76,951,862: +70,588 / +70,620 / +70,620 bytes
(about 0.092%). Requested allocation traffic fell by about 62.5 MB per run and
allocation calls by about 180,600. This confirms the cache's expected allocation
tradeoff but does not make its small total-tick improvement a gate pass.
Requested sizes are neither RSS nor GPU memory. The retained frame maxima are
12.77–14.05 ms and ScriptTick maxima 11.30–11.90 ms; none are dropped.

## Replay and disposition

[Manifest](evidence/navigation-search-cache-shipping/manifest.json),
[portable records](evidence/navigation-search-cache-shipping/records.json.gz), and
[independent read-only verifier](evidence/navigation-search-cache-shipping/verify.py)
bind the source audit, actual build graph/logs, original functional/timing
reports, commands, grants, raw histograms, and failed setup. Binary/data/PCM
artifacts remain external with exact hashes; portable replay needs no native
execution or external source checkout. The archive also contains the exact
candidate source/design for review without landing it.

```sh
python3 -B docs/spikes/evidence/navigation-search-cache-shipping/verify.py
# Optional retained-original, binary/PCM and immutable Git input checks:
python3 -B docs/spikes/evidence/navigation-search-cache-shipping/verify.py \
  --originals --binaries --git-source /path/to/incant
```

The verifier independently locates rank 2,049 in each actual 2,156-sample frame
histogram, replays all fixed comparisons, and checks functional/mode/machine
correspondence. It never launches a player or build. All owned build, functional
and timing supervisors exited zero and were reaped (63906, 75741, 34897).
The 2 ms budget remains unmet. The cache is **not adopted**; continue other
engine work without another optimization/profiling branch for this budget.
