# Implementation status

Source: [PLAN.md](PLAN.md), revision 2 with director decisions. Current phase:
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
  assets before simulation. Animation, compression tiers, GPU/ECS bindings and
  editor/GPU hot reload remain open. The authoring CLI now watches registered
  sources and reloads CPU assets.
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

- Imported model GPU geometry is implemented locally: indexed/instanced buffers,
  default glTF scene/node hierarchy, version retention, revision-driven editor
  replacement, and shared CLI/agent rendering. CPU checks, two real GPU tests and
  public CLI import/reimport/readback pass. Native Undo and missing-cache recovery
  pass without reopening. Claude approved the final native wide/minimum captures;
  112 Rust behavior tests, two explicit GPU tests and 279 UI tests pass, including
  render-error diagnostics and recovery. Default cache reads reject directory
  symlinks and asset reuse is scoped to the project grant. PR #10
  awaits final-head hosted checks; materials, lighting and the production render graph remain open.
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
  watching remains open.
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

- The first nine PRs are merged into main after their required checks passed:
  foundation #1, product site #2, asset imports #3, runtime assets/UI polish #4,
  responsive loading/focus #5, shared import batches #6, source watching #7 and
  editor/agent imports with the redesigned asset workspace #8, and runtime scene
  projection #9. The site deployed at
  https://snowdamiz.github.io/incant/. PR #4 passed all thirteen checks on c063a99
  before merge. Shared import preparation passed all twelve checks on `4bddf52`
  before merge. Editor/agent imports are the current Phase 1 increment. Merging does
  not approve a phase gate.

- Claude’s connected-panel redesign and neutral charcoal palette are integrated,
  including the titlebar logo spacing and safer account-dialog focus. The landing
  page has the matching palette, merged in PR #2. Its modeling expansion and lighter copy
  each passed 23 hosted browser checks. Graphical explanations, authentic SVG
  platform marks and the subsequent workflow correction are integrated in PR #2.
  Each workflow icon has a short visible explanation and tighter spacing; all 23
  hosted checks passed on bf99437. Visual review is resuming under the new permission. See
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

## External prerequisites still required

- Physical-device
  coverage, self-hosted graphics/device runners, and nightly artifact history.
- Apple/Windows distribution signing, store/developer accounts, staffing and the
  director's phase gate approvals. These are human-only under PLAN.md.

[Phase 0 review checklist](docs/gates/phase0-review.md) maps requirements to evidence
and distinguishes the remaining director decisions.

`python3 tools/check_gate.py` reports remaining evidence. No source-code scaffold,
mock response, simulator build or unsigned package counts as a shipped product.
