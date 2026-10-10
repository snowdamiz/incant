# Rapier serial / native parallel / browser parity

This small independent workspace tests Rapier 0.36's public rigid-body state.
It does not link the editor, extend document-driven gameplay, benchmark timing,
or replace the separate [Jolt candidate probe](../physics/README.md).

```sh
# Use the pinned root npm installation and Rust toolchain.
npm ci
./tools/cargo install -j2 wasm-bindgen-cli --version 0.2.126 --locked
rustup target add wasm32-unknown-unknown
node_modules/.bin/playwright-core install chromium
python3 tools/probes/physics-parity/run.py --output artifacts/solver-parity --trials 3 --browser
python3 tools/probes/physics-parity/features.py --output artifacts/solver-feature-policy.json
```

On Windows, use `cargo` directly for the install. If the CLI isn't on PATH, pass
`--wasm-bindgen /absolute/path/to/wasm-bindgen`. Output directories must be new;
earlier evidence is never overwritten. Without `--browser`, the runner only
certifies native execution and explicitly records the missing browser coverage.

The runner separately compiles serial and `parallel` executables from the same
Rust source and lockfile. It configures and checks explicit one/four-thread
Rapier pools, rotates execution order and starts a fresh process for each
scenario/mode/trial. Browser builds have no `parallel` feature and reject it at
compile time. Three fresh Chromium processes import the same WASM scenario
library, construct and step each world, and return the actual binary state
streams over loopback HTTP. Browser versions and source hashes enter the report.
The probe lockfile uses the product's shared dependency versions.

Each scenario uses gravity -8, dt 1/64, dyadic placement/impulse/torque inputs,
640 steps and stable spawn-order IDs. The contact case has 256 stacked cubes
plus 16 isolated sleep/wake cubes and a fixed floor (273 bodies). The joint case
has 24 eight-link chains with spherical joints and floor contact (217 bodies,
192 joints). Coverage checks require real contacts and natural sleep/wake
transitions; non-finite or unbounded numeric state fails. Every initial/stepped
frame records ID, position xyz, quaternion xyzw, linear velocity xyz, angular
velocity xyz and sleeping. No tolerance, quaternion canonicalization or
rounding is applied. The runner compares every little-endian byte and names the
first mismatching frame/body/field. Hashes summarize the compared streams and
allow hosted cross-OS checks; original raw streams remain CI artifacts.

CI retains the older candidate timing and position-reference jobs unchanged in
scope. Separate solver-parity artifacts cover macOS/Linux/Windows and actual
Linux Chromium/WASM execution; the compare job requires all those modes and
repeated trials. The feature-policy check inspects the **product** graph for
desktop, iOS, iOS simulator, Android and WASM. Graph checks do not execute phones.
Linux CI downloads the official pinned wasm-bindgen CLI and verifies its
hardcoded release SHA256 before extracting it; no tool compilation or new
dependency cache is needed. Local source installation above is optional when
that exact CLI is already installed.

This is two synthetic workloads and public rigid-body state, not hidden solver
state, all shapes/joints, soft bodies, whole runtime determinism, a speedup,
shipping-profile/device performance, Rapier 2D, the future fast variant or a
physics-backend adoption gate. Fixed inputs deliberately avoid host-native
transcendental math; application-side input determinism remains its own concern.
