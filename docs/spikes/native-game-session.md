# Native game session composition

`incant_game_runtime` composes the existing native input/VM/physics session with
retained cooked scene audio. It adds no authoring dependency to simulation, and
plain script, physics, and audio crates do not depend on the composition crate.
The separate `incant_game_player` uses the established build-produced bytecode
boundary; source compilation is a build dependency, not a shipped capability.

## Ownership and failure contract

The host owns one `NativeSession` and one `SceneAudio`. Callers receive read-only
world/input/source observations and may replace validated action bindings. There
is no mutable-world/mixer escape or composite save API. Clip bindings and backend
setup happen once at construction. Both spatial declarations and manifest-only
visual assets are refused before VM/backend construction; an actual renderer host
is still required. The standalone native audio example now refuses them too.

Each accepted tick advances input and native simulation, invokes the script, then
synchronizes audio from committed world poses. The offline variant renders its
assigned sample frames last. This preserves native simulation-before-script
ordering: a script velocity edit affects the following physics step, while a
script position edit is visible to audio in the current tick.

- Invalid input packets and incorrect offline buffer lengths are rejected before
  acceptance. World/VM/input clock and pending input edges stay retryable; the
  supplied output buffer is unchanged. An asynchronous device cursor is not frozen.
- After input is accepted, any simulation/script/audio preflight/apply/render
  failure destroys the entire live composite. Even standalone audio's retryable
  preflight rejection is terminal here because accepted simulation already ran.
  World/input/audio observations, rebinding and later ticks then fail explicitly.
- Offline accepted-tick failure clears the whole supplied buffer. A detected
  background decoder/backend error from `poll_audio` also destroys the composite
  between ticks. Device-driven users must poll while not ticking; asynchronous
  failures become observable when checked, not by magically rolling back sound.
- Device cursors and simulation are not rolled back. There is no claim of session
  restoration: VM, input, physics and audio cursors are not jointly saved.

The rational clock renders
`floor((tick + 1) * sample_rate / tick_rate) - floor(tick * sample_rate / tick_rate)`
stereo frames per tick, using checked clock advancement and `u128` arithmetic.
Rates outside 8,000–192,000 Hz are refused before backend setup. Cooked tick rates
are at most 240 Hz, so every supported tick has a nonzero frame count. Callers
must provide exactly twice that frame count in interleaved `f32` samples.

## Observable tests

Nine focused tests cover fractional clocks across four tick/sample-rate pairs;
invalid buffer and packet retry without losing pending edges; same-tick script
motion in actual static/streamed Kira output plus next-tick native physics state;
terminal script and numerical integration failures; terminal post-script audio
preflight; retained-file corruption during offline render; real background
Kira decoder failure between ticks; backend setup/drop ownership; and both forms
of spatial refusal before top-level guest code or backend setup. The exact dyadic
64 Hz physics test catches double integration without widening numerical tolerance.

The integrated scoped suite passed 146 release tests/doctests and all-target
Clippy. The full integrated workspace passed 504 release tests/doctests, all 56
serialized GPU regressions, and all-target release Clippy. The macOS native
device feature compiles; 40 Python tools tests, schema/SDK/bridge equivalence,
strict SDK TypeScript, formatting and conventions pass. The new normal dependency roots pass the runtime dependency guard; the
shipping probe separately checks actual target compile commands for authoring
crates and runtime compiler-feature leakage. Full workspace results and exact
log/source hashes are recorded in [machine evidence](evidence/native-game-session-2026-10-10.json).

## Fresh source-free shipping executable

`tools/probes/native-game.py` creates/imports a WAV and creates both static and
streamed audio/rigid-body scenes through the public command bus, then cooks them.
The existing authoring boundary has no tick-rate mutation command, so it preserves
the default 60 Hz. It builds a new native host==target shipping player with trusted
build-produced JavaScript bytecode and checks real rustc flags: opt-level 3, one
codegen unit, fat LTO, abort panic, debug information and platform split symbols.
The normal target graph contains no authoring crates. Bytecode reader/evaluator
symbols are present; source-parser symbols are absent. Six actual dynamic-source
routes throw in the packaged runtime. The source-only comment is absent from the
executable.

Before launching it, the probe deletes the project, WAV, journal, script source,
and fresh Cargo build storage. Binary scenes, fingerprinted cooked PCM/manifests,
the binary clip table and bounded startup recording/action JSON remain. Those
exchange JSON files are parsed once; they are neither cooked binary assets nor
session saves, and no tick serializes/project-validates state.

Eight fresh audio processes (two each for static/streaming at 44,117/48,000 Hz)
run 120 recorded ticks. The script sets velocity from the named action, native
kinematic physics advances once per following tick, and mixed stereo energy moves
from left to right. Each parameter combination repeats its entire observed state
and PCM hash exactly. Expected final x is 2.95; measured x is
2.9500136375427246 with a declared 0.001 tolerance for 60 Hz f32 physics. The
44,117 Hz case emits exactly 88,234 frames, exercising alternating per-tick frame
counts; 48,000 Hz emits 96,000. No equality is claimed across different rates or
static versus streamed resampling.

Eleven failing launches cover accepted script failure, recording clock mismatch,
invalid packet, wrong clip fingerprint, unknown scene version, oversized startup
actions, a forbidden manual-input route and both players' camera-only and
manifest-only version-4 refusals. Two ordinary version-1 controls emit silence.
One existing-output test preserves its original bytes. That is 22 player launches
in total; authoring tools and builds are separate. Simulation/host failures create
no output file. Output I/O itself is not atomic and can leave a partial PCM file.

Measured macOS arm64 player: **4,322,672 bytes**, SHA-256
`61b2cf1b366eb52042d2be2991fd1a50f3d46b55da85fc23723ffcb8f9e243d2`.
The genuine compiler-produced dSYM matches UUID
`96796659-c29f-3147-8f76-4d07553cf757`; DWARF sections/verification and the stripped
player identity were checked. Symbols and script metadata are retained separately
from the player. Whole-executable deltas are +1,436,688 bytes versus the previous
2,885,984-byte audio host and +380,856 bytes versus the 3,941,816-byte physics/script
player. These are different complete graphs, not an isolated feature cost or a
performance gate.

Reproduction after building the normal tool graph:

```sh
./tools/cargo build -j2 --release --locked \
  -p incant_headless -p incant_cook -p incant_audio_runtime -p incant_game_runtime \
  --features incant_audio_runtime/scene --bin incant_headless \
  --example cook_scene --example native_scene_audio --example no_renderer_fixtures
python3 tools/probes/native-game.py /tmp/incant-native-game-proof
```

The proof was executed on macOS arm64. The symbol audit currently explicitly
rejects MSVC/PDB; Linux execution and hosted integration of this new probe remain
unverified. Device output is compile-only, not a live sound-device result. There
is no renderer, game-session save/seek, native character/navigation migration,
portable script export, full legacy SDK migration or phone/performance gate claim.

PR #57 separately landed the reviewed native physics slice after all 17 exact-head
checks passed. Its merge was verified to preserve the tested tree and author
trailer; that does not substitute for this composition's own checks or approve a
phase gate.
