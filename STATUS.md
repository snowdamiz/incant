# Implementation status

[PLAN.md](PLAN.md) revision 8 is the source of truth. Phase 1 is in progress;
the full engine and Driftwake release are not complete. Calendar targets and
phase durations have been removed. Work proceeds continuously and in parallel
where dependencies allow; exit gates still require their stated evidence.
Service-backed features use local containers or in-process substitutes during
development, behind production-shaped contracts. Real provider integrations,
security/device evidence and release gates remain real; local substitutes do not
satisfy those gates.

The director authorized Phase 1 with remaining Phase 0 items deferred, sensible
engineering progress without repeated advancement questions, and agent merges
after scoped review and passing checks. These authorizations do not approve a
phase gate. Section 12 decisions are settled. Current computer use and screen
capture are authorized; Claude Opus 5.5 with explicitly verified Max effort owns
visual work, and Astra owns correctness and integration.

## Current integration and parallel work

| Workstream | Current result | Next work / boundary |
| --- | --- | --- |
| Editor and sprite authoring | [PR #61](https://github.com/snowdamiz/incant/pull/61) combines the final plain UI, typed atlas/audio imports, focus fixes and the corrected sprite/atlas baseline. 524 UI tests, strict TypeScript, production and native builds pass. | Merged after exact-head hosted checks and isolated native Claude acceptance. [Evidence](docs/spikes/editor-release-union.md). |
| Local service foundation | Digest-pinned Postgres/S3-compatible storage/Zitadel/SMTP capture with private generated state and loopback-only access passes 28 live local protocol checks plus four restart-persistence checks. | Scoped review/hosted verification pending; accounts service, PITR, other local substitutes and real provider/device gates remain. [Evidence](docs/spikes/local-service-foundation.md). |
| Import completion and Undo | Native imports return an exact revision/transaction receipt; unchanged imports preserve history and redo without offering stale Undo. Claude 0038 accepted the browser completion copy on frozen source. | Union with runtime main and original PLAN revision 8 passes 534 UI tests, three native asset tests and scoped Clippy. Hosted verification pending. [Evidence](docs/spikes/editor-import-receipts.md). |
| Native game and rendering | Input, embedded QuickJS, native physics, scene audio and retained GPU submission compose through one accepted-tick lifecycle. Binary v5 sprites share immutable geometry. | [PR #62](https://github.com/snowdamiz/incant/pull/62) is merged after all 20 checks. Native editor/legacy Play migration, complete SDK, shader pipeline work and device budgets remain. |
| Native window player | Optional cooked native game window has bounded recovery, keyboard/mouse input, suspension and device audio. Source-free lifecycle checks and Claude0037 corrected native acceptance pass. [Evidence](docs/spikes/native-window-player.md). | The reviewed PR63 source is consolidated with native project play and 2D integration in one validation head. Paused refresh preserves gameplay/audio; hosted platform checks remain pending. No phone or frame-budget claim. |
| Native tilemaps | Typed v6 data and exact solid-cell collision preparation preserve the reviewed [PR68](https://github.com/snowdamiz/incant/pull/68) ancestry in the consolidated union. Retained GPU tile rendering now shares checked atlas versions, painter ordering and combined sprite/tile resource caps; 45 ordinary and 62 GPU tests pass. | Claude0039 accepted scoped tile appearance; the reviewed PR73 renderer now shares the consolidated validation head. Native solid-map collision is included through the planar physics boundary below; legacy Play remains unsupported. [Data](docs/spikes/native-tilemaps.md), [rendering](docs/spikes/native-tilemap-rendering.md). |
| Native planar physics | Retained Rapier2D, cooked v7, typed native commands and solid-map collision pass 585 workspace tests, 66 GPU tests, 18 native/browser parity streams and seven fresh shipping processes. | The reviewed PR78 implementation now shares the consolidated validation head; hosted cross-desktop 2D parity is pending. Root/unit-scale planar bodies only; physics saves, 2D character/joints, full script SDK and phone/performance gates remain open. [Evidence](docs/spikes/native-physics-2d.md). |
| Native WebAssembly | Shared bounded capability ABI serves Wasmtime and wasm2c. PR #64 passed actual Windows /Od + /O2, Linux and macOS static conformance and fresh packaging. PR #67 composes static modules with the native game session. The versioned input ABI now drives real guest motion/spawns in both hosts and a fresh local packaged game proof. | Complete SDK, static-module renderer/session saves, new input/composed-player hosted results and device gates remain. [Input ABI evidence](docs/spikes/native-wasm-input-abi.md). |
| CI speed | Registry-only caches have actual three-host consumers. A strict documentation selector and measured opt1 functional profile are prepared and locally validated; unchanged main64 coverage passes 543 ordinary/66 ignored tests and all four fresh shipping proofs. | The main67/import union also passes 544 ordinary, 534 UI and 86 tools tests plus workspace Clippy; hosted union checks and Windows/cache-size measurements remain pending. [Profile scope](docs/spikes/ci-test-profile.md); shared fresh-build storage remains off CI. Shipping audits and device gates are unchanged. |

Credential CI has a report-only locked dependency classifier and conservative
local-crate trigger coverage. Actual tests remain enabled; any selection policy
requires separate review. [Scope and limits](docs/spikes/credential-impact-reporting.md).

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
  [physics](docs/spikes/native-physics-runtime.md). Native scripts now have bounded
  committed-state raycasts, character sweeps and trigger events with revocable
  capabilities and whole-update rejection. 131 scoped tests and a 45-process
  source-free proof pass. [Queries](docs/spikes/native-script-queries.md),
  [character/trigger evidence](docs/spikes/native-script-character.md). Native fixed-tick
  timers now stage schedules alongside numeric/structural changes, with 63 script
  tests and a 48-process source-free proof. [Timer evidence](docs/spikes/native-script-timers.md). Native
  headless observation plans now check selected typed state and input actions at
  accepted ticks, return structured false-assertion reports/nonzero status, and
  preserve terminal runtime refusal. Seven focused and 577 ordinary workspace
  tests plus 54 fresh source-free process controls pass; existing project-oriented
  legacy Play and full Core Sample/agent integration remain open.
  [Observation evidence](docs/spikes/native-headless-observations.md).
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
  state. A fresh 7,404,688-byte macOS shipping executable passes 40 process
  invocations after source/project/build removal, with exact repeated PCM/pose
  and numerical GPU hashes. The fixtures exercise v4 models and v5 atlas sprites,
  including mirrored nine-slice geometry and missing-cache/unknown-frame refusal. [Composition](docs/spikes/native-game-session.md),
  [audio host](docs/spikes/native-scene-audio-host.md),
  [GPU proof](docs/spikes/native-game-gpu-player.md),
  [sprite executable proof](docs/spikes/native-sprite-player.md).
- **WebAssembly.** Optional Wasmtime 49.0.2 development host and a statically
  compiled WABT 1.0.42/wasm2c path share copied numeric/structural capabilities.
  C traps remain in C, fuel/storage/imports are bounded, and accepted failures
  terminate the module. A fresh 2,780,816-byte macOS executable passes three
  exact-motion processes and 18 refusals after temporary sources/build removal.
  Actual iOS C/Rust compilation is compile-only. Initial static memory/table
  growth remains unsupported. A native x64 MSVC /Od baseline and /O2 candidate
  passed actual hosted conformance and a fresh optimized PE/PDB proof on PR #64.
  Static modules also compose with native input acceptance, physics and offline
  scene audio. The new `incant_input_v1` bounded snapshot preserves raw controls,
  gestures and named actions; 102 combined tests/doctests and 34 fresh local
  process checks pass. The 3,933,488-byte packaged macOS player responds to delayed
  accepted input with delayed native spawning and retains the exact audio clock.
  Hosted input results, complete SDK and session/renderer/device evidence remain
  open. [Input ABI](docs/spikes/native-wasm-input-abi.md).
  [Static game session](docs/spikes/native-wasm-game-session.md).
  [Development host](docs/spikes/development-wasm-host.md),
  [static host](docs/spikes/shipping-wasm2c.md),
  [MSVC follow-up](docs/spikes/wasm2c-msvc.md).

Local integration checks: 570 ordinary Rust release tests/doctests; 138 scoped
static/development-Wasm/runtime/script tests; workspace and optional-host release
Clippy; 529 UI tests, strict TypeScript and production build; 43 Python tool tests.
The renderer's unchanged frozen implementation separately passed all 66 actual
GPU tests. Exact sources, scopes and logs are retained in the linked reports;
these local results do not replace pending hosted checks or device gates.
The first hosted runtime run exposed stderr/JSON mixing in the Wasm audit and a
stale isolated physics lockfile. Reviewed fixes pass real combined Wasm
conformance, browser-target physics compilation and six native parity streams.
[Correction evidence](docs/spikes/runtime-ci-corrections.md). A subsequent Linux
C11 trap-link failure now has a reproduced POSIX declaration fix; the Windows
Camera accessibility test is split into its six independent scans with unchanged
coverage and bounded asynchronous cleanup. Both corrections passed local and hosted checks before PR #62 merged.

Native script logs now publish only after accepted numeric/structural updates,
with primitive Unicode validation, fixed per-update limits, bounded retained
history and dropped-message counts. Native/game observers hide logs after
terminal failure; the source-free player reports them once at completion.
585 ordinary workspace tests and 47 fresh CPU-mode process controls pass.
[Evidence](docs/spikes/native-script-logs.md). This is the QuickJS path; Wasm logs,
full native SDK, session saves and editor/headless Play migration remain open.


The reviewed native script/input stack is now tested together with the static
module lifecycle and CI/import union: 593 ordinary tests, 97 actual composed-module
tests, all-target Clippy, 86 tooling tests and two fresh shipping proofs pass.
Hosted integration checks remain pending. [Union evidence](docs/spikes/native-runtime-integration.md).

Native project playback now cooks authoring once and runs a disposable native
world with a trusted build-embedded behavior. The developer coordinator serializes
build/copy, removes temporary compiler inputs, and binds each private executable
to its source digest. Replay, selected observations, committed logs and optional
PCM preserve authoring and publish no partial output on runtime failure. The
integrated source passes 601 ordinary tests, 88 tooling tests, six custom-behavior
process controls and a fresh 47-process shipping-player regression proof.
[Evidence](docs/spikes/native-project-play.md). Editor Play, agent play tools,
project script routing, GPU capture and complete session saves remain open.

## Consolidated native-window and 2D integration

The ancestry-preserving union of PR63/68/73/78 on PR79/76 passes 650 ordinary
workspace tests, all 69 ignored GPU tests, both workspace Clippy profiles,
18 optional window policy/routing tests, 534 UI tests and 90 tools tests.
Generated schemas remain unchanged. Static-module composition passes 101 tests
and a fresh 34-process shipping proof; all 18 native/browser physics streams
match, and seven fresh planar shipping processes pass. Six custom native-project
runs and the fresh source-free GPU player also pass. Exact-head hosted checks
remain pending. [Integrated evidence](docs/spikes/native-window-2d-union.md).

## Remaining engineering requirements

- Complete migration of the editor, headless runner and full gameplay SDK from
  legacy document-backed Play into the native runtime. Persistent script state,
  remaining timer/coroutine semantics, native physics/navigation/audio/UI access and full composite
  session restoration need production coverage. Do not extend the legacy tick
  path or add project cloning/JSON/authoring validation to native ticks.
- Finish render features and their native path: offline backend shader cooking,
  persistent pipeline caches, asynchronous fallback/prewarming, temporal upscaling,
  dynamic resolution, backend interop and recorded device budgets. Runtime WGSL
  compilation is still present; source-free script proofs do not certify shader
  performance. Native pixel placement, flipbooks and broader tilemap/2D-physics
  feature and device coverage remain.
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

Hosted device/service accounts first gate real staging or device work, not
local feature development. Payments and human staffing/legal commitments remain
human-owned. Signing credentials and store/developer accounts first gate
Phase 5 store uploads. No phone purchases, name clearance or package reservations
are required. Unsigned packages and simulator/compile-only results do not satisfy
store distribution or real-device gates.

Earlier implementation notes, superseded counts and publication chronology are
preserved in the [historical ledger](docs/spikes/implementation-history-2026-10-10.md).
Technical evidence remains under [docs/spikes](docs/spikes); `python3 tools/check_gate.py`
reports the recorded gate state without granting approval.

The reviewed general-check selector allows only nine audited plan/convention/ADR
files onto a Linux documentation path, preserving full checks for every other
change and all classification failures. Local82tool tests/actionlint pass; hosted
latency remains pending. [Policy and evidence](docs/spikes/ci-document-selection.md).
