# Native physics runtime migration (in progress)

## Typed backend extraction checkpoint — 2026-10-10

`incant_physics_runtime` contains the existing Rapier stepping/query algorithms
without authoring-document or navigation dependencies. Its ordered generic ID
parameter preserves the legacy facade’s exact case-sensitive text IDs while
allowing native `StableId` values without per-tick conversion. The shared ID
retains the existing public runtime re-export and representation. Typed physics
validation is shared with the authoring boundary. Native parallel and enhanced
determinism features remain enabled; WASM remains serial.

The original facade’s tests, authoring validation tests and native runtime tests
pass (`cargo test -j2 -p incant_physics -p incant_doc -p incant_runtime --locked`).
The four-package scoped Clippy check passes with warnings denied. Additional
facade tests prove case-sensitive ID identity and unchanged published schemas.
The typed backend test compares all public body state for 600 ticks using text
and binary IDs, verifies stationary fixed bodies and eventual sleeping, and
proves unchanged direct motion input does not wake bodies. Invalid direct motion
leaves the prior body state unchanged. Both physics packages pass tests and
Clippy after that API addition. All commands used `./tools/cargo` with two jobs.

This checkpoint is extraction, not native ECS integration. Cooked physics,
structural commands, native fixed-step ownership, fail-closed save semantics,
source-free shipping evidence and target feature graph checks are still pending.
Character/navigation migration, physics saves, device measurements and physics
backend adoption are not claimed. Existing benchmark outputs are unchanged.
