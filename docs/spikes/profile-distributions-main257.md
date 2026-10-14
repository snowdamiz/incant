# Profiler distributions and Mac runner on main257

Tested source `f0f91be4fd1761edd9b0dfd042f5d818ca8ee1aa` composes the
accepted per-frame CPU profiler and Mac measurement runner on actual main257.
All 15 changed implementation/test/tool paths are byte-exact producer inputs.
There are no source conflicts, dependency-manifest changes or lock changes.
The STATUS merge is documentation only. Current source review is accepted;
final execution evidence review is pending.

The focused current-source qualification passed all 24 commands. Its 17 named
CPU cases cover five histogram units, five summary controls, actual two-source
ScriptHost aggregation, four player wrapper controls, a real three-tick CLI
roundtrip and the default no-op path. A separate allocation-counting process
passed. The eight Mac runner policy tests passed, and the runner parsed the
current actual CLI report. Three strict lint scopes, summary-enabled wasm
compilation, the empty default normal-dependency graph, formatting and conventions
passed. Eleven executables were copied immediately. The CLI test uses its compiled
child path; that exact child's bytes were checked against the immediate copy
before and after execution, before any subsequent build could replace the file.

This is CI-profile correctness evidence on the Mac, with one entity and three
CLI ticks. It is not a shipping benchmark, 10,000-entity scene, performance-gate
result, frame-rate test or GPU measurement. The runner reports percentile intervals
and missing/invalid/overflow coverage explicitly; it does not invent exact p95
values from histogram bounds. Actual shipping exports and measurements are a
separate root-owned lane.

The original profiler archive retains 83 records, 17 source inputs, 14 commands
and 11 historical executable copies. The runner archive retains 13 records and
its actual earlier CLI parsing receipt. Both original verifiers replay unchanged.
Their execution is not relabeled as current source. Explicit unchanged-tree
merges preserve producer Git identities for portable verification.

The current archive retains 31 records with 27 lossless payloads, including
3,880 frozen current inputs, exact source mapping, raw commands/logs, artifact
features and the emitted CLI JSON. Final processes were reaped and the CORE CPU
target was released before the next worker resumed.

```sh
python3 docs/spikes/evidence/profile-distributions-main257/verify.py --verify-only
```

Use `--originals --local-binaries` to rehash original local records and retained
executable copies. This verifier does not build, sample performance or execute
captured test commands.
