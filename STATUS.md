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
  automatic source watching remain open.
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

- All three initial PRs are merged into main after their required checks passed:
  foundation #1, product site #2 and asset imports #3. The site deployed successfully
  at https://snowdamiz.github.io/incant/. Runtime asset-version work and renewed
  Claude visual review continue separately. Merging does not approve a phase gate.

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
  the capture indicator still obscures the traffic lights. On 2026-10-09 the director explicitly permitted computer use
  and screen capture again when needed. Claude’s handoff 0008 fixes tablet workflow layout, platform-mark balance, clipped
  inspector paths and F2 name selection. The latest UI changes still need a rebuilt
  native check; minimum-size and settled-fullscreen captures remain open.
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
