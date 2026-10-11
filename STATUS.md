# Implementation status

[PLAN.md](PLAN.md) revision 7 is the source of truth. Phase 1 is in progress;
the full engine and Driftwake release are not complete. Calendar targets and
phase durations have been removed. Work proceeds continuously and in parallel
where dependencies allow; exit gates still require their stated evidence.

The director authorized Phase 1 with remaining Phase 0 items deferred, sensible
engineering progress without repeated advancement questions, and agent merges
after scoped review and passing checks. These authorizations do not approve a
phase gate. Section 12 decisions are settled. Current computer use and screen
capture are authorized; Claude Opus 5.5 with explicitly verified Max effort owns
visual work, and Astra owns correctness and integration.

## Current integration and parallel work

| Workstream | Current result | Next work / boundary |
| --- | --- | --- |
| Editor and sprite authoring | [PR #61](https://github.com/snowdamiz/incant/pull/61) combines the final plain UI, typed atlas/audio imports, focus fixes and the corrected sprite/atlas baseline. 524 UI tests, strict TypeScript, production and native builds pass. | Exact-head hosted checks and isolated native Claude acceptance are running. [Evidence](docs/spikes/editor-release-union.md). |
| Native game and rendering | Input, embedded QuickJS, native physics, scene audio and retained GPU submission compose through one accepted-tick lifecycle. Binary v5 sprites share immutable geometry. | Root integration PR is being prepared. Native editor/legacy Play migration, complete SDK, shader pipeline work and device budgets remain. |
| Native window player | A separate implementation branch adds real surface presentation, keyboard/mouse input, suspension and device audio. Local source-free lifecycle tests pass. | Independent review/CI and Claude native acceptance precede integration. No phone or frame-budget claim. |
| Native WebAssembly | Shared bounded capability ABI serves development Wasmtime and compiled wasm2c modules; local conformance and a source-free macOS executable pass. | MSVC /Od and /O2 configurations are prepared for hosted conformance; actual Windows execution, full SDK/game composition and device evidence remain pending. |
| CI speed | Registry-only shared auxiliary caches are merged; the agent is measuring actual warm consumers and completed job timings. | Shared fresh-build storage and a lower-optimization CI test profile are experiments, not adopted speed claims. Independent shipping audits remain intact. |

Three Astra extra-high subagents own implementation/integration work in isolated
checkouts. Root reviews and integrates completed slices rather than waiting for
all workstreams to finish together. Claude sessions own editor/native appearance
review; concurrent native reviews coordinate foreground ownership.

## Implemented boundaries and evidence

- **Authoring and accounts.** Typed canonical JSON and stable ULIDs, atomic shared
  commands, provenance, undo/redo and durable recovery; CRDT round-trip and scale
  investigations; editor/CLI OpenAI tools and a 19/20 live evaluation. macOS OAuth,
  refresh and persistence across rebuilt applications are verified. The director's
  private-file credential-store choice avoids repeated Keychain prompts. The
  editor remains usable offline. [Foundation](docs/spikes/phase-0.md),
  [login evidence](docs/spikes/auth-login-repair.md).
- **Assets.** glTF/GLB, PNG/JPEG/EXR and WAV/OGG import/cook with retained checked
  caches, reimport and source-independent loading. Stable cooked atlas frame IDs,
  packing, nine-slice and authoring validation are implemented. Native readers
  are below the authoring boundary. [Asset pipeline](docs/spikes/asset-pipeline.md),
  [atlas work](docs/spikes/sprite-atlases.md),
  [reader boundary](docs/spikes/cooked-visual-assets-boundary.md).
- **Native world and scripts.** Disposable cooked binary ECS scenes, typed bulk
  Transform/Velocity access, bounded structural command buffers, native input and
  physics, explicit observation, and versioned native world saves. Build-produced
  QuickJS bytecode excludes the source compiler from the normal shipped graph.
  Legacy document-backed Play still exists and is not the new runtime path.
  [Native script columns](docs/spikes/native-script-columns.md),
  [structural commands](docs/spikes/native-script-structural.md),
  [bytecode](docs/spikes/cooked-native-scripts.md),
  [world saves](docs/spikes/native-world-saves.md),
  [input](docs/spikes/native-input-session.md),
  [physics](docs/spikes/native-physics-runtime.md).
- **Native rendering.** Canonical typed renderer input and a separate authoring
  adapter; exact retained asset versions; binary v4 cameras/models/lights and
  binary v5 sprites; direct committed pose updates and mandatory GPU host sync.
  Numeric moving-frame regressions cover source/cache removal, reflection,
  replacement and rejection/retry. Sprite CPU staging is allocation-free in the
  measured test; complete encoding still allocates. Shared immutable sprite
  streams reduce the calculated geometry cap from 576 MiB to 232 MiB and moving
  uploads from 64 to 12 bytes/vertex, excluding textures/metadata/driver storage.
  [Renderer boundary](docs/spikes/native-renderer-boundary.md),
  [retained frames](docs/spikes/native-retained-render-frames.md),
  [native sprites](docs/spikes/native-sprites.md),
  [memory evidence](docs/spikes/retained-sprite-memory.md).
- **Native game and audio.** Retained cooked clips, buses, spatial playback and
  streaming compose with simulation, input, scripts and GPU publication. Invalid
  preflight input is retryable; failure after acceptance destroys the composite
  state. A fresh 7,404,688-byte macOS shipping executable passes 28 process
  invocations after source/project/build removal, with exact repeated PCM/pose
  and numerical GPU hashes. That fixture exercises v4 models; v5 sprite process
  coverage is a separate follow-up. [Composition](docs/spikes/native-game-session.md),
  [audio host](docs/spikes/native-scene-audio-host.md),
  [GPU proof](docs/spikes/native-game-gpu-player.md).
- **WebAssembly.** Optional Wasmtime 49.0.2 development host and a statically
  compiled WABT 1.0.42/wasm2c path share copied numeric/structural capabilities.
  C traps remain in C, fuel/storage/imports are bounded, and accepted failures
  terminate the module. A fresh 2,780,816-byte macOS executable passes three
  exact-motion processes and 18 refusals after temporary sources/build removal.
  Actual iOS C/Rust compilation is compile-only. Initial static memory/table
  growth remains unsupported. A native x64 MSVC /Od baseline and /O2 candidate
  now have conformance/proof wiring, but actual Windows execution remains pending.
  [Development host](docs/spikes/development-wasm-host.md),
  [static host](docs/spikes/shipping-wasm2c.md),
  [MSVC follow-up](docs/spikes/wasm2c-msvc.md).

Local integration checks: 543 ordinary Rust release tests/doctests; 138 scoped
static/development-Wasm/runtime/script tests; workspace and optional-host release
Clippy; 524 UI tests, strict TypeScript and production build; 40 Python tool tests.
The renderer's unchanged frozen implementation separately passed all 66 actual
GPU tests. Exact sources, scopes and logs are retained in the linked reports;
these local results do not replace pending hosted checks or device gates.

## Remaining engineering requirements

- Complete migration of the editor, headless runner and full gameplay SDK from
  legacy document-backed Play into the native runtime. Persistent script state,
  timers/coroutines, native physics/navigation/audio/UI access and full composite
  session restoration need production coverage. Do not extend the legacy tick
  path or add project cloning/JSON/authoring validation to native ticks.
- Finish render features and their native path: offline backend shader cooking,
  persistent pipeline caches, asynchronous fallback/prewarming, temporal upscaling,
  dynamic resolution, backend interop and recorded device budgets. Runtime WGSL
  compilation is still present; source-free script proofs do not certify shader
  performance. Native pixel placement, tilemaps/flipbooks and Rapier 2D remain.
- Complete animation, navigation, game UI/text/IME, localization and remaining
  input/haptics/native SDK deliverables. Existing scoped/legacy implementations
  are not evidence of full native feature parity.
- Integrate Wasm gameplay modules into the complete runtime and export route;
  generate the Rust SDK, establish platform/device conformance and measure the
  settled TypeScript AOT/physics candidate adoption rules. Do not reopen Section
  12 decisions or substitute desktop timings for reference-phone results.
- Build and exercise Core Sample and the 2D sample on the required device classes,
  including hosted real iPhone/Pixel devices through AWS Device Farm, with actual
  device model/OS, shipping profile and matched baselines. Phone script p95,
  shader-stall, frame-time and memory gates remain open.
- Deliver the remaining editor/AI/content/export/network/services/game production
  and release scope in PLAN.md. Phase 1 progress is not Engine 1.0 or Driftwake
  release completion.

## Deferred gates and human-owned actions

The [Phase 0 ledger](docs/gates/phase0-review.md) remains unapproved: live
revocation verification with a suitable disposable connection, scheduled nightly
artifact history, staffing/reviewer/contracts and director approval remain open.
Do not revoke the director's working account for a test. Manual Windows/Linux
account checks and the Windows viewport review were replaced by CI under the
director's explicit decision; no extra desktop machine or live API key is requested.

Hosted device/service accounts, payments and human staffing/legal commitments
remain human-owned. Signing credentials and store/developer accounts first gate
Phase 5 store uploads. No phone purchases, name clearance or package reservations
are required. Unsigned packages and simulator/compile-only results do not satisfy
store distribution or real-device gates.

Earlier implementation notes, superseded counts and publication chronology are
preserved in the [historical ledger](docs/spikes/implementation-history-2026-10-10.md).
Technical evidence remains under [docs/spikes](docs/spikes); `python3 tools/check_gate.py`
reports the recorded gate state without granting approval.
