# Repeated navigation-start attribution

This is a distinct, bounded follow-up to the unadopted navigation graph cache.
It starts from main298. The implementation is diagnostic only; it does not cache
lookups, change query order or relax any runtime or navigation work limit.
Native qualification and the full-Core diagnostic are complete. The measured
repeated scans are too small to justify pursuing this hypothesis for the 2 ms
target; no navigation optimization is adopted.

Each accepted nonfinal shortcut segment ends at an original path point. The next
iteration repeats the nearest-surface lookup for exactly that point, with the
same snap distance and immutable mesh. The source suggests possible redundant
work, but route corner counts alone do not establish how often it occurs or its
cost. Measure that cost before changing the algorithm.

The explicit `diagnostic-navigation` Cargo feature adds fixed counters and
durations for path queries, initial shortcut starts and repeated shortcut starts
to the existing same-frame ScriptTick observer. Initial starts exclude the
corridor's endpoint scans. Failed counts mean an error or absent observed result;
an unsuccessful optional shortcut can leave the original path valid. Queries
are nested in callbacks; lookups are nested in queries. None are additional
exclusive frame time. There are no per-triangle clocks or per-tick allocations.

Generation, frame, ordinal and callback ownership fence the observations.
Missing, overlapping, reversed, leaked or overflowing guards invalidate coverage;
ordinary early errors stay visible. Raw rows retain their compile-time 1,024-byte
bound and 4,096-row limit. Explicit navigation captures use v4 diagnostic schema2.
Original diagnostic captures retain schema1, and normal summary reports remain
v3. Normal exports do not enable either diagnostic feature.

The private full-Core adapter accepts only the original four-crate diagnostic
selection or the new exact five-crate selection. It verifies dependency rows and
actual target compiler features, preserves real compatibility fingerprints and
still requires exact authored inputs, bytecode, assets, logical output and PCM.
The normal exporter and shipping measurement admission are unchanged.

One fixed shipping diagnostic retains all 2,156 frames, both script
sources and the ten-thousand-entity workload. Its only feasibility calculation
subtracts each frame's measured repeated-start time from that same whole tick,
then ranks the resulting rows. This hypothetical zero-cost p95 is not a speedup
or gate result; instrumentation remains in the measurement. Zero repeated calls
or insufficient hypothetical savings stops this candidate. A plausible result
only motivates separately reviewed correctness work and matched shipping runs.

Sixteen data-only adapter controls pass, including original schema admission,
exact feature additions, actual compiler recipients, same-frame subtraction,
containment, missing observations and the stop condition. Seventy-one Rust
controls pass across the observer, navigation, actual ScriptHost detour/miss and
feature-off routes. Strict Clippy passes with and without the feature. These CPU
tests use the ordinary deterministic/compiler graph; the separate shipping
build below qualifies the fast4 runtime graph. No improvement or gate is claimed.

Independent review caught an initial analyzer bound that missed the final start
lookup before the sixty-fifth shortcut attempt refuses. The analyzer now retains
that legitimate observation; a positive boundary and negative successor control
cover it. The navigation algorithm and its attempt cap are unchanged.

## Actual private shipping diagnostic

Producing source is `73c1d493acb619b30c3b90e54776eccd6e2f6d90`, based on
main298. Rust 1.99, arm64, fast4 physics, opt3/fat LTO/abort/one codegen unit
were checked against actual compiler records. Only the five intended recipients
gain diagnostic features; no runtime compiler, checkpoints or SIMD8 enters the
player. All 74 original authored inputs and cooked data match the retained
main278 baseline. Bytecode, hardening and source maps remain identical; the
source-derived build ID changes honestly. The copied player's SHA256 is
`e92fa3f4909527f40412116d4abbf6b7e804389d4dad7e88981a8e240c3875fb`.

The fresh build completed in 357.68 seconds with at least 106,113,032,192 bytes
of sampled free storage. An earlier supervisor design was rejected before
execution because cancellation would not cover nested process groups. Its
replacement supervises each managed child in process; all 13 child records show
exit zero and reaping. No player ran during the build.

One unprofiled functional child and one maintained diagnostic child then each
executed the unchanged 2,156-tick Core10k recording. Both match the complete
historical logical report and 13,798,400-byte PCM exactly. Both terminate and
are reaped; there are no retries, warmup exclusions or discarded frames. The
diagnostic preserves both callback sources, complete schema2 coverage and the
normal histogram correspondence.

Machine: Mac17,9, Apple M5 Pro, 15 physical CPUs, 51,539,607,552 bytes RAM,
macOS 27.0.1 build 26A434, arm64, AC power. Low-power mode is unavailable, not
assumed false. Coordinated native lanes were quiet during the two children;
background system load is retained in their receipts.

| Nested observation | Calls | Total time over all 2,156 frames |
| --- | ---: | ---: |
| Path queries | 414 | 197.088280 ms |
| Initial shortcut starts | 338 | 6.016213 ms |
| Repeated shortcut starts | 1,026 | 14.370827 ms |

These durations overlap their enclosing callback and ScriptTick. They must not
be added to those totals. No observed query or start failed.

The instrumented whole-script p95 bucket is **2.580001–2.590000 ms**. Subtracting
each frame's repeated-start duration from that same frame, then ranking all
2,156 resulting values, gives **2.544041 ms**. Thus even the hypothetical
zero-cost repeated scan remains over 2 ms. This is an attribution result with
observer cost still present, not a matched optimization comparison or a new
shipping performance result. The specific scan-only hypothesis is closed.
The overall budget remains open to periodic, distinct investigations while
other work continues.

The [manifest](evidence/navigation-start-attribution/manifest.json),
[retained records](evidence/navigation-start-attribution/records.json.gz), and
[read-only replay](evidence/navigation-start-attribution/verify.py) bind the
actual source, commands, compiler receipts, functional output and raw frames.
Replay performs no build, gameplay or GPU work:

```sh
python3 -B docs/spikes/evidence/navigation-start-attribution/verify.py
# Also check retained local originals, player, cooked data and PCM:
python3 -B docs/spikes/evidence/navigation-start-attribution/verify.py --originals --binaries
```
