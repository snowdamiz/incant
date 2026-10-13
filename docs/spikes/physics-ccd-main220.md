# Inert CCD and integration configuration on main220

The accepted producer `1ebee95e` and default-off borrowed CCD capture getter
`d45b6c0e` are composed with actual main220 `f1ff2839`. Existing narrow-owner
records remain intact. The only shared-source conflict was the error enum;
Integration, UnsupportedIntegration and Ccd are retained alongside Narrow.
All 1,542 physics crate, solver pin and build-control inputs are mapped to the
accepted producer or main220; only the combined library declarations differ
from both. No lock or manifest change is introduced.

The actual composed crate was rebuilt and executed in the dedicated physics
integration target, after removing only its local solver/snapshot products.
Both deterministic and fast8 runs pass 164 tests in 22 groups, with no failures
or ignored cases. Strict all-target Clippy in both modes, wasm32 compilation,
std-only normal dependency graph, dependency guard, formatting, conventions and
diff checks pass: eleven zero-exit commands. Captured Cargo artifact records
prove the actual pinned solver features for each compiled mode. These results
include existing narrow-owner behavior and the new native CCD/configuration
observer controls. Prior producer binaries were not used as a substitute.

[Manifest](evidence/physics-ccd-main220.json) and
[read-only verifier](evidence/physics-ccd-main220/verify.py) retain the complete
current input freeze, raw logs, exact source joins and independent producer review.
Independent final current-main review is accepted; publication awaits the required fast check. Producer 156-case full suites and later
21-case focused suites remain historical and overlap; they are not additive
coverage to the current 164 cases.

This is inert checked representation and trusted native observation only.
Cache/shape coherence, owner provenance and full-world reconstruction are still
separate prerequisites. Hostile live decoder admission, runtime restoration and
`PhysicsUnsupported` are unchanged. The vendor getter borrows cache state only
under the default-off checkpoint feature; it changes no solver algorithm.
No GPU, device, shipping performance or phase gate is claimed.
