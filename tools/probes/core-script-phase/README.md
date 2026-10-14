# Native script phase diagnostic

This test-only probe separates costs within the real `ScriptHost::tick_with_input`
path. It uses the unchanged03bc Core stress source,9,999 real typed transform rows,
exact stable IDs/ordinal props, a real native world and compiler-produced bytecode.
It does not load the rest of Core: controller,46 original entities, assets, UI,
audio, input playback and physics are absent. This is a diagnostic, not a matched
shipping performance experiment or a substitute for the failed2ms gate.

Fourteen timestamp boundaries exist only under `cfg(all(test,feature="profile",
feature="compiler"))`. There is no new feature, dependency, public API or production
algorithm/default change. A fixed14-element thread-local array records durations
only while this explicit test arms it. Each frame also retains an outer elapsed
value and its unassigned tail. Boundaries are ordered and checked. No numeric
view, pointer, ECS identity or guest object is retained by the trace.

The named test performs128 script ticks with the ordinary50ms guest deadline,
32MiB VM/loan caps and profiling enabled, including the existing heap walk every
tick. It checks every real transform's three coordinates, untouched rotation/scale
and finite checksum sentinel after each tick, outside the timed scope. It refuses
three independently omitted x/y/z-write variants. The world simulation tick stays
zero: only the script phase advances. Existing source ordering, loan revocation,
numeric validation and publication run unchanged. No profiler thresholds are
introduced and no region p95 values are summed. Instrumentation and the CI build
make values diagnostic only; source compilation/fixture setup/oracles are excluded.

Preparation uses the pinned Node22.23.3 binary and the already-locked local
TypeScript package. It verifies the exact TS hash and creates a fresh output
folder with the unchanged transpilation plus three explicit negative variants:

```
$PINNED_NODE tools/probes/core-script-phase/prepare.mjs "$PWD" "$E/prepared-001"
```

After independent source review and a CPU-slot grant, the only diagnostic native
execution is the exact named test below. Expect exactly one test passed, with128
positive ticks and three negative mutation controls inside it. `--ignored` here
selects this CPU-only test by full exact name; it never selects GPU tests.

```
INCANT_CORE_PHASE_INPUT="$E/prepared-001" RAYON_NUM_THREADS=2 CARGO_TARGET_DIR="$TARGET" \
  ./tools/cargo test --offline --locked --profile ci -j2 --target aarch64-apple-darwin \
  -p incant_script --no-default-features --features compiler,profile,physics-fast-simd8 --lib \
  tick_probe::tests::actual_native_stress_regions_128 -- --exact --ignored --nocapture --test-threads=1
```

Retain the full raw log, command status, source/prepared hashes and immediate test
executable copy. A `CORE_SCRIPT_PHASE` line holds every interval and the existing
profile report. A failed assertion is a failed diagnostic generation, never a
reason to expand a deadline or weaken the workload. No shipping build, GPU work
or quiet performance claim is part of this protocol.
