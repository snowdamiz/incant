# Native ECS physics parity

`python3 tools/probes/native-physics/run.py --output artifacts/native-physics-parity --browser`

This isolated workspace uses the actual cooked scene decoder, NativeWorld,
Rapier backend, direct numeric columns and typed structural angular command.
Thirty-five bodies (contact pile, fixed floor, sensor, rotating compound) run 640
fixed dyadic ticks. Scripted column writes wake eight resting bodies. The stream
records every stable binary ID, position, quaternion, linear/angular velocity and
sleeping flag at every tick, including the initial state. Repeated fresh native
processes set and verify Rapier's documented calling/global Rayon pool at one and
four workers. Three fresh headless Chromium processes execute the identical Rust
scenario on the actual serial WASM runtime. No screenshots or visual assertions.

The recorder fails if collision/sleep/wake/event coverage is missing. Comparisons
check every byte and identify the first differing field, with separate cross-host
artifact verification. The existing raw Rapier native-serial/joint parity and
Jolt timing probes remain unchanged and separate. This does not add native joint
components or claim hidden solver-state equality, physics saves, device timings,
fast/SIMD adoption, backend adoption or a reference-phone gate.
