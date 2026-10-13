# Complete native root-motion Wasm observations

Operations 53–55 add `incant_root_motion_v1` snapshot length/read and atomic
controls. Every published operation 0–52 and its record definitions remain
unchanged. The complete copied frame nests the unchanged IK-v2/pose-v5 frame,
then sorted root-owner rows with model identity, selected node, application
policy, exact signed cycle counters, phase, tick and four rigid motion receipts.
Consumers evaluate each pose's distinct interpolation groups, strip selected
root translation/rotation to cooked rest while retaining scale, then apply the
ordered IK constraints. The SDK preserves `i64` cycles and `u64` ticks exactly.

Old complete pose/IK readers refuse committed root stripping before touching
shared snapshot caches. This includes extract-only and disabled application;
disabling movement does not disable stripping. Graph-only readers remain valid.
The existing epoch/world binding, bounded copied buffers and staged transaction
apply: caught authorized failures poison the callback and discard numeric,
structural, animation, root, IK and log changes. A stale epoch does not poison a
new valid callback. Maximum composed frame size is 15,596,144 bytes, below the
32 MiB memory cap; call and transfer quotas still apply.

The actual Wasmtime and compiled wasm2c suite passed 453 tests across 80 result
groups, with zero failures or ignored cases. A real Rust guest and independent
typed native oracle match complete world checkpoints for 192 updates including
reverse cycles, paused clocks, compound 16-pose transitions, parameter/state
changes, root application/reset controls and ordered IK. Thirteen Rust failure
variants and 28 raw modes execute on both hosts. They cover all seven older
pose/IK APIs, copied buffer limits, exact quotas, late invalid owners, traps and
caught-error rollback. Successful-prefix controls establish that the same writes
commit when the later error is absent.

Eight-package all-target Clippy and three fresh development processes pass.
Supplemental direct tests cover a scene whose sole root binding is inactive
ExtractOnly, before and after cached complete reads. Their focused tests/Clippy,
SDK/runtime wasm32 checks, dependency guard, generated SDK, conventions and
format checks pass. All 240 Python tooling cases pass with four platform skips.
The isolated native-physics lock was refreshed offline to add only the new local
`incant_anim` package and dependency edge; existing versions stayed pinned.

The producer separately rehashed all 1,031 prepared files for 206 modules before
its private input directory was removed. Eighty root-specific original/prepared
Wasm, generated C/header and facade artifacts plus three root guest sources are
retained outside temporary storage. The durable packet also binds the complete
330-artifact/12-source selected conformance set and preserves failed preliminary
runs, including tool-path, test-lint, stale isolated lock and command-usage errors.
Production source and actual-host final evidence reviews are accepted; the
[retained final receipt](evidence/native-root-motion-wasm/final-review.json)
qualifies removed prepared inputs separately from directly reviewed artifacts.

[Exact source, raw/compressed logs and receipts](evidence/native-root-motion-wasm.json)
record what ran. The [ordinary public source-deleted game proof](native-root-motion-package.md)
also passed and has a separate review packet. Physics and arbitrary guest/module checkpoints remain
explicitly unsupported. CUBICSPLINE, general affine roots, rendered evidence,
reference-device performance and full Phase1 acceptance remain outside this
CPU boundary result.
