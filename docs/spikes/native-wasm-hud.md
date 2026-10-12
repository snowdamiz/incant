# Native Rust/Wasm HUD controls

The additive `incant_hud_v1` interface uses operations 56–58 for bounded copied
pre-command widget/text declarations, routed UI outcomes and typed HUD controls.
Operations 0–55, input-v1 bytes and all earlier definitions are unchanged. UI
outcomes distinguish committed runtime input values from immutable declarations.
Both module paths accept composed UI; the existing router filters consumed keys
before gameplay input. Commands use the same ordered final-state validation and
transaction as native TypeScript. Invalid active calls reject the whole tick.

The frozen implementation passes 365 ordinary tests in 75 groups and three
actual Wasmtime/wasm2c tests, including 11 raw ingress cases and five composed
failure modes per host. The final actual-host tests use the exact Wasm bytes
accepted by the public exporter. Six boundary tests are included in the ordinary
count. All-target Clippy, 16 generator tests, generated equality, wasm32 SDK,
compiler-disabled runtime, dependency guard and formatting checks pass.

The ordinary public exporter produced a 9,818,064-byte player. After deleting
its disposable authoring/cache/build inputs and debug maps, three fresh 12-tick
processes agree on exact declaration/routed-event logs and PCM. Their canonical
CPU host reports actual shaped `Pickup 中文`, `Health 8` and edited `Name中文`,
two used fonts, zero missing clusters, nonzero raster coverage and expected
movement. Six precise refusal controls verify empty stdout and no output
publication for invalid role, ignored guest error, quota, short scratch, late
missing owner and the unsupported static-module checkpoint. The package's 22
file hashes are unchanged across execution.

Guest build paths affect byte identity. Earlier focused host runs used different
paths and are retained as separate evidence. One attempted reconstruction failed
an identity assertion before tests. A bounded reconstruction using the exact
original compile argv then reproduced every deleted build hash. That temporary
build was removed again, and the final host run passed with the matching public
guest. The original public runs already executed after deletion; no shipping
rerun or production change was needed.

The [final packet](evidence/native-wasm-hud.json) retains 30 source identities,
10 build inputs, 1,710 engine inputs, raw/compressed logs and public command,
report, PCM and refusal receipts. Earlier failed fixture/setup attempts remain
recorded. Independent source and final execution/durable packet reviews are accepted;
the [final receipt](evidence/native-wasm-hud/final-review.json) binds the exact proof. The earlier [source checkpoint](evidence/native-wasm-hud-source.json)
is retained with its original, narrower status.

Static/development guest checkpoints remain unsupported. No native-window,
GPU, rendered-appearance, live IME or device-performance gate is claimed. The
separate headless native-play UI viewport/recorded-UI route remains pending.
