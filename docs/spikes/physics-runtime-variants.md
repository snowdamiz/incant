# Separately compiled physics kernels

The document-free 2D and 3D physics crates now expose separate build variants.
Their default remains `deterministic`, preserving current callers. Building both
packages with `--no-default-features` selects hardware math; adding `fast-simd8`
selects Rapier's eight-lane candidate. Deterministic and eight-lane features are
mutually exclusive. Native builds retain `parallel` in every variant. Browser
builds retain the serial solver.

This is the kernel boundary, not completed per-project selection. Existing
dependent crates still request the deterministic default. Forwarding the build
choice through the runtime graph, preserving it in cooked data and selecting
matching export/play processes are the next work. The editor has no physics
toggle backed by this increment.

## Local validation

On macOS ARM64, the existing ten behavior tests pass separately in deterministic,
fast and fast-eight-lane builds. They cover 2D motion/collision/query behavior and
3D stable-ID parity, sleeping preservation and invalid-motion refusal. Each
variant passes all-target Clippy. This is behavioral regression evidence, not a
cross-variant bit-equality requirement or a performance benchmark.

Actual Cargo compiler-artifact records, including freshly built dependencies,
verify the selected features:

| Build | Rapier/Parry determinism | `simba/libm_force` | Native parallel | Eight lanes |
| --- | --- | --- | --- | --- |
| Default deterministic | Enabled | Enabled | Enabled | Disabled |
| Fast | Disabled | Disabled | Enabled | Disabled |
| Fast eight-lane candidate | Disabled | Disabled | Enabled | Enabled |

Both deterministic and fast packages also compile for `wasm32-unknown-unknown`;
their actual target dependency records omit `parallel` and `rayon`. These are
compile checks, not browser execution. An actual contradictory-feature build
fails with Rapier's diagnostic that eight lanes and enhanced determinism cannot
be combined.

The [retained manifest](evidence/physics-runtime-variants.json) binds source
hashes, exact commands, terminal outcomes, compiler feature records and test
output. No source changed during either run. Dependency resolutions are unchanged.
The dedicated target was cloned from an idle cache and all local workspace
packages were cleaned before it was assigned to this checkout.

No phone, shipping-profile timing, cross-platform checksum, rollback-state or
backend-adoption gate is claimed. Eight-lane SIMD is opt-in until target-specific
measurements justify selecting it. ADR 0003 and PLAN.md's numerical adoption rule
remain in force.
