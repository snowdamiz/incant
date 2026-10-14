# Original Core Sample: actual Mac shipping CPU baseline

Three retained shipping-player runs complete the original Core Sample win with
aggregate script p95 below 2 ms on this Mac. This is a **46-entity deterministic
reference with one script controller**. It is not the fast 10,000-script workload,
display frame rate, GPU timing, visual acceptance, or an improvement comparison
against an earlier engine. The third run's 5% script repeatability result remains
inconclusive; this packet does not approve a phase gate.

| Run | `script.tick` p95 interval, ms | `player.frame` p95 interval, ms | Peak requested Rust bytes | 5% comparison against run 0 |
| --- | --- | --- | --- | --- |
| 0 | [0.840001, 0.850000] | [1.070001, 1.080000] | 59,540,056 | Baseline |
| 1 | [0.820001, 0.830000] | [1.040001, 1.050000] | 59,540,056 | All three metrics pass |
| 2 | [0.880001, 0.890000] | [1.060001, 1.070000] | 59,540,056 | Script inconclusive; frame and memory pass |

Each run covers all 2,156 requested frames, including startup/warmup; no samples
are excluded. Profile-v2 histograms use nearest-rank p95 (rank 2,049 here), exact
zero bin and 10,000 ns positive bins. The bounds are quantization intervals, not
statistical confidence intervals. Missing, invalid, overflowing, failed or active
frames refuse the metric. `script.tick` measures the aggregate script phase;
source percentiles are not added. An interval passes the 5% comparison only if
its upper bound is at most 1.05 times the baseline lower bound. Comparing two
upper bounds as exact samples would incorrectly turn run 2 into a pass.

## Workload and execution binding

The executed engine/source is `bff83af61807435926b10d77b5a04718a464aae5`.
The independently checked source freeze contains 4,724 logical paths. Both
attempts use the same 75-file prepared authoring input and unchanged controller:

- Project: `0df80837afec5e9074387213d4a7b257deb3b4053748d4a4b9b50aec5516b4c3`.
- Authored `gameplay.ts`: `8d5f5feec2235f1718235901c6073f7425acf971a4e087755a0b3833ecb79698`.
- Compiled bundle: `3c10ebc3edb3a3f231452d11d585dd2985eed8c3dcf03c4a3b6fe33bef8f583c`.
- Shipped player: `64824523e66398d5b320b8f9ab895c5375c034b78a80a4936dd305d5577f57d5`.

The input explicitly retains its original omitted `physics_mode`, whose authored
contract selects deterministic mode. Cargo's actual target artifact messages
confirm enhanced determinism in both dimensions and no fast SIMD features.
Actual rustc invocations show optimization 3, one codegen unit, panic abort,
debug information 2 and packed symbols for the runtime, script, player library
and executable; the executable uses fat LTO. Summary profiling is compiled and
selected explicitly, without Tracy transport. Rustc is 1.99.0, commit b940084d7,
LLVM 23.1.1; target is `aarch64-apple-darwin`.

Generation `mac-core-shipping-001` failed the build because pinned Node dependencies
were absent. Its exit 101, raw output and exact unchanged source freeze remain
retained. Generation `002` records `npm ci --ignore-scripts --no-audit --no-fund`,
Node v22.23.3, SWC 1.16.13 and TypeScript 5.9.3, then a fresh successful shipping
export. The setup receipt records npm success and dependency hashes; separate
raw npm stdout/stderr were not retained. No successful execution is attributed
to generation 001.

The public exporter removed temporary build storage before its source-free
startup check. The runtime package contains 28 cooked files and trusted embedded
QuickJS bytecode, with no TypeScript/JavaScript/map files in runtime data. Existing
cooked assets are reused, not reauthored. Symbol files/maps remain developer-side
external artifacts. This is not a claim that every source file on the host was
deleted. The normal runtime graph excludes document/command/agent crates.

A separate unprofiled functional run and all three profiled runs produce exactly
the same non-profile JSON report and PCM hash. They reach `core.won` at tick 2,066
with health 2 and four samples. The retained observation includes 11 entity
readbacks and one passing check; this is not a complete serialization of all
46 entities. Tick count, logs, observations and PCM also match the earlier
accepted Core win. Each run emits 1,724,800 stereo f32 frames at 48 kHz, SHA
`ccabf2c68b6388ac554ac3efb5609ddf6b4a45d1a3f316891d5f344094e99ee3`.
All render-input reports explicitly say CPU-only and `gpu_submitted: false`.

## Machine, load and limitations

The retained runner metadata identifies Mac17,9, Apple M5 Pro, 15 physical CPUs,
51,539,607,552 bytes memory, arm64, macOS 27.0.1 build 26A434, AC power. Low-power
mode is **unknown (`null`)**, not established as off. The runner checked its Mac
identity before and after each sample; those identities and per-run load averages
are retained, but separate raw sysctl/pmset output is not. The load averages around
4.0–4.4 are context, not proof that the machine was otherwise idle. Exact player
argv, cwd, PATH/TMPDIR-only environment and process timestamps are retained.

Requested Rust memory includes the enabled profiler's fixed histogram storage.
It is neither process RSS, QuickJS heap size nor GPU memory. Timing includes CPU
simulation/host work and offline audio, not real-time display/presentation or a
physical audio device. This three-run repeatability baseline does not establish
statistical long-run tails or a previous-engine performance improvement.

## Independent evidence audit and reproduction

The [manifest](evidence/mac-core-baseline/manifest.json) binds 278 records to 61
deduplicated payloads in a 4,203,280-byte gzip archive. Large player/symbol files,
reused assets and PCM are explicit original-path records with size and SHA-256:
185 records are external rather than silently omitted. The packet embeds raw
reports, both failed/successful build logs, command receipts, source freezes,
current runner/profiler source copies and prior independent source acceptance.
The audit does not independently re-review its author's profiler implementation;
that source acceptance belongs to the retained root review.

The separate verifier parses raw Cargo output, recomputes p95 from raw histogram
counts, compares exact workload identities and independently derives comparison
verdicts. Eight controls refuse altered p95/coverage/mass or a mismatched baseline,
and distinguish the retained inconclusive interval from a definite regression.
The [audit result](evidence/mac-core-baseline/audit-result.json) records all checks.
No compiler, player, GPU or performance run is launched by this audit.

From a checkout containing this packet:

```sh
python3 docs/spikes/evidence/mac-core-baseline/verify.py
python3 docs/spikes/evidence/mac-core-baseline/verify.py --external --git-source
```

The first command verifies portable payloads and recomputes metrics. The second
also hashes every retained original path and all 4,724 frozen Git inputs at the
immutable executed commit; it needs the original local files and that Git object.
It does not compare the moving current checkout with old execution evidence.

To repeat measurement later, reserve an idle measurement slot, use a checkout of
executed source `bff83af6`, and choose a **new, nonexistent** output directory:

```sh
python3 tools/mac_performance.py \
  "$HOME/.codex/incant-evidence/mac-core-shipping-002/package" \
  "$HOME/.codex/incant-evidence/mac-core-repeat-new" \
  --ticks 2156 \
  --input-recording "$HOME/.codex/incant-evidence/mac-core-shipping-002/functional-win/win-input.json" \
  --observe "$HOME/.codex/incant-evidence/mac-core-shipping-002/functional-win/win-observe.json" \
  --baseline "$HOME/.codex/incant-evidence/mac-core-shipping-002/measured-0/measurement.json"
```

This command is documented, not rerun by the reviewer. Preserve any mismatch or
inconclusive result. Fresh fast-10k shipping qualification is a separate experiment.
