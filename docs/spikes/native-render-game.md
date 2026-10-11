# Mandatory native render-input hosting

`incant_render_game` composes the committed native spatial adapter with the
explicit `HostedGameSession` / `HostedOfflineGameSession` lifecycle. One
`NativeRenderHost<C>` owns one checked `NativeRenderInput` and one trusted
`RenderInputConsumer`. Startup binds cooked model/texture versions and prepares
that consumer against the pending world's exact identity before session-owned
QuickJS/audio setup. The selected camera is an explicit binary stable ID.

Every accepted tick synchronizes the host after script, structural commit,
physics and audio, including offline PCM production. The consumer receives the
same retained typed input and cached camera label, with `Motion` or `Rebuilt`.
It cannot be omitted by the caller. Read-only host/consumer observations expose
the last accepted input; no mutable resource snapshot escapes this composition.

The default `CpuInputObserver` accepts typed CPU input only. GPU resource
validation, draw submission, device output and appearance are not implied.
A GPU consumer must preflight its resources, own/acquire its target internally,
and publish replacements only after its own acceptance. CPU input restores on
consumer error/unwind; external submitted audio/GPU effects cannot rewind.
The composite destroys all live state on any post-acceptance error/unwind and
clears offline PCM. Invalid input or an incorrect output buffer is retryable
and leaves the prior accepted state intact. Existing nonvisual constructors
remain strict for both spatial declarations and visual asset manifests.

## Verification

Seven release integration tests pass with real cooked glTF and PCM readers,
actual QuickJS, native physics and offline mixing: source/cache deletion,
matching simulation/physics/audio/render poses, retained slots/version identity,
reparent rebuilds, camera removal, input retry, script failure, startup binding
and consumer refusal, and late consumer error/unwind cleanup. A test consumer
observes actual calls and tests failure handling; it is not GPU/device evidence.
Release all-target Clippy and the resolved normal dependency guard pass.
The normal graph excludes authoring, importers and GPU. Test-only cooking and
script compiler features do not describe a shipping build.

Logs: `/tmp/incant-render-game-tests.log`,
`/tmp/incant-render-game-clippy.log`, `/tmp/incant-render-game-graph.log`.
The opt-in existing-player source-free proof is recorded below. No full
renderer/Play/pixel-placement migration, save support or performance gate is
claimed by this CPU composition.

## Source-free existing-player CPU mode

Build `incant_game_player` with `--features render-input` to enable the explicit
startup suffix `--cpu-render-input MODEL_CACHE TEXTURE_CACHE CAMERA_HEX` after
its existing eight arguments. The camera uses exactly 32 hexadecimal digits
encoding its 16-byte stable ID. The normal invocation still refuses spatial
scenes, including in this feature build; the default-feature binary rejects the
new mode explicitly and has no render crates in its normal dependency graph.
Build-time source embedding is unchanged: host must equal target, QuickJS is
pinned, and runtime source parsing is absent. This adds no arbitrary bytecode
loader or project path. CPU input metadata is reported once after all ticks;
there is no per-tick JSON projection or GPU dependency.

`tools/probes/native-game.py OUT --render-input` uses public command/import/cook
tools to produce real model + camera + physics + audio v4 scenes, then builds a
fresh shipping executable with embedded bytecode. It deletes the glTF/bin/WAV,
project/journal, JS and fresh Cargo storage before launching the retained package.
Eight successful 120-tick runs (static/streaming audio, 44,117/48,000 Hz, repeated
processes) agree exactly on final CPU/model/solver poses and PCM hashes. Sixteen
negative launches cover existing script/input/clip/format limits, nonvisual
player refusals, malformed/missing camera, missing model cache, and a late
selected-camera despawn distinct from the audio listener. Two plain v1 controls
and existing-output preservation bring the proof to 27 launches.

The actual `aarch64-apple-darwin` executable is 4,521,424 bytes. Its rustc commands
verify fat LTO, one codegen unit, optimization 3, panic abort, matched symbols and
portable CPU selection. Target normal dependencies exclude authoring/importers,
GPU and the compiler feature; compiler symbols/source comments are absent and
all six dynamic source-compilation routes reject. Default and optional normal
graphs were separately inspected, and a built default-feature executable refuses
both v4 input and the unavailable mode without output. This is desktop evidence;
no GPU submission, visible presentation, mobile execution, timing/performance or
full migration gate follows from it.

Integrated validation: 75 scoped release tests/doctests, both default and optional
player release Clippy, 40 tools tests, formatting, conventions, workflow lint and
dependency guards pass. The fixture-producing binaries are the already-tested
public tools from the native-audio-host integration; their hashes are recorded in
[the full fresh proof](evidence/native-render-game-shipping-2026-10-10.json).
Raw artifacts and logs remain in `/tmp/incant-render-game-shipping-02/` and
`/tmp/incant-render-game-{union-tests,union-clippy,shipping,tools}.log`.
