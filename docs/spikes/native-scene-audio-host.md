# Typed native scene audio host

The optional `incant_audio_runtime/scene` feature connects cooked native ECS audio
nodes to the retained PCM/Kira runtime. The core scene/world remains independent
of Kira. Plain PCM/mixer builds do not enable this dependency; runtime normal-edge
guards pass both with and without the feature, including workspace feature
unification. No document, command bus, importer or JSON projection enters sync.
The parent integration owns enabling `scene` consistently in the full workspace
CI commands and connecting this host to the ordinary game session/player.

`ClipBinding::load` resolves an immutable StableId plus the package's expected
fingerprint, validates the manifest and every PCM chunk, and retains its open
version. `SceneAudio::new` checks the entire bounded snapshot before creating a
backend. `sync` consumes committed native poses after successful world/script
publication. Listener orientation composes ancestor rotations without inherited
scale/shear. The host supports static/streamed playback, bus gains, source
pause/resume, gain/pan/rate, start/loop/replacement, removal and spatial motion.
Adding/removing/reparenting buses is explicitly unsupported after construction.
Clip bindings cannot be loaded or replaced during ticks.

The limits are 128 retained bindings, 4 MiB of retained metadata allocation
capacities, 128 voices including retired handles, 32 buses and one listener.
Static retained sample storage is bounded to 8,388,608 stereo frames (64 MiB).
Transient decoding copies, Kira queues and previous renderer-owned generations
add bounded storage; 64 MiB is not a total process peak bound. Metadata accounting
includes actual String/Vec capacities; container/Arc bookkeeping is separately
bounded by object counts. Scratch vectors are reused and steady sync allocates
no per-source strings or arrays. Structural source starts/replacements allocate
bounded handles and decoder storage. No allocation/time performance gate is
claimed.

Preflight failure is retryable and issues zero mixer commands; it cannot freeze
an asynchronous device cursor. The desired static set fitting its budget does
not guarantee that a previous renderer-owned allocation has already been freed.
Actual decode/backend/apply/render failure is terminal and drops the mixer;
subsequent calls fail. Offline render failure clears the complete caller buffer.
A game session must stop after an unsuccessful post-simulation sync rather than
claim world rollback or a successful tick. The host does not serialize cursors.

Only the host forgets its voice handles: private owned playback never performs
standalone Mixer's automatic stopped-voice cleanup. This prevents an asynchronous
completion between inspection and another source's play from invalidating a
cached ID. A stopped handle is checked again for decoder errors before release.
Natural one-shots stay stopped until an explicit restart edge. Replacements wait
for renderer acknowledgement and actual Kira arena capacity; a later sync starts
them. Sync alone never advances a cursor. Existing public Mixer behavior remains
unchanged. Native background streaming is exercised without opening a device;
WASM background streaming fails explicitly, while the synchronous offline path
is available. No actual browser/device playback is claimed.

## Observable checks

The final scoped union command passed 92 release tests/doctests across audio,
authoring audio, runtime and cook, with `incant_audio_runtime/scene` enabled.
The final added test then exercised the actual Kira background decoder error
channel (instead of the synchronous offline adapter): the audio suite passed all
17 tests, including 15 new cases. The union and final audio all-target Clippy
commands passed with warnings denied. `scene,device` native and `scene` WASM
compilation passed; these are compile checks, not device/browser execution.

Behavior coverage includes cooked V3 loading, stationary sample batch invariance,
pause/resume cursor preservation and parameter changes against direct Mixer,
one-shot/restart/removal lifecycle, whole-snapshot rejection before any earlier
bus update, fixed-topology rejection, all checks before backend creation,
corruption silencing/terminal failure, spatial hierarchy/listener rotation,
128-voice replacement/reclamation, retained fingerprint identity, metadata budget,
clock rewind, failed render, owned-handle lifetime and late decode errors. The
late-error regression was run against the preceding implementation and failed
before the fix; it proves errors are not silently dropped when a stopped handle
is forgotten.

## Fresh shipping execution

`python3 tools/probes/native-scene-audio.py artifacts/native-scene-audio-final`
passed 16 fresh processes on Apple arm64 using actual cooked V3 scenes, retained
PCM and the packaged native audio example. Public import and command-backed
authoring created four scenes (static/stream × rates 0.5/1.7); source project,
WAV, journal and fresh build storage were deleted before execution. Runtime
binary scenes, the bounded binary ID/fingerprint table and cooked PCM/manifests
are intentionally retained. Each process executes 120 native fixed ticks and
renders 96,000 frames. Moving parent poses cross from left to right, with measured
stereo energy reversal. Two batch sizes and repeated processes are checked;
identical-batch repeats are bit-exact. Moving parameter transitions are not
claimed bit-identical across different render block sizes.

The player is 2,885,984 bytes, SHA256
`548408dd09e4fc41c8ca405f9a740c3b7f59da82510ba9d6ccb9c8e1151c2b7b`, with matching
arm64 dSYM UUID `7d8f1f32-945a-3c44-9697-0831287d641e`. Real compiler invocations
verify opt-level 3, one codegen unit, abort panic and fat LTO on the executable.
The size increase over the 585,072-byte standalone cooked mixer is 2,300,912 bytes;
this includes the native world/physics dependency graph and is not isolated audio
cost or a phone performance result. Symbols are packaged separately.

After that build, only the example header comment, an additional test and proof
tool UTF-8/output-semantics wording changed. Runtime implementation sources did
not change. Evidence records the built example's original hash separately from
the final commented file; the reported binary is the actual tested artifact.
Simulation/host failures stop before output creation. Output file I/O is not
atomic and may leave a partial PCM proof file after a write failure. The example
uses create-new output and rejects malformed bindings/scenes and existing output.
This is a disposable proof artifact, not an application export transaction.

[Machine evidence](evidence/native-scene-audio-host-2026-10-10.json) records source
and check-log hashes and the complete fresh-process result. Full workspace checks
with the optional feature, normal native VM/input/audio session integration,
audio cursor saves, device/browser audio execution and reference-phone gates
remain open. No workflow edits or publication are part of this host commit.

## Workspace integration

The parent enables `incant_audio_runtime/scene` consistently in ordinary workspace
Clippy, release tests, ignored GPU tests and the combined editor/tool builds. The
GPU pass therefore retains the same feature graph as the preceding tests. The
dependency guard accepts explicit Cargo features and checks this optional host's
resolved normal edges. The macOS engine job also runs and uploads the fresh
source-free native audio proof. Hosted and integrated local checks remain pending
at this wiring commit; the scoped results above keep their original identity.
