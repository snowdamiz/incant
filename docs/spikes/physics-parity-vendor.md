# Portability probe recognizes the pinned Rapier patch

Checkpoint21's portability job refused the selected native dependency graph after
the trusted continuation work moved Rapier to the repository's pinned source.
The probe searched for `rapier3d v0.36.0|`; Cargo correctly added the local source
path before the feature separator. The original exact-head failure and a local
old-source reproduction are retained.

The corrected probe requires Rapier 0.36.0 at its exact canonical repository patch
path. Every selected unit must match; foreign paths, registry substitutes, wrong
versions, conflicting units and missing solvers fail explicitly. The previous
native-parallel, Wasm-serial and enhanced-determinism requirements are unchanged.
No dependency, solver, workflow, build flag or required context changes.

Seven focused tests pass, including both dimensions and all seven targets with
positive and negative identity controls. Actual selected normal dependency graphs
pass for five product roots, yielding 49 solver/target records. The complete tools
suite passes 373 Python tests (four existing platform skips) and 22 Node tests;
conventions and diff checks pass. These are resolved graph checks, not target
execution. The [packet](evidence/physics-parity-vendor.json) retains exact raw bytes
and the [source review](evidence/physics-parity-vendor/source-review.json).
The current candidate stays frozen until all jobs finish; a reviewed successor
must pass the actual hosted portability check and every release requirement.
