# Physics kernel and save-boundary integration

The accepted kernel features and trusted-state diagnostic are integrated on
actual main169. The four production inputs and five standalone probe inputs
match their reviewed producer commits exactly. Only STATUS sections needed
additive reconciliation; the root dependency lock, simulation/world save code,
and all existing physics-save refusals are unchanged.

The same ten behavior cases pass in each of the deterministic, hardware-math and
opt-in eight-lane builds. All three pass Clippy. Actual Cargo compiler records
match the producer's five pinned dependency feature sets exactly, including
native parallel stepping, deterministic math isolation and eight-lane selection.
Deterministic and fast wasm32 checks pass without parallel features; a combined
deterministic/eight-lane build still fails explicitly as intended.

The separately built diagnostic again passes eight cases and 960 continuation
ticks. Its complete JSON result matches the reviewed producer result exactly:
trusted typed clones preserve the compared state/handles/events, while pose-only
reconstruction diverges after nonzero checkpoints. No serialized solver state is
accepted or decoded. This is evidence about the save boundary, not an adopted
world loader or production physics checkpoint.

All 15 recorded commands meet their expected outcomes, including the intentional
contradictory-feature build refusal. The [integration manifest](evidence/physics-boundaries-main169.json)
binds 1,827 frozen inputs, all nine accepted inputs, original and new logs,
actual feature records and the copied diagnostic executable. It also verifies
46 retained producer artifacts. The same ten tests run under three builds;
these are not thirty distinct cases, and diagnostic scenarios are not Rust tests.

[Independent integrated final review](evidence/physics-boundaries-main169/final-review.json)
is accepted; exact-head fast CI remains required. Project/process
physics selection remains separate work. No performance, reference-phone,
renderer, backend-adoption, exact production physics save or phase gate is claimed.
