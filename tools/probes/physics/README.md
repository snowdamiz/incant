# Physics candidate probe

This isolated Cargo workspace compares two pinned candidates without adding a
physics backend to Incant. PLAN.md chooses Jolt; ADR 0003 requires measurement
and portability evidence before committing to its Rust integration. No engine
backend decision is made here.

Candidates: oxijolt 1.0.1 (Jolt 5.6.0), with cross-platform-deterministic;
Rapier 0.36.0, with enhanced-determinism. Both use f32 positions. Cargo.lock
pins all resolutions. Building Jolt needs a C++ compiler and CMake. Set CMAKE
to an existing executable if it is outside PATH.

From the repository root:

```sh
./tools/cargo build --manifest-path tools/probes/physics/Cargo.toml --release --locked
tools/probes/physics/target/release/incant-physics-probe jolt1 /tmp/jolt1.positions.bin
tools/probes/physics/target/release/incant-physics-probe jolt4 /tmp/jolt4.positions.bin
tools/probes/physics/target/release/incant-physics-probe rapier /tmp/rapier.positions.bin
```

The optional second argument writes raw little-endian f32 position bits for
all 512 bodies at every one of 600 steps. Compare this file across repeated
processes and Jolt worker counts. The stdout JSON includes sorted step times.
The FNV value is a quick checksum, not cryptographic proof of equality.

The scene contains 512 boxes (0.9 m sides, 1.2 m spacing) falling onto a static
floor. Gravity is -9.81 m/s², dt 1/60 s, friction 0.5, zero restitution/damping,
and sleeping disabled. The first 30 steps warm up; 570 are timed. Only stepping
is timed, excluding creation and pose extraction. Jolt counts worker threads
in addition to its calling thread; Rapier uses its serial world step. Solver
settings and default density differ between backends. This is a feasibility
probe with small synthetic geometry, not a controlled performance ranking or
an Incant game/device gate.

Initial macOS execution produces identical per-backend position sequences in
three independent processes; Jolt one/four-worker sequences also match. Actual
cross-compilation of the chosen Jolt binding fails for iOS simulator (target
not registered) and wasm32 (only 64-bit targets supported). This does not prove
Jolt itself cannot support those platforms; other bindings or a maintained
shim still need evaluation. No cross-platform runtime determinism is claimed.

Primary sources:
- https://github.com/pockerhead/oxijolt
- https://rapier.rs/docs/user_guides/rust/getting_started/
- https://github.com/SecondHalfGames/jolt-rust


`run.py --output <new-directory>` runs three alternating-order trials and checks
full position streams within each backend, including Jolt's worker-count change.
`cargo test` against this manifest checks Jolt floor contact, raycast identity,
pose/velocity replay after restore, invalid timestep rejection, stale handles,
and sensor entry/exit without collision response. These checks exercise the
candidate directly; Incant colliders, scripts and play sessions are not connected.

The `jolt` and `rapier` Cargo features allow isolated target builds. Both are on
by default. `portability.py --output <new-directory>` records the known Jolt
binding refusal and actual Rapier builds for iOS simulator and wasm32. It fails
on any unexpected error or on a changed Jolt result that needs reassessment.
Successful compilation is not device execution. Hosted CI compares native pose
hashes from macOS, Windows and Linux; its results must be recorded separately
before claiming cross-host repeatability.
