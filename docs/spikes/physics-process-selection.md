# Physics selection in development Play and desktop export

The developer launcher and ordinary desktop exporter read the authored physics
mode and select fixed engine Cargo features. Omitted settings retain deterministic
behavior. Fast uses hardware math; neither authored mode silently enables the
eight-lane optimization candidate. The Rust schema and startup admission remain
the final authority, and a mismatched cooker or compiled solver fails explicitly.

```sh
python3 tools/play_native.py path/to/game.json -- --ticks 120
python3 tools/export_native.py path/to/game.json path/to/new-export
```

Development Play uses the repository compiler and a cache outside the project.
A process lock covers building and copying the selected executable; gameplay runs
from a private copy after releasing that lock. Project bytes must still match
when Play starts. The project cannot select build flags or executable paths.
This launcher uses the existing native CPU host; the editor worker is separate.

The ordinary exporter selects the same mode for the trusted cook and shipped
player, checks the scoped normal dependency graph, and audits actual target
compiler records for all five solver dependencies. It retains the existing
four-unit shipping codegen, source-compiler exclusion and symbol checks. The
producer receipt must report the requested mode and exact compiled variant.

## Local verification

A public command-bus probe creates an authored TypeScript project, then selects
deterministic, fast and deterministic again. Each selection passes Undo/Redo,
save/reopen and twelve ticks of actual native Play. All three runs move the
entity to the expected position without modifying the authored project. A
deterministic process explicitly rejects the fast project before producing output.
The retained Cargo records confirm the actual target features in each run.

Fresh ordinary shipping exports of both modes run for twelve ticks after their
private source project, caches, maps and build storage are removed. Both copied
players match the corresponding native observations, logs and 9,600 stereo PCM
frames exactly, and leave their packaged files unchanged. The complete tool suite
passes: 278 Python tests discovered (four skipped) and 22 Node tests, zero failures.
All 1,526 frozen implementation inputs remained unchanged during the successful
shipping proof. [Retained records](evidence/physics-process-selection.json) include
83 lossless artifacts and identities for eight external executables/symbol files.

The first developer attempt failed because `rustc` was absent from the caller's
PATH; explicit repository toolchain discovery fixed it. The first shipping
attempt stopped before compilation because workspace-wide Cargo metadata included
an unrelated optional JIT path. The exporter now inspects the selected player's
normal graph. Both failures remain retained; neither is counted as a pass.

## Remaining work

Editor Play still uses its existing compiled worker and needs process selection.
This ARM macOS CPU proof does not establish window rendering, reference-phone
performance, cross-platform replay, SIMD promotion, or physics-world saves. The
movement fixture exercises source compilation and mode admission without active
physics bodies; solver behavior has separate tests. PCM is silence and establishes
repeatability, not audible playback. Shipping compiler JSON is audited before
temporary storage removal; retained verbose commands independently establish
the actual solver features and shipping codegen. Development binary copies are
ephemeral, with their build/launch records and hashes retained.
