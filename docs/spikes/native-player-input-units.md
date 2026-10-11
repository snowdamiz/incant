# Native player logical input units

This bounded nonvisual correction was implemented by the director-authorized
`gpt-6.1-sol` agent at Extra High (`xhigh`), classified as `Built-by: astra`.
Its clean source baseline is `77ed9777b8218725a4a35c62848d23293feed8a8`.
It changes the native player input adapter, its per-window caller and numeric
tests. No editor UI, bridge, handoff or dependency resolution changes are included.

`incant_input/src/types.rs:172` requires device adapters to supply logical pixels.
The player forwarded Tao physical cursor positions and pixel-wheel deltas
directly, making these coordinates twice their logical values on a scale-two
display. The adapter now converts both with the actual `window.scale_factor()`
queried for each input event. Tao's checked `to_logical::<f64>` conversion divides
both coordinates by that factor without integer rounding. Invalid factors return
a typed `InputScale` error before changing the pending batch. Line-wheel units,
both axis signs, focus handling, the 1,024-event bound and first/latest cursor
coalescing remain unchanged. Moving between different scales uses the factor
provided for each sample instead of retaining a startup factor.

The audit uses the local Cargo registry source selected by the frozen lockfile:

- Tao **0.37.1**, Cargo package checksum
  `f37f381f4e048e6cdf038b5705f8cf14ad108279d46eb968140a7b291aba9400`.
  `src/event.rs:386` types cursor coordinates as physical positions;
  `src/event.rs:981` also types pixel deltas as physical positions.
- Tao `src/platform_impl/macos/view.rs:1003` converts its top-left logical cursor
  coordinates to physical coordinates using the backing scale. At lines
  1075–1080, it negates the native horizontal wheel sign, scales precise deltas
  into physical pixels and leaves line deltas unscaled.
- Tao `src/event.rs:972` documents positive line deltas as forward/rightward.
  Linux `src/platform_impl/linux/event_loop.rs:801` negates the GDK horizontal
  and vertical deltas into line units. Windows
  `src/platform_impl/windows/event_loop.rs:1302` and `:1337` normalize vertical
  and horizontal wheel messages into line units. The player retains Tao's
  resulting signs; this correction introduces no additional sign inversion.
- DPI **0.1.2**, Cargo package checksum
  `d8b14ccef22fc6f5a8f4d7d768562a182c04ce9a3b3157b91390b52ddfdf1a76`.
  `src/lib.rs:486` implements physical-to-logical division; `:143` checks positive
  normal factors before the conversion's assertion.

Four new tests call the same private value adapters used by the native event
match and then advance the real `InputRuntime`. They verify fractional pointer
position/displacement and pixel wheel values at scale one/two, unchanged line
values and signs, scale changes with zero invented displacement, atomic invalid
scale rejection and high-rate scaled cursor coalescing with button ordering.
The tests construct typed physical values without unsafe dummy native device IDs.

Validation ran on frozen source on local macOS 26.6 (25G72), Rust 1.99.0,
using the functional `ci` profile and `/tmp/incant-agent-native-target`:

```sh
CARGO_TARGET_DIR=/tmp/incant-agent-native-target ./tools/cargo test -p incant_game_player --features native-window --all-targets --profile ci --locked -j2
CARGO_TARGET_DIR=/tmp/incant-agent-native-target ./tools/cargo clippy -p incant_game_player --features native-window --all-targets --profile ci --locked -j2 -- -D warnings
./tools/cargo fmt --all -- --check
git diff --check
```

All **18 unit tests and one routing test** pass. The routing subprocess refuses
unsupported startup arguments before scene/device/window creation. All-target
Clippy, formatting and diff checks pass. Both Cargo processes were reaped before
the target directory was released back to the coordinator.

Raw local test log `/tmp/incant-native-player-input-tests.log` SHA-256:
`6f148e4f65ab1efbcb094cbab11130a0555ad20afafb78e0aa69b9aa5067ea62`.
Raw local Clippy log `/tmp/incant-native-player-input-clippy.log` SHA-256:
`ffbd1915876ce094a4613440643cf9881db346724282797cd4f231c2409f4adc`.
Tested source SHA-256 values:

- `src/window/input.rs`: `3e2bc519f839907b1b4841dc96fc494ddfec60024aad79065ca90b68237b11cf`
- `src/window/mod.rs`: `3b2d3495383df56050d74d78599b93b3e4c0f4a9caeb9cfe2f31bd193721af34`
- `src/window/input/coordinate_tests.rs`: `682bed941e66aef61bb057fabe1e08111a0f9a065a2abff06dcc2b0e1b8b7aa3`

These are numerical adapter/runtime checks and a pinned-source audit. No GPU or
native window was launched, and no live display transition, Linux/Windows
execution, rendered acceptance, performance measurement or phase gate is claimed.
