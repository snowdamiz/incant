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
  Zero/minimized and other out-of-range OS sizes defer work without terminating
  the session. Maximize/fullscreen controls are disabled where Tao supports it;
  OS constraints are not assumed to be infallible. Resizing invalidates the configuration.
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
- Keyboard, three-button mouse and mapped gamepad bindings are supported. Gestures,
  extra mouse buttons, captured-pointer mode, text/IME and haptics are outside
  this host. Unsupported declared action bindings fail at startup.

## Native desktop gamepads

The optional `native-window` feature pins Gilrs 0.11.2 / gilrs-core 0.6.8.
Default, CPU-input and offscreen-GPU players have no Gilrs normal dependency.
The shared `incant_input::gamepad` adapter has no OS dependency and keeps backend
identities out of script state. It assigns player IDs0–15 in discovery order,
retains a slot across a disconnect/reconnect recognized by the backend, and
refuses a seventeenth distinct backend identity during that player session.
This is a session identity, not a persistent hardware identifier or player-choice UI.
Gamepad bindings outside IDs0–15 fail explicitly at startup.

The window polls real Gilrs events during `MainEventsCleared`, including while
focus, minimization, surface acquisition or platform suspension freezes ticks.
Active connection/disconnection and button transitions retain their order in
the existing 1,024-entry input queue. A discarded focus/suspend batch reconciles
physical connection state against the last accepted tick; it cannot lose an
unpublished connect/disconnect or replay stale held controls. Resume emits current
connection differences before accepting a tick. Buttons and sticks stay neutral
until a fresh OS event, matching the existing keyboard focus policy. Catch-up
ticks consume a packet once, and connection publication commits only after a
successful game tick. Paused hotplug can collapse to its current topology because
no simulation or input tick runs in the paused interval.

Gilrs +Y-up sticks are inverted to the input contract's +Y-down. Its first trigger
pair maps to shoulders and its second to analog triggers. Only measured
`ButtonChanged` values become button input: an analog `ButtonPressed` threshold
event cannot fabricate a value of1.0. D-pad axes map to direction buttons with
the opposite direction released first. Unknown controls and Z axes have no
counterpart in the current normalized contract. Gilrs default jitter/dead-zone
filters are disabled; Incant action settings own dead zones. Each poll reads at
most1,024 raw events plus one overflow lookahead, including ignored events while
paused. Raw or combined normalized queue overflow terminates the host explicitly.

Returned backend initialization errors are terminal, including Gilrs's dummy
`NotImplemented` backend. Zero connected controllers is valid and leaves keyboard
and mouse usable. The pinned macOS backend starts its IOHID manager asynchronously;
manager creation failures are logged internally and not returned to the caller.
Successful construction or an empty poll is therefore not a liveness/availability
proof. The backend's OS/thread buffers are outside the adapter's queue bound.
Force feedback is disabled, no haptic API is connected, and haptics remain
unsupported. [Pinned primary API documentation](https://docs.rs/gilrs/0.11.2/gilrs/)
describes backend/platform support; physical-controller behavior is still unverified.

Linux window builds require `pkg-config` and `libudev-dev` as well as the existing
desktop packages; the desktop checkpoint workflow installs both. Access to evdev
devices depends on host permissions. macOS uses IOHID; Windows uses Gilrs's
default Windows Gaming Input backend, whose events require an associated focused
window. No accounts or build-time service become a runtime dependency.

Local macOS validation and exact source/backend/log hashes are recorded in
[gamepad evidence](evidence/native-window-gamepad-2026-10-11.json). Synthetic
normalized/backend events exercise the actual adapter and `InputRuntime`, not
physical hardware. No native window, GPU, account or controller was used for this
verification. Windows/Linux execution, physical-controller acceptance and the
Phase1 haptics/touch/device gates remain outstanding.

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

The corrected prerequisite `9cf171275debfd0cc959a636abf3db4c93865bfb` additionally carries
v5 executable proof coverage, separated Cargo metadata channels and the
parse-only ULID dependency/isolated physics lock fix. Seventeen affected player
and 2D release tests, native-window all-target release Clippy, 43 tools tests,
fmt, conventions, actionlint and the explicit normal graph pass after this
second integration. The window/audio/game lifecycle sources remain unchanged.

The next prerequisite `9c531a92854eab448053031a544579011bcc07a3` fixes Linux
strict-C11 trap declarations and isolates the existing Camera accessibility
scans. Integration passes 94 Camera tests, 43 tools tests, fmt and conventions.
The optional window source is unchanged; these checks do not replace the
prerequisite's combined Wasm conformance or pending exact-head hosted checks.

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

## Native review corrections

Claude Opus 5.5 Max completed [the original five native acceptance cases](https://github.com/snowdamiz/incant/blob/4073f8fab8b3a377230513d7cfa0c35b9f1db178/handoffs/0037-native-player-window/result.md)
on the frozen `6b6a53a1` executable. Its 180 evidence hashes are verified. The
review found an enabled fullscreen control that exceeded the bounded target and
terminated the session, and stale stretching when resizing a background window.

The correction disables maximize where supported and treats unsupported OS sizes
as recoverable suspension. `HostedGameSession::refresh_host` (also available on
the offline host) calls the existing owned host's sync with the current committed
world. It advances neither native tick nor input, script, physics or audio. Host
error/unwind drops the complete session. The window uses it only for pending
redraw/resize while visible and paused; a 50 ms throttle advances only at its
previous deadline, so a continuous event stream cannot postpone redraw forever.
Successful refreshes are counted separately as `presentation_refreshes`; total
`accepted_frames` includes both simulation presentations and these refreshes.

Thirty-six ordinary scoped release tests, release all-target Clippy and four
actual GPU regressions pass. Tests prove unchanged tick/input/poses, exact next
audio samples after repeated paused refreshes, terminal error/unwind, unchanged
GPU pixels, and bounded redraw opportunities under continuous 1 ms events.
Forty-three tools tests and generated conventions also pass.

The corrected fresh shipping executable is 8,120,976 bytes, SHA-256
`ec18e426c67584c4ac762cf72deda6dfc10d9044c4a5ff91a07fc4ad0648edf5`.
Its independently audited build/source directories were removed before the two
actual static/streamed lifecycle processes and two refusals. Both accepted 40
ticks with zero dropped wall ticks and zero presentation-only refreshes in that
particular automated probe. Static acquisition deferred once; the streamed case
never deferred. These timings/counts are observations, not performance gates.
Claude Opus 5.5 Max accepted both corrected cases in the [separate hash-pinned
delta review](https://github.com/snowdamiz/incant/blob/0dd843e08f2a39abd5a67d4914cda8b0b5880ea8/handoffs/0037-native-player-window/result-refresh.md).
All 49 new evidence files and the 180 unchanged original files verify. The
control was disabled/inert; paused aspect changes and a continuous resize burst
presented fresh output without advancing the world. The second run reported
897 ticks plus 16 presentation-only refreshes, exactly 913 accepted frames.
Both owned review processes closed normally. These are local acceptance
observations, not frame-time or scanout measurements.

The AX fullscreen attribute remains settable. macOS clamped the tested oversized
and undersized requests, so the recoverable out-of-range suspension path has
policy tests but no live native reproduction. Linux/Windows behavior and forced
fullscreen remain outside this acceptance. Original evidence and executables are
unchanged. Raw fresh proof: `/tmp/incant-native-window-refresh-shipping-02`.

The original review also observed one tick of motion on refocus (the existing
step-before-script input latency) and 0.64–0.68 seconds of application-audio tail
during macOS's blocking minimize animation. Focus-loss silence settled around
0.10 seconds. No synchronous audio stop during an OS-blocked event loop or
audible-speaker observation is claimed.
