# Core tick cost diagnostics

The actual Core 10,000-scripted-entity shipping result remains above the 2 ms
script p95 gate. This experiment only identifies small candidates for subsequent
source review and matched shipping measurement. It changes no engine or authored
sample code, workload, VM budget or acceptance threshold.

The source is based on `03bc4b987f3e553249846c4cfce991793d4d2c48` and the actual
`samples/core/stress.ts` hash is
`7285e9009633052ba86ee0996c87f58097eb47fdaf0a73df10f42597b0119f1a`.
Pinned Rust 1.99.0, QuickJS 0.14.0 and Bevy ECS 0.19.1 were used with the explicit
`aarch64-apple-darwin` target, CI profile, two compiler jobs and Rayon workers.
The standalone resolved registry package identities are a subset of the engine
lock. This is not the engine's selected feature graph or shipping profile.

All 500 calls are retained for each JS variant, including cache construction.
Means from this single fixed-order run were:

| Variant | Mean ms | Diagnostic p95 ms |
| --- | ---: | ---: |
| Unchanged source | 1.104138 | 1.229208 |
| Count outside loop | 1.066745 | 1.204750 |
| Count and length outside loop | 1.066506 | 1.201416 |
| Forward cursor through ordinary array | 0.996408 | 1.143833 |
| Ordinary structure-of-arrays metadata | 1.094712 | 1.234791 |
| Structure-of-arrays bound to locals | 0.997015 | 1.143458 |

The independent scalar oracle checked every owned row's three written columns,
untouched padding/controller, and the sentinel at ticks 1, 64 and 500. Separate
controls changed stride while preserving IDs, reordered IDs, and changed the
attachment-to-ordinal mapping. Omitted actual writes and NaN in either checksum
were each rejected. Every candidate still performs all three writes and both
actual checksum reads for all 9,999 owned rows. No Float64Array metadata or
unrolled variant was adopted from earlier slower diagnostics.

The separate synthetic 10,000-token Bevy test included fresh safe `QueryState`
preparation in each of 300 batch intervals. Mean direct versus query cost was
52.50 versus 32.06 microseconds initially and 51.37 versus 32.82 after archetype
movement, despawn/ID reuse and wrong-ID mutation. Foreign and replaced worlds
were rejected by both paths. These use a synthetic `u32` ID component, not Incant's
actual 128-bit stable IDs or host. No state was cached across mutation.

The smaller cursor change is a reasonable next source candidate. The roughly
20-microsecond synthetic identity saving is useful but cannot account for the
full remaining shipping gap. No change is adopted by this packet. Measurements
ran without a quiet-window guarantee, with fixed variant order, standalone
fixtures and CI optimization; none establish a native script-phase percentile,
the 2 ms gate, startup, FPS, GPU or device performance.

Independent source review found that the original checksum comparison admitted
NaN. Source001 was retained without execution; source002 added finite checks and
two NaN mutations. The review accepted that correction. Two setup attempts were
then retained: metadata with `--no-deps` did not refresh the isolated lock and the
locked build refused it; full metadata next exposed the inherited diagnostic
JSON pin 1.0.145, which the engine-lock subset assertion rejected before building.
Source003 changes only that logging dependency to the actual pinned 1.0.151.
The independent follow-up accepted it. Final build, strict all-target Clippy and
one diagnostic execution all passed. The immediate executable copy is executable,
hash-bound externally, and all child handles were reaped before releasing the slot.

The compact packet embeds source generations, review receipts, generated fixture
and lock inputs, command logs and all raw intervals. It excludes only the copied
executable bytes while binding their size/hash. The external evidence root is
`~/.codex/incant-evidence/core-tick-cost-001`. Replay without compiling or running:

```sh
python3 docs/spikes/evidence/core-tick-cost/verify.py
python3 docs/spikes/evidence/core-tick-cost/verify.py --external --originals
```

`tools/probes/core-tick-cost/README.md` describes preparing a fresh diagnostic.
Actual retained argv, environment, tool versions and binary identity are in
`run-003/{result,binary,prepared-freeze}.json`. The supervisor and setup-failure
generations remain part of the archive. Production adoption requires a separate
review and full native lifetime/rollback controls before matched shipping runs.
