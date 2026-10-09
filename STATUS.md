# Implementation status

Source: [PLAN.md](PLAN.md), revision 2. Current phase: **0, in progress**.
The complete engine/game release is not implemented. No phase gate is approved.
The plan forbids beginning Phase 1 before Phase 0 passes and the director approves.

## Implemented and locally verified

- Typed versioned project JSON, semantic validation, published derived schemas,
  canonical formatting and stable ULIDs.
- One command bus with atomic edits, provenance, revisions, undo/redo, CRDT merging,
  durable recovery and exclusive journal ownership.
- Bevy fixed-step simulation; isolated play state; SWC/QuickJS scripting, sandbox,
  live ECS queries and compatible hot reload.
- Actual wgpu rendering and PNG readback on Apple M5 Pro.
- Local OpenAI Responses loop, shared typed tools, approvals, token limits,
  interruption checks and strict streamed-response handling; deterministic tests.
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

See [written evidence and limitations](docs/spikes/phase-0.md). Forty-one Rust
behavior tests, 180 UI/bridge tests and four Python tool tests pass after integration. The thousand-entity script benchmark meets the local
frame budget within its documented workload.

## Active work

- Claude completed the macOS native-window review, including traffic-light
  alignment, rounded GPU viewport, resizing and screenshots. The connected-account
  UI is integrated. Windows native viewport evidence is pending.
- The real twenty-task evaluation completed with the director's saved session:
  **19/20 passed**, exceeding the 14/20 score threshold. The ten-step case reached
  the expected state but returned an incomplete provider response and is counted
  as failed. [Per-case results](docs/spikes/evidence/live-agent-2026-10-08.json).
- Draft PR: https://github.com/snowdamiz/incant/pull/1. macOS checks and the
  six-platform workflow ran: full checks and five platforms passed. Android
  reached compilation but pulled in an unconfigured Android activity implementation.
  Core now depends directly on Bevy app/ECS, removing that unused dependency;
  local tests pass and remote validation is pending.

## External prerequisites still required

- Private repository created at https://github.com/snowdamiz/incant. The bootstrap
  is on main; implementation is in draft PR #1 on impl/phase0-foundation.
  The director remains responsible for merging.
- Android execution and desktop authentication checks, physical-device
  coverage, self-hosted graphics/device runners, and nightly artifact history.
- Apple/Windows distribution signing, store/developer accounts, staffing and the
  director's phase gate approvals. These are human-only under PLAN.md.

`python3 tools/check_gate.py` reports remaining evidence. No source-code scaffold,
mock response, simulator build or unsigned package counts as a shipped product.
