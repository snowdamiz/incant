# Vendored Parry compiler identities in shipping audits

Checkpoint19's macOS ordinary export compiled the fresh player successfully, then
refused the physics artifact audit. Cargo represents a package whose name matches
its source directory with a short ID such as `path+file:///.../parry2d#0.31.1`.
The audit still required the registry-style `#parry2d@0.31.1` suffix. The failed
candidate `2fab0688` and complete hosted log remain retained.

The corrected audit admits short or named path IDs only for the two pinned Parry
vendor directories, with the exact version and matching absolute Cargo manifest
path. Registry handling is preserved. Library kind, all five solver feature sets,
explicit target output paths, host exclusion, conflicting units and exactly one
successful build completion remain mandatory. No solver source, dependency,
workflow or timeout changes are included.

The new regression fails against the old parser and passes with the correction.
All 22 focused tests pass, including foreign path/version/name/manifest/kind
refusals and host-versus-target/wrong-math controls. Actual deterministic and fast
native compiler-artifact streams pass the corrected audit for all five solver
packages and reproduce the old parser's Parry identity refusal byte-for-byte.
These are ci-profile library checks, not a new shipping export.

The complete tools suite passes 369 Python tests (four existing platform skips)
and 22 Node tests; formatting, conventions and full-base diff checks pass. All
2,878 frozen implementation/build inputs remain unchanged through the six checks.
Commands, raw streams, the hosted artifact, old-parser replay and exact source
hashes are retained in the [manifest](evidence/physics-artifact-vendor-identity.json).
[Independent final review](evidence/physics-artifact-vendor-identity/final-review.json)
accepted source and evidence. The later Windows desktop failure is retained as
[same-cause evidence](evidence/physics-artifact-vendor-identity/windows-same-cause.json);
no additional source change was needed. Publication awaits the required fast check. A repaired frozen checkpoint
must execute the ordinary shipping export and pass all required release checks.
