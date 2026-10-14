# Core cursor and private inspector: shipping comparison fails the gate

The combined candidate does not qualify for production intake from this
experiment. All three script p95 results exceed the unchanged 2 ms limit, and
pair1 exceeds the 5% frame-time regression limit. No rerun or threshold change
was used to obtain a different result. The two changes remain separately
source/CPU reviewed experiments.

| Pair | Baseline script p95, ms | Combined script p95, ms | Baseline frame p95, ms | Combined frame p95, ms | Frame within5% |
|---|---:|---:|---:|---:|---|
| 0 | 2.410001–2.42 | 2.370001–2.38 | 3.300001–3.31 | 3.350001–3.36 | pass |
| 1 | 2.300001–2.31 | 2.390001–2.40 | 3.110001–3.12 | 3.320001–3.33 | regression |
| 2 | 2.370001–2.38 | 2.310001–2.32 | 3.270001–3.28 | 3.260001–3.27 | pass |

Script and requested-Rust-memory comparisons are within5% in each pair. Requested
Rust allocation peak is76,881,318 bytes baseline and76,881,225 candidate; this is
not RSS or GPU allocation. Small differences do not establish an improvement.
The intervals are nearest-rank p95 bucket bounds, not confidence intervals.

Baseline source is `03bc4b987f3e553249846c4cfce991793d4d2c48`, player
`a2794edcc1e7b21db3291eebb76cb2cf923290cc18005f91b023fe4de27fa361`.
Candidate source is `7f452e9bc1e3d82674051bd8dc366b9464d6f9d1`, player
`5d132416f4e6d4c42fe94e1e522b54437064fbd28978547b2620ab42dd2aef55`.
The candidate combines sample cursor source `0978f654` with private inspector
source `9b0a819d`; individual effects cannot be inferred. The prior source/CPU
reviews are retained unchanged. This result does not supersede their correctness
review and does not confer performance acceptance.

The actual machine reported Mac17,9, Apple M5 Pro,48GiB,15 physical CPUs,
macOS27.0.1 build26A434, arm64, AC power; no low-power-mode value was available.
Each raw command retains load averages. Root coordinated a quiet window across
all four agents, with builders, heavy archive work and GPU execution stopped.
No claim is made that unrelated user applications were idle.

The ordinary exporter built a fresh private shipping target using Rust1.99.0,
explicit `aarch64-apple-darwin`, locked/offline dependencies,-j2/Rayon2, fast8
physics and summary profiling. Compiler logs retain opt-level3, one codegen unit,
abort panic, packed symbols and final fat LTO. The unchanged exporter deleted
private build storage before source-free startup. Export wall time296.105s and
compiler wall time287.164s are diagnostics, not startup or runtime gate metrics.
No host-side compiler existed in the source-free player environment.

The original project, gameplay, IDs, settings and assets are unchanged. The
9,999-attachment stress source changed through public `write_script`, followed
by observed undo, redo, save and reopen. The retained historical authoring binary
`7b75efd9` was used only for public authoring. A fresh shipping player executed
all runtime checks. The document has10,045 entities and10,000 Script attachments
across two sources. The exact authoring delta includes stress.ts and its public
transaction history; cooked runtime data maps are equal.

Before profiling, one full2,156-tick candidate win run exactly matched the
baseline's entire non-profile JSON report and PCM. Forty paired65-tick batches
then observed all9,999 final stress translations, with independent finite scalar
checks, population and phase sentinels at1/63/64/65. All80 reports and PCM pairs
match; the total is81 unprofiled processes. This covers every final row at tick65,
not every individual intermediate write. Separate cursor tests retain omitted
x/y/z-write controls.

Timing used exactly B0,C0,C1,B1,B2,C2,2,156 ticks each, all frames included and no
warmup exclusion. Both sources have exactly2,156 callbacks per process, no
unwinds/unavailable samples, and complete finite histogram coverage. Every
non-profile report and actual PCM again matches the full-win control. All six
processes exit0 with empty stderr, supervisor37371 is terminal/reaped, and the
quiet window is released. Process success is distinct from the failed gates.

The derived comparison permits only the declared `script_source_sha256`
workload-key difference. All remaining match keys are equal. Exact engine/tool
changes are separately frozen, including the inspector and unexecuted standalone
diagnostic tools. The ordinary `mac_performance.comparison` policy is unchanged.
The interval rules require candidate upper<=1.05*baseline lower to pass; candidate
lower>1.05*baseline upper is regression, otherwise inconclusive. Absolute script
acceptance requires upper<=2,000,000ns. This is CPU simulation evidence, not
60fps, GPU, other-device or complete Phase1 qualification.

Portable evidence lives in `docs/spikes/evidence/core-cursor-inspector/`.
`verify.py` independently recomputes raw p95 intervals, pair decisions, complete
report equality, all9,999 row scalars/sentinels, command status and identity
joins. It does not execute the benchmark or import its evaluator. Assets,
authoring histories, PCM, player/symbol binaries and the large Cargo JSON stream
remain external with exact sizes/hashes. The portable packet contains their
identity records rather than claiming those files are embedded. Verification:

```
python3 docs/spikes/evidence/core-cursor-inspector/verify.py
python3 docs/spikes/evidence/core-cursor-inspector/verify.py --external --originals --git-source
```

The second command verified829 records,248 portable payloads and7,849 captured
build-relevant Git inputs across both source revisions. During verifier authoring,
one check initially expected a flat codegen field (the receipt is per-crate),
and another initially applied build-environment expectations to the deliberately
clean startup environment. Both packaging checks were corrected against raw
receipts; no exporter, runtime source or measured result changed.

Original evidence roots are `~/.codex/incant-evidence/core-cursor-inspector-shipping-001`
and `core-cursor-inspector-pair-001`. The baseline is
`mac-core-numeric-loan-shipping-001`. `collect.py` only repackages existing files;
it never launches a player. Do not repeat the timing run without a new reviewed
protocol and coordinated quiet grant. The sole publisher may retain this failed
experiment independently; it must not intake the production candidates as a
passing performance change.
