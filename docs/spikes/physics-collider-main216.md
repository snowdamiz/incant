# Collider integration on main216

Accepted producer `9c7f5a44` is merged onto actual main216 `428ab762` without
physics source, vendor, manifest or lock changes. The only implementation
changes relative to the producer are 17 previously merged renderer/UI/text test
and presentation files. None are in the snapshot crate's selected normal,
build or development dependency graph. The complete normalized selected graph
is identical in both checkouts.

Eight current checks pass: both graph captures, actual re-execution of the two
immutable producer binaries, the normal runtime dependency guard, formatting,
conventions and the complete main-relative whitespace check. The copied binaries
pass six collider cases and 28 native/trusted-world cases (34 total), including
both dimensions' exact collider history and 300-step continuation observations.
They were not rebuilt; exact source closure and binary identities are retained.
The full 135-case suite, strict all-target Clippy and wasm32 build remain the
separately accepted producer checks, not new current-main compilation claims.

The [integration manifest](evidence/physics-collider-main216.json) and
[read-only verifier](evidence/physics-collider-main216/verify.py) bind current
source identity, the 17 disjoint changes, raw/compressed logs and all 73 original
producer records. Source and final producer receipts are unchanged. The initial
false enabled-state invariant remains recorded. Independent current-main review
is pending.

This remains an inert boundary: no hostile decoded record is admitted into a
native world, live physics saves remain refused, and no shipping/GPU/device or
phase gate is approved. See the [producer contract](physics-collider-state.md)
for unsupported ownership and derived-state boundaries.
