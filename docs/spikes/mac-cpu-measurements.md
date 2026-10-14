# Matched Mac CPU measurements

`tools/mac_performance.py` consumes an existing public native exporter package
made with `--profiling summary`. It checks the actual shipping compiler log,
exported player and cooked data hashes before running the copied player with
`--profile`. It records this Mac's model, chip, architecture, memory, OS build,
power source and exposed low-power setting. The output directory is create-only;
the command receipt and stdout/stderr survive a failed run.

Example after creating a current shipping package:

```
python3 tools/mac_performance.py PACKAGE NEW_RUN --ticks 3600
python3 tools/mac_performance.py PACKAGE ANOTHER_RUN --ticks 3600 --baseline NEW_RUN/measurement.json
```

Optional `--input-recording` and `--observe` select a recorded gameplay route and
state assertions. Their bytes are copied to the run directory and included in
the workload identity. The exported UI viewport is preserved. Compiled script
source identity and a static module selector are part of the comparison, so a
changed script cannot masquerade as the same workload merely because its project
document stayed unchanged.

The runner consumes profile report version 2. It recomputes nearest-rank p95 from
the bounded per-frame histogram and checks both interval endpoints, frame counts,
missing/invalid/overflow counters and allocation availability. The measured
`script.tick` phase is the aggregate script tick; source percentiles are never
added. A baseline requires its original raw profile and matching machine,
workload, numerical mode, compiler, shipping flags and profiling features.

Comparisons retain quantization uncertainty. A result passes the 5% comparison
only when its upper bound is within 105% of the baseline's lower bound. Intervals
that overlap that threshold are inconclusive. The 2 ms script limit is checked
separately; meeting an absolute limit does not waive a regression.

This helper is one CPU measurement component, not Phase 1 acceptance. It measures
simulation time, not display cadence or GPU work. Requested Rust allocation size
is not process RSS or GPU memory. The caller must establish the actual Core
Sample/10,000-entity workload, repeat trials with comparable load and collect
separate frame presentation, shader-stall and total memory evidence. Load averages
are retained as context; they do not establish an idle machine. No warmup frames
are discarded. Other-device execution is deferred under PLAN revision 9.

Eight focused Python controls verify quantile boundaries, invalid/missing samples,
baseline identity and retained-report mismatches, conservative regression
decisions and non-Mac refusal. They are synthetic policy checks, not performance
results. Actual machine identity collection succeeds locally. A new shipping
execution with version 2 remains pending composition with the profiler change.
