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

See [written evidence and limitations](docs/spikes/phase-0.md). Fifty-four Rust
behavior tests, 223 UI/bridge tests and five Python tool tests pass after integration. The thousand-entity script benchmark meets the local
frame budget within its documented workload.

## Active work

- Claude’s connected-panel redesign and neutral charcoal palette are integrated,
  including the titlebar logo spacing and safer account-dialog focus. The landing
  page has the matching palette in PR #2. See
  [integration evidence](docs/spikes/connected-editor.md). Native menu Undo now
  routes by focus; automated tests pass, but its final native interaction check
  remains pending. **Computer use, browser automation and screen capture are
  stopped at the director’s request** until explicitly permitted again.
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
- Draft PR: https://github.com/snowdamiz/incant/pull/1. All six platform jobs
  [passed on 640fbdd](docs/spikes/evidence/six-platform-2026-10-08.json), including
  Android APK packaging. Both arm64/x86_64 native libraries, manifest and DEX are
  present in the downloaded APK. [Android emulator execution also passed](docs/spikes/evidence/android-emulator-2026-10-08.json):
  both activity launch and native instrumentation ran the document/120-tick probe.
  Nightly history remains a distinct requirement. All source, desktop editor and
  credential checks also passed on 1045f4f. The newly integrated UI/menu revision
  has passed local checks and awaits its own remote results.

## External prerequisites still required

- Private repository created at https://github.com/snowdamiz/incant. The bootstrap
  is on main; implementation is in draft PR #1 on impl/phase0-foundation.
  The director remains responsible for merging.
- Physical-device
  coverage, self-hosted graphics/device runners, and nightly artifact history.
- Apple/Windows distribution signing, store/developer accounts, staffing and the
  director's phase gate approvals. These are human-only under PLAN.md.

`python3 tools/check_gate.py` reports remaining evidence. No source-code scaffold,
mock response, simulator build or unsigned package counts as a shipped product.
