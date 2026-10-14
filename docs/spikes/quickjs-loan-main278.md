# Main278 native Loan shipping result

The complete ScriptTick remains above the unchanged 2 ms budget in all six
fresh timing processes. All four within-mode ScriptTick and requested-memory
5% checks pass. M2 has a definite player-frame regression against M0; the other
three frame checks pass. Every composite comparison fails, and M0 remains the
preselected maintained baseline despite failing the budget. No saving is
established and no phase gate passes.

Execution source and base are exact main278
`0b57cc37db409cd228a7332aa42e2c9dad02ac2e`, after the reviewed native Loan undo/ID
capacity-reuse change in PR278. The retained source freeze contains 30 input
snapshots. This packet packages the existing
`quickjs-loan-main278-001` execution; packaging runs no player or compiler and
creates no new measurement. The earlier
[main275 measurements](quickjs-maintained-main275.md) remain historical and are
not used as a matched speedup verdict or relabeled as main278.

The one fresh public export used Rust 1.99.0
(`b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`, LLVM 23.1.1), locked/offline shipping,
jobs 2/Rayon 2, `aarch64-apple-darwin`, fast physics and summary profiling. The
public exporter's private-target lifecycle was unchanged. Its built-in one-tick
startup passed, and the retained build outcome reports no remaining private target.
The player SHA256 is
`21c23c9f8aae75a4cccb506af02925ceded760bf75b8449e25fcc17df8f02a02`;
the export receipt SHA256 is
`bbe4c115202626103873e9b1b2ac9daa510b677392868a27cc410afa50e00ec3`.

All three full functional controls—unprofiled, detailed and maintained—complete
2,156 ticks, with Core win and stress assertions passing. All 74 authoring files
remain exact. Full logical reports after removing only `profile` and exact PCM
match the historical unprofiled anchor. The PCM SHA256 is
`ccabf2c68b6388ac554ac3efb5609ddf6b4a45d1a3f316891d5f344094e99ee3`.
These functional controls may overlap other CPU/GPU work; their profile durations
are explicitly excluded from timing qualification.

The separate quiet sequence used the same player/package on Mac17,9, Apple M5
Pro, macOS 27.0.1 (26A434), arm64, AC power, 15 physical CPUs and 48 GiB RAM.
Low-power mode was unreported, not established as disabled. The recorded grant
acknowledges all agent lanes idle; it does not establish total OS/user idleness,
and other-repository development processes were present. The actual machine
metadata, grant, process-snapshot identity and release receipt remain retained.

The fixed order is D0, M0, M1, D1, D2, M2, with detailed anchor D0 and maintained
anchor M0 selected before execution. All six processes complete with every frame
included, no retry, no warmup exclusion and no quantile averaging. Each contains
2,156 valid heap observations and two ordered sources with 2,156 calls each,
zero unwind and zero unavailable CPU samples. Profile v3/heap v1, the selected
mode, 32 MiB heap limit and detailed/null estimate rules match. Each raw histogram
has 2,156 valid player-frame and ScriptTick samples, with no failed, missing,
invalid, overflowing or saturated samples.

| Run | Whole ScriptTick p95 interval, ns | Player-frame p95 interval, ns | 2 ms | Within-mode 5% frame / script / requested memory |
| --- | --- | --- | --- | --- |
| D0 | 2,180,001–2,190,000 | 2,990,001–3,000,000 | fail | detailed anchor |
| M0 | 2,120,001–2,130,000 | 2,990,001–3,000,000 | fail | maintained anchor |
| M1 | 2,080,001–2,090,000 | 2,900,001–2,910,000 | fail | pass / pass / pass |
| D1 | 2,190,001–2,200,000 | 3,010,001–3,020,000 | fail | pass / pass / pass |
| D2 | 2,210,001–2,220,000 | 3,000,001–3,010,000 | fail | pass / pass / pass |
| M2 | 2,200,001–2,210,000 | 3,170,001–3,180,000 | fail | **regression** / pass / pass |

M2's frame lower bound, 3,170,001 ns, exceeds 105% of M0's upper bound,
3,150,000 ns. This is a failed within-mode stability check; it is retained rather
than dropped, averaged or replaced. Process peak requested bytes range from
76,881,182 to 76,881,214. This allocator metric is not resident memory or the
QuickJS heap limit, and the retained cache is not evidence of lower memory use.

D0−M0, D1−M1 and D2−M2 have descriptive ScriptTick quantile differences of
50,001–69,999 ns, 100,001–119,999 ns and 1–19,999 ns respectively. These compare
independent distribution quantiles; they are not telemetry p95 or evidence of a
guest-loop or Loan saving. No strict 5% comparison crosses observation modes.
M0 callback wall/current-thread CPU means are 0.219984051/0.219875368 ms for Core
and 0.924967355/0.923929072 ms for the 9,999-instance bulk source. These contextual
means are not callback p95s or exclusive cost attribution, and no callback or
observer time is subtracted from the whole-tick p95.

The [portable manifest](evidence/quickjs-loan-main278/manifest.json) binds all 157
records, including every retained fresh original and clearly identified
historical oracle/authoring files. Its 3,189,241-byte archive embeds raw reports,
commands, metrics, source snapshots, complete verbose compiler logs and
functional/build/quiet receipts. PCM, player, assets, symbols and the unrelated
process snapshot remain external hash-bound originals. The
[summary](evidence/quickjs-loan-main278/summary.json) is an explicitly labeled
offline derivative; no raw execution file was changed.

```sh
python3 -B docs/spikes/evidence/quickjs-loan-main278/verify.py --verify-only
# On the original host, additionally rehash every retained original:
python3 -B docs/spikes/evidence/quickjs-loan-main278/verify.py --verify-only --external
```

The single external replay exited 0; its exact command and result are retained in
[replay.json](evidence/quickjs-loan-main278/replay.json). Verification only reads
files. The verifier's digest, histogram, instrumentation, metric, interval
comparison and Core oracle functions are byte-exact from the reviewed main275
verifier; only execution identities and this dataset's expected verdict/output
are adapted. It independently derives the p95 intervals and stability outcomes
from the raw histograms, checks functional/PCM parity and preserves all failures.
The public exporter removed its private Cargo JSON stream before independent
retention; published artifact reports and the complete verbose compiler log
remain available.

This is Mac CPU simulation evidence. It does not qualify GPU/frame presentation,
other device/platform combinations or a complete phase gate. The native Loan
capacity reuse remains functionally qualified, with no established shipping
saving and the 2 ms requirement still unmet.
