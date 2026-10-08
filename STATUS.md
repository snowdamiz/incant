# Implementation status

Source plan: [PLAN.md](PLAN.md), revision 2. Current phase: **0, in progress**.
The complete engine/game release is not implemented. No phase gate is approved.

## Work in progress

- Monorepo and pinned Rust/Node development tools.
- Spike 6: scoped ACP client, shared generated conventions, visual handoff packet.
- Spikes 2–3: typed document, command journal and QuickJS/Bevy integration.
- Spikes 4–5: provider integration and authentication with local secure storage.
- Spike 1: native viewport proof and Claude-owned UI.

## Required evidence

Each spike needs measured results in docs/spikes. Live model evaluations must be
identified separately from deterministic/mocked tests. Platform runs that did not
happen must remain pending. Phase 1 cannot start before the Phase 0 gate passes.

## External prerequisites

- Authenticated Claude Code subscription for the visual handoff.
- User-authorized OpenAI provider connection for live twenty-task evaluation.
- Windows/Linux/mobile runners, Apple/Windows signing credentials, remote GitHub
  repository and CI configuration. No deployment destination was supplied.
- Director approval of phase gates; staffing, legal and store accounts per PLAN.md.
