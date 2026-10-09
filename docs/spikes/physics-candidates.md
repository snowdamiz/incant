# Physics candidate feasibility

The isolated probe in `tools/probes/physics` runs pinned Jolt and Rapier
candidates without adding a physics dependency to Incant. PLAN.md still chooses
Jolt; no replacement decision is made. The engine's colliders, rigid bodies,
character controller, queries and script integration remain open.

On Apple M5 Pro, three independent 600-step simulations of 512 boxes produce
byte-identical position streams within each candidate. Jolt's one/four-worker
streams also match each other. Each step is 1/60 s; sleeping is disabled. The
first 30 steps warm up and the next 570 are timed. Initial medians of trial medians
are 0.365 ms for Jolt with one worker, 0.221 ms with four workers, and 0.283 ms for
serial Rapier. Jolt workers are additional to the calling thread. Solver defaults
and density differ, so these are feasibility measurements, not a controlled
ranking. A second harness run preserves timing variation in the evidence.

Two additional native behavior tests pass for Jolt: floor contact and raycasts,
pose/velocity replay after restoring a saved world, invalid timestep/shape
rejection, stale-handle rejection, and sensor entry/exit without blocking motion.
The probe builds with Clippy and the complete dependency resolution is locked.

Actual target builds show that the selected oxijolt 1.0.1 binding refuses iOS
simulator because it is unregistered, and wasm32 because only 64-bit targets are
supported. Rapier 0.36 builds for both targets. These are compile results, not
execution on either device. This does not establish that Jolt itself cannot
support these targets; another binding or an owned, audited shim still needs
evaluation before choosing the engine integration.

A scoped workflow runs the probe and behavior checks on macOS, Linux and Windows,
then compares recorded hashes across hosts. A separate job records the known
binding refusals and successful Rapier target builds. Hosted results remain
pending; a passed expected-refusal check must never be called platform support.

[Full local evidence](evidence/physics-candidates-2026-10-09.json) retains exact
position hashes, raw timing samples, commands, target outcomes and log hashes.

Primary sources: [oxijolt](https://github.com/pockerhead/oxijolt),
[Rapier setup and features](https://rapier.rs/docs/user_guides/rust/getting_started/),
[Jolt's linked Rust binding](https://github.com/SecondHalfGames/jolt-rust).
