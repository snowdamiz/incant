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
The opt-in existing-player source-free proof is a separate follow-up. No full
renderer/Play/pixel-placement migration, save support or performance gate is
claimed by this CPU composition.
