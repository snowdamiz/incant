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

## Shared agent project access — locally verified

The agent loop can share the editor command bus without holding its mutex during
provider requests, streaming, approvals, asset cooking or perception. The common
mutation path retains revision checks, provenance and undo. Native Play validates
the captured revision after immutable startup and before candidate ticks, so a
concurrent script write cannot publish a mixed scene/source revision. All 47 agent
tests, five native agent Play cases and scoped Clippy pass; independent review
accepted the corrected source-race boundary. [Evidence](docs/spikes/shared-agent-project-access.md).
This is backend work; chat-panel wiring, visual approval/history and budget UI
remain outstanding. It does not complete Phase 3 or a live-provider gate.

## CI cadence change — 2026-10-11

The director authorized local tests and a fast hosted development gate for main,
with comprehensive cross-platform CI on one frozen release checkpoint approximately
every three hours. [ADR 0014](docs/adr/0014-ci.md) defines integration, cache and
failure handling. The integrated policy passes 138 Python, 12 Node and 536 UI tests, strict types/build,
formatting, generated checks and actionlint; [evidence](docs/spikes/release-checkpoints.md).
PR83 merged after its exact-head fast check passed: 2s queue and119s execution. Main requires that fast check; release was bootstrapped at fully checked main82 and now requires all 23 full contexts, including local-service protocols. Both enforce administrators and prohibit force pushes/deletion. The three-hour heartbeat is active; the first two checkpoints remain unpromoted after diagnosed failures. The historical missing-Wasm-target failure remains recorded separately.
No phase, live-provider or physical-device performance gate is waived.

The initial frozen checkpoint PR85 found a Windows-only journal test lock violation. A reviewed test-only lifetime correction passes all six scoped cases and Clippy locally; Windows confirmation and complete release promotion remain pending. [Failure and repair evidence](docs/spikes/script-journal-test-portability.md).

The second frozen checkpoint includes the Windows journal repair and all 23 release
contexts. Its local-service job exposed a Linux private S3 config UID mismatch;
the isolated repair passes the real local protocol/restart proof while preserving
private permissions and the non-root server. The checkpoint also found macOS
Python process-observation and Windows native TypeScript startup failures. All eight workflows are now terminal: 20 required contexts passed and these three failed.
The combined reviewed repairs pass 199 Python tests (four Windows-only skips),
17 Node tests and scoped native compiler checks. All failures remain recorded;
no failed checkpoint is promoted. PR89 merged after its 126-second fast check;
checkpoint 3 is frozen at repaired main89 with all 23 contexts retained.
[Combined evidence](docs/spikes/checkpoint-2-portability.md).
[Service correction evidence](docs/spikes/local-service-foundation.md#linux-private-bootstrap-correction).

Checkpoint 3 passed the Linux service repair and macOS Python tooling step, then
Windows exposed a separate restricted TypeScript-host path lookup mismatch.
The reviewed correction normalizes only compiler callback keys, preserves strict
source/library confinement and passes actual TypeScript drive/UNC regressions,
19 Node tests, six actual native compiler/agent tests and the source RPC proof.
The failed candidate remains frozen; corrected Windows execution is still
required before release. [Evidence](docs/spikes/windows-native-compiler-entry.md#checkpoint-3-restricted-typescript-host-lookup).

## Authored source and native package — locally integrated

A public host-equals-target exporter now cooks authored Script attachments into a
fresh shipping player and copies only referenced native assets. Three fresh
source-deleted runs reproduce all expected motion; startup tampering is refused.
Source transactions and the shared compiler/exporter now integrate actual main87.
The public acceptance probe creates/writes/attaches, plays, saves, reopens and
exports through the real interfaces, then reproduces motion after deleting its
private authoring fixture. 786 workspace, 75 GPU, 15 optional window/project,
178 Python (two Windows skips), 17 Node and 536 UI tests pass, with Clippy,
build/generation and shipping audits. Independent review removed a resurrected
old workflow and duplicate tests. [Evidence](docs/spikes/native-project-package.md).
Existing full checkpoint jobs now include the export probe; hosted execution,
window package execution and complete UI/export/device gates remain open.

## Current integration and parallel work

The closed retained-state / native SDK / shared Play-startup union passes 837
ordinary and 75 serialized GPU tests, full Clippy, 536 UI tests and the fresh
four-player shipping proof. Audio and navigation preparation now precede all
publication, with a combined atomic-refusal regression. World-save v2 remains
world-only; solver/audio/VM continuation and editor controls remain open.
The repaired main89 integration also passes two actual compiler cases, 51
Play/headless tests, scoped Clippy, the 32-request public source proof and 199
Python tests (four Windows-only skips).
[Integrated evidence](docs/spikes/native-state-play-union.md).

- Checkpoint 2 Windows native TypeScript startup has a reviewed local path adapter
  correction: fixed engine-relative entry and canonical-equivalent ordinary launch
  cwd; the scrubbed environment remains unchanged. Nine local tests and scoped
  Clippy pass; actual Windows helper/workspace execution remains required.
  [Diagnosis and evidence](docs/spikes/windows-native-compiler-entry.md).

| Workstream | Current result | Next work / boundary |
| --- | --- | --- |
| Editor and sprite authoring | [PR #61](https://github.com/snowdamiz/incant/pull/61) combines the final plain UI, typed atlas/audio imports, focus fixes and the corrected sprite/atlas baseline. 524 UI tests, strict TypeScript, production and native builds pass. | Merged after exact-head hosted checks and isolated native Claude acceptance. [Evidence](docs/spikes/editor-release-union.md). |
| Local service foundation | Digest-pinned Postgres/S3-compatible storage/Zitadel/SMTP capture with private generated state and loopback-only access passes 28 actual local protocol checks plus four restart-persistence checks after integration with actual main84. Private state-home refusal and complete SMTP body verification are covered by 16 scoped tests; all 154 Python and 17 Node tooling tests pass. | Independent scoped review accepted the foundation; checkpoint 2 exposed the Linux private-config ownership issue. Its reviewed correction passes the actual local protocol and restart controls. The candidate-only `local-service-protocols` context is now required among 23 checks; checkpoint 3 has passed all 28 protocol and four restart/persistence controls on hosted Linux AMD64. Accounts/player services, PITR, other local substitutes and real provider/device gates remain. [Evidence](docs/spikes/local-service-foundation.md). |
| Script-source transactions | Shared `write_script` combines one bounded `.ts` content edit with registration and attachment, with exact-preimage Undo/Redo, actor provenance and recoverable Prepare/Commit/Abort history. Registered-ID read/write tools and trusted editor/headless adapters use the same bus. | Locally: 145 scoped tests, Clippy, 59 generated contracts and 32 real RPC requests across two processes pass, including native motion and crash recovery. Source/compiler/package integration now passes the actual recovery guard and source-deleted shipping proof; existing three-OS checkpoint CI will collect platform execution evidence. Multi-file authoring, reconciliation UI, a script editor and complete agent-created gameplay remain open. [Evidence](docs/spikes/script-source-transactions.md). |
| Import completion and Undo | Native imports return an exact revision/transaction receipt; unchanged imports preserve history and redo without offering stale Undo. Claude 0038 accepted the browser completion copy on frozen source. | The original PLAN revision 8/import union is included in merged PR76 and PR80 after their full applicable checks passed; 534 UI tests and the scoped native receipt/Undo checks pass. [Evidence](docs/spikes/editor-import-receipts.md). |
| Native TypeScript cooking | Project-attached sources now cook once into grouped native behaviors with frozen per-entity props, scoped timers and atomic whole-tick rollback. 730 workspace tests, 98 Python/17 Node/536 UI tests and a 16-process source-deleted embedding proof pass locally. | Reviewed local implementation; editor controls, source editing, complete SDK/save/export integration and hosted/device gates remain. [Attached scripts](docs/spikes/native-project-scripts.md), [embedding proof](docs/spikes/native-project-script-embedding.md). |
| Native Play RPC and agent | Cooked-native retained RPC and bounded agent play/query passed twelve source-deleted developer processes and a real saved-OAuth Astra turn. The new local attached-script route passes actual fixed-Node dispatcher tests, public CLI motion checks and a saved-OAuth provider run with native play/query. | Base capability is merged through PR82 after all21 required checks passed. Attached-script changes stay local for combined integration; editor controls, gameplay capture and complete session saves remain. [RPC](docs/spikes/native-play-rpc.md), [agent evidence](docs/spikes/native-agent-play.md). |
| Native game and rendering | Input, embedded QuickJS, native physics, scene audio and retained GPU submission compose through one accepted-tick lifecycle. Binary v5 sprites share immutable geometry. | [PR #62](https://github.com/snowdamiz/incant/pull/62) is merged after all 20 checks. Native editor/legacy Play migration, complete SDK, shader pipeline work and device budgets remain. |
| Native window player | Optional cooked native game window has bounded recovery, keyboard/mouse input, suspension and device audio. Source-free lifecycle checks and Claude0037 corrected native acceptance pass. [Evidence](docs/spikes/native-window-player.md). | PR63 is included in merged [PR80](https://github.com/snowdamiz/incant/pull/80), whose 21 applicable checks passed. Paused refresh preserves gameplay/audio; fullscreen-route limitations remain explicit. No phone or frame-budget claim. |
| Native tilemaps | Typed v6 data and exact solid-cell collision preparation preserve the reviewed [PR68](https://github.com/snowdamiz/incant/pull/68) ancestry in the consolidated union. Retained GPU tile rendering now shares checked atlas versions, painter ordering and combined sprite/tile resource caps; 45 ordinary and 62 GPU tests pass. | Claude0039 accepted scoped tile appearance; PR68/73 are included in merged PR80 with passing hosted checks. Native solid-map collision is included through the planar physics boundary below; legacy Play remains unsupported. [Data](docs/spikes/native-tilemaps.md), [rendering](docs/spikes/native-tilemap-rendering.md). |
| Native planar physics | Retained Rapier2D, cooked v7, typed native commands and solid-map collision pass 585 workspace tests, 66 GPU tests, 18 native/browser parity streams and seven fresh shipping processes. | PR78 is included in merged PR80; hosted cross-desktop 2D parity and all applicable checks passed. Root/unit-scale planar bodies only; physics saves, 2D character/joints, full script SDK and phone/performance gates remain open. [Evidence](docs/spikes/native-physics-2d.md). |
| Native flipbooks | Local v8 playback and its additive QuickJS/Wasmtime/wasm2c bulk SDK are independently reviewed. The frozen SDK passes 687 workspace and 158 combined conformance tests plus a fresh 64-success/43-refusal shipping proof. | The corrected FBX/prewarming/flipbook baseline is merged through PR81 after all21 hosted checks. The SDK and shared shipping union are published in PR83 with integrated local verification; the policy transition and corrected setup merged through PR83 after the fast main gate passed. Full release-checkpoint validation remains pending. The generated public Rust SDK is locally integrated below; complete gameplay SDK, save and device/performance gates remain. [Playback](docs/spikes/native-flipbooks.md), [SDK evidence](docs/spikes/native-flipbook-sdk.md). |
| Static FBX import | Shared asset transactions accept a bounded static subset through pinned MIT ufbx; existing cooked model/texture formats and stable reimport IDs are preserved. 71 scoped Rust and 531 UI tests, Clippy and 11 CLI process controls pass. | Claude0040 accepts static conversion/colour/reimport and the separate generated-tangent correction after 101 ordinary/61 numerical GPU tests and a bounded 12-capture delta. The corrected FBX/prewarming/flipbook union passes 685 ordinary/73 GPU tests and a fresh source-free player proof; all21 combined hosted checks passed in merged PR81. The separate OPAQUE/emissive/environment mip-filter correction is locally integrated below; combined checks remain pending. Browser source-model cooking explicitly refuses; actual browser dispatch and native fixture checks pass. Animation, skinning and full FBX compatibility remain unsupported. [Evidence](docs/spikes/static-fbx-import.md). |
| Color mip roles | Local INCMOD03 and Color manifest3 preserve alpha-independent OPAQUE/emissive/environment RGB while retaining sprite/MASK/BLEND coverage filtering. Integrated PR81 source passes 703 workspace ordinary tests, Clippy and 68 numerical renderer GPU tests; checked base sharing retains logical resource caps. Claude0043 accepted 13 bounded captures without a blocker. | The PR83 SDK/color union passes 739 ordinary and 75 serialized GPU tests plus the complete local four-player fresh proof; twenty old hosted checks passed, and the general setup failure is corrected locally. The exact new fast main gate passed; complete release checks remain a separate pending gate. Old immutable versions require source-backed reimport to adopt the fix. [Evidence](docs/spikes/color-mip-roles.md). |
| Native WebAssembly | Shared bounded capability ABI serves Wasmtime and wasm2c. PR #64 passed actual Windows /Od + /O2, Linux and macOS static conformance and fresh packaging. PR #67 composes static modules with the native game session. The versioned input ABI now drives real guest motion/spawns in both hosts and a fresh local packaged game proof. | The input/composed-player hosted checks passed in the merged runtime union. Complete SDK, static-module renderer/session saves and device gates remain. [Input ABI evidence](docs/spikes/native-wasm-input-abi.md). |
| CI speed | Registry-only caches have actual three-host consumers. The strict documentation selector and measured opt1 functional profile are merged through PR76/80; all applicable hosted checks passed with full shipping coverage. | Main79 saved all three native CI-profile dependency caches. These cold migration jobs do not establish a warm Windows speedup; next natural consumers provide that evidence. [Profile scope](docs/spikes/ci-test-profile.md). A single older matched local macOS sample saved 16.8%; the final SDK/color union now passes all four fresh proofs and selects the reviewed shared session in the macOS workflow. Its hosted correctness/latency remains pending; no Windows/device speed claim. Shipping audits and device gates are unchanged. [Proof](docs/spikes/shipping-fresh-session.md). |
| Generated Rust module SDK | Local public no_std Rust SDK and generated v1 host/guest/C registry bindings preserve existing capabilities. 167 actual-host conformance tests, 692 workspace tests, Clippy and a fresh 8-success/26-refusal shipping proof pass. | Independent review and combined 167-test actual-host conformance, real guest execution and fresh GPU/SDK player proof pass; hosted platform checks remain pending. No complete gameplay SDK, module build sandbox, save, device or performance gate. [API guide](sdk/rust/README.md), [evidence](docs/spikes/native-rust-sdk.md). |

The local npm cache follow-up preserves existing keys and download paths while
restricting saves to successful exact default-branch or frozen release-candidate
jobs. All 131 tooling tests and
workflow lint pass; hosted restore/version compatibility and retained-size
measurements remain pending. [Evidence](docs/spikes/npm-cache-producers.md).

Credential CI has a report-only locked dependency classifier and conservative
local-crate trigger coverage. Actual tests remain enabled; any selection policy
requires separate review. [Scope and limits](docs/spikes/credential-impact-reporting.md).

Pinned WABT provisioning now retries only bounded transient transport failures;
all URL, checksum and archive checks remain enforced. Five local fault controls
and all 95 tools tests pass; a real pinned macOS download succeeds. Hosted
verification remains separate. [Evidence](docs/spikes/wabt-download-retry.md).

Three Astra extra-high subagents own implementation/integration work in isolated
checkouts. Root reviews and integrates completed slices rather than waiting for
all workstreams to finish together. Claude sessions own editor/native appearance
review; concurrent native reviews coordinate foreground ownership.

## Merge coordination

One integration coordinator writes main. At most two independent, reviewed PRs
are published for checks at a time; parallel implementation stays local until
there is capacity. Each candidate integrates actual main before publication.
Completed candidates merge after their applicable checks pass and the prospective
main tree is verified against the tested source. Included source PRs close only
after their exact authored commits are verified in main. This avoids dependent
PR chains and repeated CI restarts for metadata-only updates.

PR76, PR79, PR80 and PR81 are merged. PR80 includes the exact PR63/68/73/78 source
history; PR81 preserves the corrected PR77 history, pipeline preparation and
flipbooks after all 21 checks passed. PR82 has since merged native TypeScript
cooking, RPC/agent Play and planar-save refusal after all21 checks. PR83 now
combines the reviewed SDK/color/shared shipping work with the director-authorized
CI cadence transition. Authored native Play is merged through PR84. PR86 adds local services and PR87 fixes the checkpoint-discovered Windows journal test. The source-transaction/package successor integrates actual main87 with passing local checks; navigation and retained world saves remain the next local consolidation slice.
Further work remains local; this queue does not waive any phase or device gate. [Verified ancestry and queue evidence](docs/spikes/foundation-integration-queue.md).

## Implemented boundaries and evidence

The native v1 save reader now rejects unsupported 2D solver state consistently
with its writer and in-place restore. A rehashed-payload regression fails before
the fix and passes afterward; full native session saves remain open.
[Evidence](docs/spikes/native-save-planar-refusal.md).

The reviewed native-assets/rendering union now preserves corrected FBX, pipeline
prewarming and flipbook source on actual main80. All 685 ordinary/73 GPU tests,
536 UI tests, 96 final tools tests, both workspace Clippy profiles, optional-window
checks, actual browser smoke and fresh source-free shipping controls pass. Final
combined hosted checks passed and PR81 is merged. [Evidence](docs/spikes/native-assets-render-union.md).

- **Pipeline preparation (merged through PR81).** The current finite renderer
  catalog is prepared before the first frame. All 601 ordinary and 67 actual GPU
  tests pass; material reimports across all current variants and four output
  formats allocate no additional pipelines or shader modules during drawing.
  A fresh source-free shipping proof passes eight GPU cases and 33 refusals.
  Claude 0041 accepted the scoped appearance. Combined flipbook source passes
  666 ordinary tests, two helper tests, 90 tools tests, release Clippy, all
  72 GPU cases and the fresh integrated shipping proof. All 21 combined PR81
  hosted checks passed. Offline backend
  shaders, persistent caches, recorded per-level permutations and phone hitch
  gates are still open. [Evidence](docs/spikes/prewarmed-render-pipelines.md).
  Claude also identified existing opaque-texture mip darkening; asset-cook
  diagnosis is in progress alongside the generated-tangent correction.

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
  Hosted input results passed in the merged runtime union; complete SDK and
  session/renderer/device evidence remain open. [Input ABI](docs/spikes/native-wasm-input-abi.md).
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
PR76 merged after all 21 applicable checks passed. [Union evidence](docs/spikes/native-runtime-integration.md).

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
runs and the fresh source-free GPU player also pass. PR80 merged after all 21
applicable checks passed; its actual-main tree equals the tested head. [Integrated evidence](docs/spikes/native-window-2d-union.md).

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

The private authored-script union now includes the reviewed PR83 policy without
changing its tested runtime/compiler/ABI inputs. Integrated verification includes
761 ordinary tests, 75 GPU tests, 178 actual-host conformance tests, full Clippy,
536 UI tests and 17 Node tests. The fresh bound-script embedding proof passed all16 copied-player processes after removing source/build storage. [Integrated evidence](docs/spikes/native-authored-sdk-integration.md).
Actual main83 is now integrated without a tree change; the authored candidate is ready for direct-main publication.
