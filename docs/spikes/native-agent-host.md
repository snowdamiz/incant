# Shared native agent Play host

`incant_agent_native` moves the existing headless native Play controller, session,
agent host and lazy authored viewport capture into one authoring-only library.
Headless CLI, retained RPC, agent and evaluation callers use this library. An
editor service can reuse it without importing a binary crate or copying a second
simulation loop. This change does not connect editor controls or add agent tools.

## Ownership and unchanged behavior

- `Session` prepares through the unchanged `incant_play::PreparedPlay`, then owns
  the existing offline native game or CPU spatial host. Simulation owns no project
  document, command bus, source text or borrowed authoring state.
- `Controller` owns start/step/selected-state/stop and the bounded agent run/query
  path. `Host<P>` forwards `EngineHost` to that controller and its supplied
  perception implementation. Both share the existing typed requests and errors.
- `GpuPerception` lazily initializes the same offscreen renderer. Its screenshot
  implementation is byte-identical to the prior headless implementation: it
  renders the supplied **authored project/camera snapshot**, not the retained
  native Play world. CPU agent Play does not initialize it.

The trusted constructor parameter is
`FallbackCompiler = fn() -> Result<CompiledScript, ScriptError>`.
Headless still passes its unchanged build-embedded loader. An editor may supply
its engine no-op. This is a Rust host parameter; no project or tool argument can
choose a process, compiler or source. Authored Script attachments always use the
shared fixed TypeScript cook and never silently fall back after a source failure.
The loader's artifact/compiler trust boundary is unchanged.

`run_checked` finishes immutable source/asset preparation, checks the cooperative
cancellation/deadline budget, invokes the caller's original-snapshot validator,
and checks again before any observation or guest tick. Only a completed bounded
run replaces the retained candidate. Rejected input, preparation, validation,
runtime error or a trusted fallback unwind leaves the prior candidate intact.
A completed false assertion stays inspectable. The existing 1 MiB request,
600-second/36,000-tick duration, 10-second cooperative host budget, selected-entity
bounds, JSON pointer allowlist, and original-revision reporting are preserved.
Startup/ticks remain non-preemptible inside each bounded native operation.

The new crate deliberately depends on authoring/agent/render adapters. It is
explicitly forbidden by the shipped normal-dependency guard. `incant_play` and
all runtime source/manifests are unchanged; its normal graph remains agent-free.
No third-party lock resolution changed.

## Local verification

On base `d50fb0b3a91ca27982ddcf1ce05fc8e2192e8f04`, 102 scoped CI-profile tests
passed across agent, the new host, headless and Play startup. This includes the
five moved actual native agent cases, three new fallback/candidate/source-failure
cases, and the unchanged eight CLI plus six RPC real-process tests. Two existing
GPU tests stayed ignored. Matching four-package all-target Clippy passed with
warnings denied. The dependency guard passed for runtime, script, game and the
GPU-feature player; seven guard unit tests passed. All 62 regenerated schema files
matched exactly, and formatting/conventions/generated SDK/bridge checks passed.
Independent Astra review found no blocker; all 14 reviewed input hashes match the
verified source. The first compile found a removed trait import still required
by the separate screenshot CLI; it was restored before the passing run.

[Exact source, review, log and command identities](evidence/native-agent-host.json).
The existing screenshot implementation was moved, not visually reviewed or
executed here. No new GPU, live provider, shipping, phone, performance or phase-gate
claim is made. Editor service/UI integration, native Play-frame perception and
complete session/VM persistence remain separate work.
