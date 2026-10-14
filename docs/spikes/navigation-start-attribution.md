# Repeated navigation-start attribution

This is a distinct, bounded follow-up to the unadopted navigation graph cache.
It starts from main298. The implementation is diagnostic only; it does not cache
lookups, change query order or relax any runtime or navigation work limit.
Native qualification and full-Core measurements are still pending.

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

One prospective shipping diagnostic will retain all 2,156 frames, both script
sources and the ten-thousand-entity workload. Its only feasibility calculation
subtracts each frame's measured repeated-start time from that same whole tick,
then ranks the resulting rows. This hypothetical zero-cost p95 is not a speedup
or gate result; instrumentation remains in the measurement. Zero repeated calls
or insufficient hypothetical savings stops this candidate. A plausible result
only motivates separately reviewed correctness work and matched shipping runs.

Fifteen data-only adapter controls pass, including original schema admission,
exact feature additions, actual compiler recipients, same-frame subtraction,
containment, missing observations and the stop condition. Rust observer controls,
an actual ScriptHost detour/miss control, feature-off checks and strict lint are
authored but not yet run. No performance improvement or phase gate is claimed.
