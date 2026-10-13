# Full private Rapier regressions after compound checkpoint integration

This CPU qualification combines lower compound integration `fdba894a` and the
reviewed work-precharge correction `19d42811` with the accepted Parry helper
producer `ac2e59d0`. The exact compiled source is private commit
`29bb8d9db9a579616d20e4c64e8aa8f926dfe9fd`. The prior Parry producer is unchanged.
No production source correction was necessary in this regression lane.

## Observed execution

The unfiltered private native library suite ran in both dimensions with
`incant-checked-restore`, `incant-checkpoint`, `serde-serialize` and `parallel`.
Each selected mode passed:

| Mode | Planar | Spatial | Failed / ignored / filtered |
| --- | ---: | ---: | --- |
| enhanced-determinism | 159 | 177 | 0 / 0 / 0 |
| SIMD8 | 159 | 177 | 0 / 0 / 0 |

Strict native-library Clippy passed in both modes. Selected normal dependency
graphs bind the intended feature on both Rapier and both Parry packages; SIMD8
does not enable enhanced determinism. All 96 package identities shared by each
prepared lock and the root lock are unchanged. Ten prepared-harness, metadata,
graph, native-test and Clippy commands exited zero. These runs include upstream
native regressions, rather than filtering to the checked checkpoint tests.

The native test commands took 74.37 seconds for deterministic and 93.01 seconds
for SIMD8, including their local compilation. These are audit timing records,
not runtime or comparative performance claims. There was no GPU execution.

## Reproducibility and scope

`tools/probes/physics-native-construction/prepare_world.py` created separate
vendor workspaces. The only vendor manifest change relocates the snapshot path.
The generated observer remains test-only. Sixteen inherited island/narrow
fixture blobs in the SIMD8 copy are bound to their existing mode-specific
captures, with original and replacement hashes. Current world controls capture
fresh native state; the fixture reuse does not establish whole-world byte
compatibility across numerical modes.

The source freeze retains 1,786 tracked inputs, including uncompiled vendor
files, and the verifier checks 1,608 prepared original/copy rows. Four executed
binaries were copied immediately after their corresponding suite, before a
later Cargo build could overwrite them; executable permissions, source and log
identities are recorded. The exact run script, commands, source/pin metadata,
logs, capture adaptations, selected graphs and binary receipts are retained.

One read-only evidence assertion initially read its own empty redirected JSON
output. That observer failure and an exact diagnostic reproduction are retained;
the corrected verifier writes its output after completion. Native source,
commands and executed binaries were not rebuilt for that packaging correction.

The portable [manifest](evidence/rapier-compound-regressions/manifest.json) and
lossless archive can be checked without executing native code:

```sh
python3 tools/probes/rapier-compound-regressions/collect.py --verify-only
```

On the producer machine, append `--originals --local-binaries` to rehash all
retained originals and binary bytes. The archived `assertions.py` additionally
checks prepared-source transformations, mode-specific fixture bindings and
shared lock identities. Final parent review is separate from this producer
execution record. This evidence does not qualify the outer runtime/game union,
public persisted physics admission, a GPU backend, reference phones or release
performance.
