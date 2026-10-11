# Optional native window player

The existing `incant_game_player` now has an additive desktop `native-window`
feature and `--window` mode. It owns one Tao window, wgpu surface, device audio
backend and `HostedGameSession<DeviceBackend, NativeRenderHost<GpuRenderConsumer<SurfaceTargets>>>`.
It consumes the same cooked scene/assets and build-embedded QuickJS artifact as
the [offscreen player](native-game-gpu-player.md). The vetted `build.rs` is
unchanged. Default, CPU-input and offscreen-GPU modes remain available; this is
not an editor, a second gameplay loop or a runtime source compiler.

## Lifecycle and bounds

The window reserves an owned surface frame **before** accepting a simulation
tick. The mandatory GPU consumer takes that reservation, submits and presents
inside the tick. No surface frame, borrowed target or unsubmitted command buffer
escapes an event callback. A failed accepted tick destroys the composite session;
submitted GPU work and device audio cannot be rolled back.

- Physical sizes are bounded to 16–1920 by 16–1080 pixels, initially 640×480.
  Zero/minimized surfaces defer work. Resizing invalidates the configuration.
- Each acquisition performs at most two attempts. `Outdated` reconfigures;
  `Lost` recreates the surface and rechecks its capabilities. Repeated loss,
  timeout and occlusion defer simulation and audio. Validation/device failures
  are terminal. Surface presentation uses FIFO with maximum frame latency two.
- Focus loss, minimization and platform suspension freeze ticks and master
  audio. Resuming discards the paused wall-time debt. Input focus reset releases
  previously held controls; physically held keys need a new OS event afterward.
- The integer clock retains fractional nanoseconds without per-tick rounding
  drift. A callback accepts at most four catch-up ticks and counts/discards
  additional whole wall ticks. This can slow simulation relative to wall time
  under overload; it is not a measured frame-budget guarantee.
- The input queue has 1,024 entries. Each contiguous absolute cursor run retains
  its first and last sample, preserving displacement even when the previous
  position is unknown. Buttons, keys, wheels and focus preserve their order.
  `CursorMoved` is the only uncaptured motion source: forwarding raw device
  motion as well would count movement twice. Nonfinite/out-of-range positions
  and genuine discrete-event overflow are rejected.
- Keyboard and three-button mouse bindings are supported. Gamepad, gestures,
  extra mouse buttons, captured-pointer mode, text/IME and haptics are outside
  this host. Unsupported declared action bindings fail at startup.

`Mixer` now owns one root subtrack below Kira's main track so master suspension
does not rewrite individual source playback state. Paused sources remain paused;
static and streamed cursors do not advance while suspended. Commands apply at
an audio callback boundary, and a callback can precede the initial suspension.
Kira interpolates resume gain over its first internal chunk even with a
zero-duration tween; the regression verifies exact cursor/sample correspondence
after that chunk rather than claiming a bit-identical transition envelope.

The native device specialization exposes read-only `poll_device`: any reported
CPAL error, unavailable error queue or discarded error fails the entire session.
It runs before accepted ticks and every 100 ms while waiting. This observation
is asynchronous; an empty queue is not proof of continued device liveness.
Generic `poll_audio` continues to report decoder errors only.

## Verification on 2026-10-10

[Machine-readable evidence](evidence/native-window-player-2026-10-10.json)
records source/log identities and the full process results.

- 43 current scoped release tests pass: 18 audio runtime, 17 game runtime and
  eight player tests. Ten legacy audio compatibility tests also pass. Tests cover
  master cursor continuity, individually paused sources, terminal suspend
  failure, retryable suspended ticks, 4,096 cursor samples through the actual
  `InputRuntime`, button ordering, bounded recovery and rational clock behavior.
- Release all-target Clippy passes for the optional window/audio/game graph and
  separately for default and CPU-input player configurations. Forty tools tests,
  fmt, generated conventions, actionlint and the explicit normal dependency
  guard pass. No external package resolutions changed: Tao was already pinned.
- CI adds optional-window release Clippy and eight policy tests on the existing
  desktop jobs; `native-window` includes `render-gpu`. These are compile/unit
  checks, not hosted interactive window execution.

`tools/probes/native-window.py FIXTURE_PROOF OUTPUT` creates a fresh owned
shipping target from the prior completed `native-game.py --gpu` fixture ledger.
It audits actual target rustc invocations, strips/packages matching symbols,
rejects six dynamic JavaScript parsing routes, checks source-compiler symbols
and deletes both script-source and build directories before launching. It adds
four process cases and does not replace the prior 28-case offscreen proof.

The final local executable is **8,104,464 bytes**, SHA-256
`6b6a53a1ee86fabc9b2c98c7dd4601534bd72521c828bd55c779dbafbe5b3560`.
The host is Apple M5 Pro / Metal, macOS 26.6 (25G72), Rust 1.99.0,
Tao 0.37.1, Kira 0.12.5, CPAL 0.18.2 and wgpu 29.0.4.

Both static and streamed cooked fixtures accepted and presented 40 frames,
observed an actual OS resize and minimize/restore, and froze the world at tick
16 while minimized. Both explicitly recreated the real surface once and
configured it three times. The static run discarded five wall ticks; the
streamed run discarded zero. Neither observed surface timeout or OS-generated
loss. Explicit recreation plus synthetic retry tests do **not** establish that
an actual OS `Lost` event was reproduced. Unsupported gamepad actions and a
malformed camera ID were refused in separate processes.

The same final executable also ran the old 120-tick offscreen fixture: its PCM
hash, committed world/physics poses and initial/final GPU readback reports
exactly matched the frozen prior player. This specifically checks that the new
master-track routing preserves unsuspended behavior.

Raw local artifacts are in `/tmp/incant-native-window-shipping-final`; source
and build storage are absent. `on_screen: true` records actual surface submission
and the presentation API call, not scanout measurement or appearance approval.
Astra did not capture or inspect window pixels. Visible acceptance is assigned
to verified Claude Opus 5.5 Max in
[handoff 0037](../../handoffs/0037-native-player-window/brief.md).

## Integration with the renderer prerequisite

The branch integrates PR62 head `d8d00c3fbdf930a2ceea1e376088157e048fb0e4`.
Only the status ledger conflicted; the reviewed window/audio/game lifecycle
source is unchanged. The combined native host/adapter suite passes 62 ordinary
release tests and three actual GPU host regressions, including device loss and
presentation failure. Release all-target Clippy, 40 tools tests, fmt,
conventions, actionlint and the normal dependency guard pass.

The fresh window executable above and Claude0037 packet remain pinned to
`f68d910f45b8d311e93cc84bcacf8714ad71e250`; they are historical artifact evidence,
not a claim that the executable was rebuilt from the later integrated head.

## Running and remaining scope

```sh
./tools/cargo build -p incant_game_player --features native-window --release --locked -j2
target/release/incant_game_player --window SCENE AUDIO_CACHE CLIPS ACTIONS MODEL_CACHE TEXTURE_CACHE CAMERA_HEX
```

`--stop-after TICKS` bounds an interactive run to 1–36,000 ticks. The optional
final `--lifecycle-probe` runs the numerical 40-tick resize/minimize/recreation
exercise and prints its bounded report only on exit. The player still requires
matching trusted build-produced bytecode; it never accepts arbitrary bytecode
files or parses script source at startup.

The real window/device proof is local macOS with the existing v4 model fixture.
It does not claim v5 sprite window coverage, Linux/Windows interactive execution,
mobile support, audible speaker verification, a surface/device reset guarantee,
shipping installers/signing, phone performance, full renderer migration, pixel
placement completion or any phase gate. PDB source-symbol inspection is
explicitly unsupported by this helper; Windows compile coverage remains in CI.
