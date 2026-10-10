# Implementation status

Source: [PLAN.md](PLAN.md), revision 3 (2026-10-09) with director decisions. Current phase:
**1, in progress**. The complete engine/game release is not implemented.
On 2026-10-08 the director authorized beginning Phase 1 while keeping the open
Phase 0 items deferred, and instructed agents to continue sensible work without
repeated permission questions. Phase advancement does not mark those gates passed.
On 2026-10-09 the director authorized agents to merge completed PRs into main
after review and passing checks, without requesting separate merge approval.

## Implemented and locally verified

- Typed versioned project JSON, semantic validation, published derived schemas,
  canonical formatting and stable ULIDs.
- One command bus with atomic edits, provenance, revisions, undo/redo, CRDT merging,
  durable recovery and exclusive journal ownership.
- Phase 1 static textured glTF/GLB import, meshopt cooking and versioned binary
  cache, with dependency invalidation and source-independent CPU loading. PNG,
  JPEG and EXR cook to independently validated KTX2 with color/normal-aware mips.
  CLI import preserves stable IDs and durable import settings through the shared
  command bus. The CPU runtime store publishes asset replacements atomically and
  keeps retained versions valid across reimport and undo. Headless runs load cooked
  assets before simulation. Animation, compression tiers and production
  material/ECS bindings remain open. The authoring CLI watches sources;
  the native editor now watches sources and replaces imported GPU geometry.
  See [asset pipeline evidence](docs/spikes/asset-pipeline.md).
- Bevy fixed-step simulation; isolated play state; SWC/QuickJS scripting, sandbox,
  live ECS queries and compatible hot reload.
- Actual wgpu rendering and PNG readback on Apple M5 Pro.
- Local OpenAI Responses loop, shared typed tools, approvals, token limits,
  interruption checks and strict streamed-response handling; deterministic tests.
  Saved-account providers reload credentials per request, refresh near expiry,
  retry HTTP 401 once after refresh, and stop after disconnect/account switch.
  Sign-out cannot reactivate an older API key; missing/damaged OAuth records can
  be cleared locally without claiming remote revocation.
- Shared editor/CLI OpenAI login and twenty-task live eval harness. Real macOS
  OAuth sign-in, refresh, session restoration across rebuilt apps and a live
  command-bus scene edit pass. The director-requested private-file store removes
  repeated Keychain prompts; see [login evidence](docs/spikes/auth-login-repair.md).
- Real execution of platform probes on macOS, browser/WASM and iOS simulator.
- Completed Claude Opus 5.5 ACP handoff, redesigned UI with a custom titlebar,
  retained before/after screenshots, and native integration.
- Supplied wisp mascot integrated into the titlebar, favicon and native app icons;
  [visual and packaging evidence](docs/spikes/wisp-mascot.md) records verification
  and the remaining native Dock visual-review limitation.
- Fourteen architecture records, generated TS structural bindings, developer docs,
  gate ledger and GitHub workflow source.

See [written evidence and limitations](docs/spikes/phase-0.md). The Phase 0 baseline of fifty-four Rust
behavior tests, 223 UI/bridge tests and five Python tool tests pass after
integration. The thousand-entity script benchmark meets the local frame budget
within its documented workload.

## Active work

- Bounded local avoidance exposes read-only `api.steerAgents` proposals for up
  to 128 agents. Library and public 100-agent save/reopen workflows pass.
  Claude's four-way plaza review completed twice with 803 identical frames and
  13 identical logs, no body overlaps and eventual arrival for all walkers.
  Goal-block standoffs and large lateral detours remain behavior-level limits;
  host sleep/load can trigger the existing script wall-clock deadline. The
  integrated branch passes 284 Rust tests and Clippy. Updated native/GPU/platform
  checks and hosted CI are in progress. See [steering evidence](docs/spikes/navigation-steering.md).
- Tiled runtime navigation bakes selected static colliders and cooked model
  geometry, reuses unchanged tiles and returns bounded portal-graph A* paths
  with funnel smoothing and connected visibility repair. Claude's rendered
  reviews exposed clearance, detour, phantom raster spans and short-query
  precision bugs; all have regression fixes. Current source passes 276 Rust,
  40 GPU, 315 UI and five tool tests, plus public saved-character gameplay,
  Clippy, SDK/generation checks, native packaging and WASM/iOS compilation.
  There are 7,056 local and 34,932 room-wide short-query regressions. See the
  [navigation evidence](docs/spikes/navigation-runtime.md). PR #35's scoped implementation
  and v9 rendered review are complete: both 600-tick runs match 726 frames/logs,
  with no query exceptions and grounding on every tick. All twelve CI checks
  passed on d7517b0; final report changes await current-head checks before merge. Quantized height
  detail can undershoot true step discontinuities; physics owns grounding.
  Steering is implemented and reviewed on its separate integration branch;
  off-mesh links are in integration checks. Grid navigation, native Inspector/
  debug draw and Core Sample/device gates remain open.


- XLIFF translator exchange exports typed tables and imports matching targets
  through one reversible shared-command transaction. Unknown/stale sources,
  malformed XML and incompatible arguments reject the full batch. No-op imports
  preserve history; empty targets and absent targets retain distinct meanings.
  The public two-table/strict-TypeScript workflow, exact durable Undo/Redo and
  rollback checks pass. Local verification passes 254 Rust tests, 40 GPU checks,
  315 UI tests/build, five tool tests, Clippy, contracts and native packaging;
  macOS executes the XLIFF smoke check, while WASM/iOS compile. All twelve hosted
  checks passed at `3724633`; PR #34 merged as `106a4cc` with its tested tree
  unchanged. The graphical localization panel remains open. See
  [translator exchange](docs/spikes/localization-exchange.md).

- Typed string tables, a bounded ICU MessageFormat subset, ICU4X plural/number/date
  formatting, fallback, pseudo-localization, missing-string reports and saved
  runtime locale switching are implemented. Shared commands preserve provenance,
  journal history and Undo/Redo. Local checks pass 245 Rust tests, 40 GPU checks,
  315 UI tests/build, five tool tests, Clippy, contracts and native packaging.
  Six public gameplay probes pass; the macOS platform probe executes locale data,
  while WASM/iOS target checks compile. A live saved-account Astra run authors and
  verifies a Japanese table that produces the expected gameplay text, without
  another login or Keychain prompt. An explicit UTF-8 subprocess fix resolves the Windows cp1252 probe failure.
  All twelve hosted checks passed at `184a0fe`; PR #33 merged as `7052e01` with
  its tested tree unchanged. Shaping/bidi/IME,
  localization UI and Core Sample visual/device gates remain open.
  See [localization runtime](docs/spikes/localization-runtime.md).

- Named button/axis input actions and runtime rebinding now use validated project
  settings and the shared command bus. Ordered physical transitions preserve
  short taps, combine alternative bindings, apply dead zones and normalize
  diagonal movement. Eight new tests, 212 total Rust tests, 315 UI tests/build,
  five tool tests, Clippy, contracts and native build/package pass. The strict TS
  public probe verifies Undo/Redo, invalid rollback, exact saved rebinding and
  expected movement; existing save/input/timer probes pass. Local input-only p95
  is 0.0025 ms for the typical workload and 2.8033 ms at the configured maximum.
  Integration with audio and merged timers passes 232 Rust tests, the five public
  gameplay probes, contracts, Clippy and native packaging. All twelve checks passed
  at `f6ebc14`; PR #32 merged as `68f6004` with its tested tree unchanged.
  Device adapters, haptics and the graphical binding editor remain open.
  See [input actions](docs/spikes/input-actions.md).

- Cooked WAV/OGG audio, typed sources/buses/listeners, Kira mixing/spatialization,
  bounded native/offline streaming and script-driven WAV export are implemented.
  The public import/RPC/strict-TypeScript probe works after deleting source files;
  repeated PCM is exact, pause is silent and bus attenuation matches expected
  levels. Twenty new tests bring the distinct passing Rust total to 215; Clippy,
  315 UI tests/build, five tool tests, generated contracts and native packaging
  pass. macOS/iOS device-backend and WASM compilation pass. All twelve hosted
  checks passed at `246a380`, including the Windows retained-reader repair test.
  Integration with merged timers passes 224 Rust tests, the combined public audio
  and saved-timer probe, contracts, Clippy, UI checks and native packaging. All
  twelve checks passed at `de83a4e`; PR #29 merged as `f96dca7` with its tested tree
  unchanged. Native controls/playback, browser resources, hardware,
  lifecycle/latency and mixer save cursors remain open. No device playback or local
  capture occurred. See [audio runtime](docs/spikes/audio-runtime.md).

- Fixed-tick script timers, repeat/cancel/replace, copied payloads and a saved
  game clock are implemented. Version 2 game saves preserve pending deadlines
  and read version 1 with an empty schedule. Seven script tests and a CLI failure
  test pass; a strict TypeScript public-CLI probe preserves callbacks, command
  effects and log continuity across process restarts. All 204 distinct Rust tests,
  315 UI tests/build, five tool tests, Clippy, generated contracts and the native
  build/package pass locally. All three hosted checks passed at `54f85a1`;
  PR #31 merged as `93bc5ed` with the reviewed tree unchanged. Coroutines remain
  open; Promise/generator behavior
  callbacks now fail explicitly before commit. See [timers](docs/spikes/script-timers.md).

- Headless `play --assertions FILE` evaluates bounded data-only checks over
  runtime, behavior and input state at absolute ticks. Failed checks return
  structured diagnostics, exit nonzero and suppress save publication. Five new
  CLI tests and the strict TypeScript character probe cover checkpoint seeking,
  movement/jumping/landing assertions, intentional failure, numeric equivalence,
  missing/null data, invalid plans and bounded output. All 195 Rust, 315 UI and
  five tool tests, Clippy, generated contracts and the native build/package pass.
  All three hosted checks, including GPU diagnostic-report retention, passed at
  `7379f9c`; PR #28 merged as `734d008` with the reviewed tree unchanged. Local
  capture stays paused. See [gameplay assertions](docs/spikes/play-assertions.md).

- Shared fixed-tick keyboard/mouse/gamepad/touch processing, sandbox `api.input()`
  and bounded headless input replay are implemented. Atomic validation, focus
  recovery, device lifetimes, gestures and saved-game input seeking pass eighteen
  new tests. The strict TypeScript public-CLI character probe jumps, reaches a wall
  and resumes a mid-jump save with exact final state/logs and unchanged authored
  files. All 190 Rust, 315 UI and five tool tests pass, along with Clippy, generated
  contracts and the native build/package. macOS normalized-input smoke execution
  passes. All twelve hosted checks passed at `0021738`; PR #27 merged as `72fb2bd`
  with the reviewed tree unchanged. Action mapping is implemented in the
  increment above; live OS/browser adapters and physical-device verification
  remain open. No computer use or local
  captures occurred. See [game input](docs/spikes/game-input.md).

- Versioned logical game saves preserve runtime scenes, hierarchy, spawned/deleted
  entities, JSON behavior state and the simulation clock. The headless host loads
  saves into isolated sessions and publishes new files atomically after success.
  Exact float decoding fixes clock/rotation drift on JSON reload. Nine new tests
  cover continuity, physical state, hot reload, invalid inputs and publication races;
  172 Rust tests and workspace Clippy pass. The strict TypeScript public-CLI probe
  matches uninterrupted state/logs across two process restarts with authored files
  unchanged. The native release package, 315 UI tests/build and five tool tests pass.
  All twelve hosted checks passed at `8cea975`; PR #26 merged as `39fed2d`
  with an identical reviewed tree. The local main app was rebuilt without opening. Physics
  warm starts, sleeping, VM globals and cross-revision migrations are outside this
  logical save format. See [game saves](docs/spikes/game-saves.md).

- Read-only character movement queries are implemented on the primitive runtime,
  with script-owned gravity/jumping and Velocity commands. Real motion review
  found walking stalls and a misleading slope flag; numerical face-normal handling
  and final-support classification correct both. Twelve 300-tick capsule/sphere/
  box walking cases pass, along with slope/stair/mask/isolation tests and the real
  strict-TypeScript CLI jump/landing probe. The corrected tree passes 163 Rust,
  40 GPU and 315 UI tests plus Clippy and the native release build. Actual macOS,
  browser/WASM and iOS simulator queries pass. Claude accepted the corrected
  course; 303 repeated frames match exactly. Stair speed/smoothing and moving
  platform behavior remain game-feel work. All nine final hosted checks passed at
  `54985fa`; PR #24 merged as `9f5906f`. See
  [character movement](docs/spikes/character-movement.md).

- Physics runtime integration is in progress on `impl/physics-runtime`. The
  [candidate probe](docs/spikes/physics-candidates.md) passed all six checks and
  merged as PR #22 (`a6fd91f`). The engine now takes PLAN.md's Rapier fallback
  after measured Jolt binding portability gaps; see ADR 0003. Typed bodies,
  primitive colliders, fixed stepping, sensors, masks and raycasts connect to
  isolated play and the shared command bus. Ten new behavior tests, full workspace Clippy, 154 Rust tests and
  315 UI tests/build pass locally. The earlier 40 GPU checks pass; all 26 final
  engine look-dev PNGs match Claude’s reviewed originals byte for byte. Public CLI,
  browser/WASM and iOS simulator physics execution pass. Shared validation and
  fewer snapshots reduce paired local 512-body p95 from 9.99 to 8.80 ms, with
  exact final-state equality. Claude handoff 0022 implemented and reviewed the new Inspector
  in browser fixtures and the native app. Shared schemas supply field order and
  units; WebKit field names and the compact Agent state are fixed. All twelve hosted
  checks passed at `6d5fca0`; PR #23 merged as `eb77419`. Character controllers
  are implemented in PR #24. Compound colliders remain in draft PR #25 pending
  the native visual review paused by the director. Mesh colliders, hierarchy/scale,
  rollback snapshots and device performance remain open. See [runtime evidence](docs/spikes/physics-runtime.md).

- Opt-in directional cascades are implemented with scoped Claude appearance
  and native acceptance. The scheduled depth pass honors imported caster flags,
  alpha masks, reflection and sidedness; retained queued frames and cascade
  transitions pass real GPU checks. Public CLI edits, atomic invalid rejection,
  Undo/Redo, durable reopening and source-free cached captures pass. Initial
  checks pass 144 Rust, 40 GPU and 294 UI tests plus Clippy. Native enabled/Off
  states, separated Assets workspace and sun-angle Undo/Redo pass. Final 1080p analytical timings
  measure 1.389–3.096 ms across 0–4 suns; thin-caster contact/aliasing limits remain.
  All twelve hosted checks pass at `5d31768`; PR #21 merged as `899cc2a`. local-light shadows and
  adaptive quality remain open. See [directional shadows](docs/spikes/directional-shadows.md).

- Real GPU passes now execute through a persistent Bevy ECS schedule with explicit
  dependencies and owned frame resources. Reverse-order queued submission after
  scene/source disposal passes. 139 Rust, 33 GPU and 283 UI tests, Clippy and the
  native build pass; 120/124 baseline PNGs match exactly, four differ at one/two
  pixels by one channel level. Those light fixtures now use deterministic IDs;
  repeated captures are exact. Claude approved the final fixtures/native images.
  Paired timings are stable; all twelve hosted checks pass at `50c950a` and PR #20
  merged as `13edae6`. Production shadows and
  post-effects remain open. See
  [render-pass scheduling](docs/spikes/render-pass-graph.md).

- Authored Camera selection is connected to renderer captures, public CLI,
  isolated playback and the agent screenshot tool. Geometry, material view vectors,
  transparent sorting and light clusters share the selected camera. Projection,
  clipping, inheritance, retained views, public command history and moving-camera
  playback pass. Empty camera-rig parents no longer create diagnostic cubes.
  139 Rust, 32 GPU and 283 UI tests, Clippy and the native release build pass.
  Claude approved all 36 captures, including independent skewed-basis checks.
  A live saved-account agent captured the exact camera with no edits or login
  prompts. Hosted desktop/credential checks pass; the source job found a stale
  generated tool-schema snapshot, corrected at `d9ace60`. All six exact-head checks now pass at `d9ace60`; PR #19 merged as
  `695f3b3`. No editor camera-selection control is claimed.
  See [authored camera evidence](docs/spikes/authored-camera-capture.md).

- Cluster membership masks replace overflow fallback in the next lighting
  increment. Exact GPU readback covers every bit through 4096 lights, word-count
  changes and stale clearing; full-capacity mixed point/spot output matches an
  explicit all-light reference byte for byte. A final-bit-only visible capture
  also matches the one-light reference. Claude approved appearance; 135 Rust,
  27 GPU and 283 UI tests, Clippy and native release build pass. Three paired
  trials measure 34.23 → 6.44 ms median at 4096 lights on M5 Pro, with identical
  cooked projects. This is a synthetic fenced frame probe, not a game/device
  gate. All three applicable hosted checks pass at `2c517ce`; PR #18 merged
  into main as `1b35f02`. See [light mask evidence](docs/spikes/light-masks.md).

- Authored directional, point and spot lights use real GPU clustered forward
  shading with bounded lists and an exact overflow fallback. The shared command
  bus, durable history, schemas and read-only Inspector remain the authoring
  interface. Final validation passes 135 Rust tests, 24 GPU checks and 283 UI
  tests. Claude approved the final headless/native captures after fixing the
  range-edge rim, RGB labels and unit display. Public CLI edits, atomic rejection,
  Undo/Redo and source-free rendering pass. The 1080p local frame probe measures
  1.38–28.05 ms median across 0–4096 local lights on M5 Pro; the dense case needs
  optimization and no game/device performance gate is claimed. All twelve hosted
  checks pass at `788da07`, including the serialized Windows GPU rerun; PR #17
  merged as `726a25d`. The dense case is improved by the mask increment above.
  See [clustered lighting evidence](docs/spikes/clustered-lighting.md).

- Distant environment lighting includes diffuse convolution, GGX roughness
  filtering, a matching BRDF lookup and retained GPU versions. Authored
  `EnvironmentLight` components bind imported equirectangular textures through
  the shared command bus. Public CLI rotation, Undo/Redo and source-free loading
  pass. 131 Rust tests, 15 GPU checks and 282 UI tests pass. Native reimport,
  Undo/Redo, error retention/recovery and rebuilt account restoration pass.
  Claude approved the headless and native captures and the final Inspector label
  fix at wide/minimum sizes. All twelve hosted checks passed on `42a991d`; the
  final revision `1db7c88` also passed all twelve checks. PR #16 merged as
  `c259b5d` on 2026-10-09.
  See [environment lighting evidence](docs/spikes/environment-lighting.md).

- HDR geometry now renders to retained RGBA16Float attachments before the display
  transform. Claude's derived preview curve preserves ordinary colors and rolls
  off bright highlights. Eleven explicit GPU checks cover tone mapping, display
  formats, dark materials, HDR transparency, resize lifetime, imports and playback.
  The workspace passes 124 Rust tests, Clippy and 282 UI tests/build. Claude approved
  the 24 final GPU captures and both native sizes. All three applicable hosted checks passed on `573fbc8`; PR #15 merged
  as `5b8fcd3` on 2026-10-09. Full production lighting, render graph, IBL,
  shadows, bloom and antialiasing
  remain open. See [HDR output evidence](docs/spikes/hdr-output.md).

- Structured script logging now uses the bounded sandbox API and preserves
  messages from successful ticks. Both headless script/play commands return logs;
  playback can additionally stream a new JSONL file without requiring a GPU.
  Failure retains prior completed records, and output names/budgets are checked.
  Five new behavior tests and the combined frame/log GPU case pass. The combined
  material/log tree passes 124 Rust tests, seven explicit GPU tests and Clippy;
  the unchanged UI passes 282 tests and its build. All three applicable hosted
  checks passed on `b09a3e2`; PR #14 merged as `b36b76f` on 2026-10-09. See [runtime log evidence](docs/spikes/runtime-logs.md).

- Imported glTF material previews now use typed metallic/roughness factors and
  retained GPU base-color, metallic/roughness, normal, occlusion and emissive maps.
  Color spaces, samplers/mips, alpha modes, sidedness and reflected instances are
  implemented. 119 Rust behavior tests and seven explicit GPU tests pass locally.
  Native texture reimport, Undo/Redo, retention on source error and repair
  pass. Claude approved the final headless and native captures, including the
  minimum-height Inspector scroll behavior. All twelve hosted checks passed on
  `f7607ab`; PR #13 merged as `794c765` on 2026-10-09. Production lighting and
  the full render graph remain open. See
  [GPU material evidence](docs/spikes/gpu-materials.md).

- Isolated headless playback now accepts ticks/seconds and an optional compiled
  TypeScript behavior, with actual GPU captures at selected ticks. Authored
  documents/journals remain unchanged; bounded output and failure handling pass.
  117 Rust behavior tests, three explicit GPU tests and 282 UI tests pass after
  integration with native source watching.
  A source-free imported-model run and a compiled TypeScript run produce real
  frame sequences. Claude approved the six captured frames. All twelve hosted checks passed on
  `58118f1`; PR #12 merged into main as `1d9650b` on 2026-10-09. See
  [headless playback evidence](docs/spikes/headless-playback.md).

- Saved native projects now watch registered sources and dependencies on a worker,
  prepare outside the command-bus lock and commit with revision checks. Imports
  share durable history; source errors retain the last valid geometry and clear
  on repair. Actual native external edits, Undo/Redo, error/recovery, journal
  replay and reopening the rebuilt app pass. 115 Rust tests, two explicit GPU
  tests and 282 UI/bridge tests pass locally. Claude approved the native sequence
  and final wrapping fix at 1440×900 and 1000×650. All three applicable hosted
  checks passed on `083bc8e`; PR #11 merged as `dd9c292` on 2026-10-09.
  See [editor source watching](docs/spikes/editor-source-watch.md).

- Imported model GPU geometry is implemented locally: indexed/instanced buffers,
  default glTF scene/node hierarchy, version retention, revision-driven editor
  replacement, and shared CLI/agent rendering. CPU checks, two real GPU tests and
  public CLI import/reimport/readback pass. Native Undo and missing-cache recovery
  pass without reopening. Claude approved the final native wide/minimum captures;
  112 Rust behavior tests, two explicit GPU tests and 279 UI tests pass, including
  render-error diagnostics and recovery. Default cache reads reject directory
  symlinks and asset reuse is scoped to the project grant. All twelve hosted
  checks passed on `277893d`; PR #10 merged as `faa86fa` on 2026-10-09.
  Materials, lighting and the production render graph remain open.
  See [imported GPU geometry](docs/spikes/imported-gpu-geometry.md).

- Runtime scene projection now composes parent transforms, preserves mesh bindings,
  and atomically synchronizes topology while reusing the ECS world. Five new core
  behavior tests and asset-kind validation pass. The real GPU probe matches a
  parented scene to equivalent flattened geometry. All twelve hosted checks passed
  on `97cf521`; PR #9 merged into main as `331a3cb`. The imported-geometry increment
  above builds on this projection; GPU materials remain open. Script benchmark results
  and their host-load variability are recorded in
  [runtime scene evidence](docs/spikes/runtime-scene-projection.md).


- `incant_import` now prepares glTF/image import batches on an owned document
  snapshot and commits all changed assets in one command-bus transaction. The CLI
  uses this shared service. Five service behavior tests, the existing CLI tests, full workspace release
  tests and Clippy passed locally and all twelve hosted checks passed before PR #6
  merged into main as `130d249`. Editor/agent integration is the current increment.

- Automatic source watching now debounces registered model/image sources and
  dependencies, commits successful changes through shared history and reloads CPU
  assets in `incant watch-assets`. Six service and two separate-process CLI tests
  pass. Editor dispatch and GPU/ECS hot reload remain open.
  See [source watching evidence](docs/spikes/asset-source-watch.md).
  PR #7 merged as `0c8e1bc` after all three applicable hosted checks passed on
  `79a821e`, including source checks and Windows/Linux desktop jobs.
  [Exact checks and merge evidence](docs/spikes/evidence/asset-source-watch-2026-10-09.json).

- Native editor imports now prepare outside the event thread/document lock and
  commit with revision checks through the existing persistent history. Two new
  Rust behavior tests and 276 integrated UI/bridge tests pass. Claude Opus 5.5 ACP
  handoff 0010 supplies the integrated asset library and import/reimport interface.
  Real native batch import, Undo/Redo, texture reimport, error recovery, concurrent
  edits, stale-import rejection and text/project Undo separation pass. The director
  rejected the bottom-dock asset layout. Claude's replacement moves Assets to the
  left navigation and properties/import into the main Inspector. The dock now holds
  only Problems, Console and History. All 276 UI tests pass; native batch, busy,
  failure recovery, keyboard and hidden-Inspector checks pass in the replacement.
  Claude approved the replacement's native captures at wide/minimum sizes and
  supplied a final import-row separator polish. All twelve hosted checks passed
  on `73b817b`; PR #8 merged into main as `cda5ef3`. Automatic editor
  watching is implemented in the next increment.
  Agent asset list/inspect/import now use the shared service and a project-bound
  filesystem capability. Three new agent tests and a live saved-session import
  pass, including journal recovery, Undo/Redo and source-independent CPU loading.
  Default cache writes reject symlink redirection; this fix merged with PR #7.
  See [editor/agent import evidence](docs/spikes/editor-agent-asset-imports.md).

- Responsive project loading merged in PR #5 after all twelve hosted checks passed.
  File reads and journal recovery
  run off the native event thread, with a 15-second timeout and rejected late
  results. Three Rust loader tests, 240 integrated UI/bridge tests, workspace release tests
  and Clippy pass locally. Claude handoff 0009 supplies the loading/error
  presentation and keyboard-dialog focus ring; the native app builds successfully.
  Protected-folder access remains unproven. The rebuilt app opens both default
  and saved disposable projects; native account focus behavior passes, with the
  visible keyboard ring also approved by Claude.

- The first eighteen PRs are merged into main after their required checks passed:
  foundation #1, product site #2, asset imports #3, runtime assets/UI polish #4,
  responsive loading/focus #5, shared import batches #6, source watching #7 and
  editor/agent imports with the redesigned asset workspace #8, and runtime scene
  projection #9, imported GPU geometry #10, native source watching #11 and headless
  playback #12, material previews #13, structured runtime logs #14, HDR output #15
  authored environments #16, clustered lights #17 and exact light masks #18. The site deployed at
  https://snowdamiz.github.io/incant/. PR #4 passed all thirteen checks on c063a99
  before merge. Shared import preparation passed all twelve checks on `4bddf52`
  before merge. Current Phase 1 work is tracked above. Merging does not approve a phase gate.

- Claude’s connected-panel redesign and neutral charcoal palette are integrated,
  including the titlebar logo spacing and safer account-dialog focus. The landing
  page has the matching palette, merged in PR #2. Its modeling expansion and lighter copy
  each passed 23 hosted browser checks. Graphical explanations, authentic SVG
  platform marks and the subsequent workflow correction are integrated in PR #2.
  Each workflow icon has a short visible explanation and tighter spacing; all 23
  hosted checks passed on bf99437. Visual review subsequently resumed, then was paused again on 2026-10-09. See
  [integration evidence](docs/spikes/connected-editor.md). Native menu Undo routes by focus. Actual macOS CUA checks now confirm
  project Cmd+Z/redo, isolated text-field Undo, safe account-dialog focus, divider
  keyboard resizing and fullscreen transitions. Claude reviewed the native captures;
  an unobscured native capture now confirms correct traffic-light/logo spacing. On 2026-10-09 the director explicitly permitted computer use
  and screen capture again when needed. Claude’s handoff 0008 fixes tablet workflow layout, platform-mark balance, clipped
  inspector paths and F2 name selection. Rebuilt native F2 rename and Undo pass. The transport now preserves schema field
  order (223 tests passed); native Translation/Rotation/Scale order now also passes.
  Claude confirmed settled fullscreen. The dialog ring fix, minimum-size tabs,
  detail focus rings and visible traffic-light positioning now pass native review.
  Per the director's 2026-10-08 decision, Windows/Linux manual
  login and Windows viewport checks are replaced by CI editor/engine builds,
  UI/bridge tests, GPU readback and credential-persistence checks. Those jobs are
  now passing on both Windows and Linux; see
  [desktop CI evidence](docs/spikes/evidence/desktop-editor-2026-10-08.json).
  No additional desktop machines or interactive logins are needed.
- The real twenty-task evaluation completed with the director's saved session:
  **19/20 passed**, exceeding the 14/20 score threshold. The ten-step case reached
  the expected state but returned an incomplete provider response and is counted
  as failed. [Per-case results](docs/spikes/evidence/live-agent-2026-10-08.json).
  Its [single-case follow-up](docs/spikes/evidence/ten-step-followup-2026-10-08.json)
  passes all checks with the new output budget and diagnostic handling. The original
  score remains 19/20; the old failure's cause was not retained.
- Cross-build synthetic credential checks run on all three desktop CI hosts;
  they verify the real storage backends without using provider accounts.
  [All three hosted jobs passed](docs/spikes/evidence/desktop-credentials-2026-10-08.json),
  including record cleanup.
- Foundation [PR #1](https://github.com/snowdamiz/incant/pull/1) merged into main
  on 2026-10-09 after all twelve checks passed on 575cc35. All six platform jobs
  [passed on 640fbdd](docs/spikes/evidence/six-platform-2026-10-08.json), including
  Android APK packaging. Both arm64/x86_64 native libraries, manifest and DEX are
  present in the downloaded APK. [Android emulator execution also passed](docs/spikes/evidence/android-emulator-2026-10-08.json):
  both activity launch and native instrumentation ran the document/120-tick probe.
  Nightly history remains a distinct requirement. All source, desktop editor and
  credential checks also passed on 1045f4f. The integrated UI/menu/sign-out revision
  **d87436f** has now passed all four workflows: source checks, Windows/Linux
  editor builds and renderer probes, three-desktop credential persistence, and all
  six platform probes. The evidence files retain the exact revision and run links.

## Current screen-work constraint

Revision 3 is the current implementation baseline. Its additional Phase 1 work
includes the implemented tiled navmesh/path-query and localization runtime above.
Still open are navigation steering/local avoidance, off-mesh links and 2D grids;
sprites/atlases/tilemaps and Rapier 2D; shaped text, font fallback and IME;
localization UI and gamepad/mobile haptics. The expanded Core Sample gate includes navigation,
RTL/CJK/localization coverage and a 2D sample on all four reference devices.
This does not replace the remaining original renderer, animation, audio/device,
input-adapter, SDK/coroutine and complete-game performance requirements. Later
networking and service scope follows Revision 3 Sections 6.8/6.10; their open
director decisions and human-owned production prerequisites remain explicit.

The director reauthorized computer use and screen capture on 2026-10-10.
Claude's resumed compound review accepted the supplied native and corrected-runtime
scope, then improved the unavailable Agent pane. Final post-change native captures
currently await Mac unlock; the unlock request is pending. Compound PR #25 remains
a separate draft while that final native confirmation is open. All twelve hosted
checks passed on compound head `5b97ecd`.
The compound changes have not merged into main.

## External prerequisites still required

- Physical-device
  coverage, self-hosted graphics/device runners, and nightly artifact history.
- Apple/Windows distribution signing, store/developer accounts, staffing and the
  director's phase gate approvals. These are human-only under PLAN.md.

[Phase 0 review checklist](docs/gates/phase0-review.md) maps requirements to evidence
and distinguishes the remaining director decisions.

`python3 tools/check_gate.py` reports remaining evidence. No source-code scaffold,
mock response, simulator build or unsigned package counts as a shipped product.
