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
- OAuth/API-key/secure-storage implementation and twenty-task live eval harness.
  macOS temporary keychain roundtrip passes. Live authentication is unverified.
- Real execution of platform probes on macOS, browser/WASM and iOS simulator.
- ACP worktree handoff, explicit Opus 5.5 selection and interruption recovery.
- Fourteen architecture records, generated TS structural bindings, developer docs,
  gate ledger and GitHub workflow source.

See [written evidence and limitations](docs/spikes/phase-0.md). Twenty-nine Rust
behavior tests and four Python tool tests have passed; checks will run again after
native editor integration. The thousand-entity script benchmark meets the local
frame budget within its documented workload.

## Active work

- Claude Opus 5.5 is finishing the editor shell, visual tests, screenshots and result
  packet in `.worktrees/0001-editor-foundation`.
- Astra's Tauri host and typed native bridge await UI integration and native-window
  interaction/composition validation. Windows native viewport evidence is pending.
- CI workflows are authored; remote execution and target-specific validation remain.

## External prerequisites still required

- A GitHub repository URL to run CI and prepare a review PR; none is configured.
- A working user-authorized OpenAI connection for the live twenty-task evaluation.
  The first browser attempt did not complete; no provider tokens were obtained.
- Windows/Linux/Android execution and desktop authentication checks, physical-device
  coverage, self-hosted graphics/device runners, and nightly artifact history.
- Apple/Windows distribution signing, store/developer accounts, staffing and the
  director's phase gate approvals. These are human-only under PLAN.md.

`python3 tools/check_gate.py` reports remaining evidence. No source-code scaffold,
mock response, simulator build or unsigned package counts as a shipped product.
