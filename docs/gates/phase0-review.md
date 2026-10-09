# Phase 0 review checklist

This records the foundation against PLAN.md. On 2026-10-08 the director explicitly
authorized starting Phase 1 while the remaining Phase 0 items stay open, then
instructed agents to continue sensible implementation without repeated permission
questions. This authorizes engineering progress; it does not mark missing evidence
complete. The full Engine 1.0 and Driftwake objectives remain open. The director owns
phase approval. On 2026-10-09 the director authorized agents to merge completed PRs
after review and passing checks. Foundation [PR #1](https://github.com/snowdamiz/incant/pull/1)
merged into main after all twelve checks passed on 575cc35; this does not approve
the Phase 0 gate.
The separate landing page is [PR #2](https://github.com/snowdamiz/incant/pull/2).

## Deliverables and evidence

| Requirement | Current evidence | Remaining boundary |
|---|---|---|
| Architecture records for all stack rows | [Fourteen ADRs](../adr/README.md) cover all fourteen rows | Engineering proposals await director review; they are not implemented later-phase subsystems |
| Native viewport spike | [macOS review](../../handoffs/0002-native-viewport/result.md), Windows/Linux native builds and GPU readback in [desktop evidence](../spikes/evidence/desktop-editor-2026-10-08.json) | Accepted under the director's CI replacement for manual Windows review; latest UI follow-ups are separate below |
| Bevy → CRDT → two clients → text roundtrip | [Collaboration result](../spikes/evidence/collaboration-spike.json) records two-client convergence and byte-identical canonical text after 120 ticks | No production network collaboration service is claimed |
| QuickJS/Bevy hot reload and 1,000 entities | [Measured run](../spikes/evidence/script-benchmark.json), behavior tests and [scope](../spikes/phase-0.md#spike-3-quickjs-typescript-and-bevy--local-evidence-passed-with-scope-limits) | Batched script updates; no claim of 1,000 isolated VMs, mobile frame time or full rendering frame budget |
| Twenty live agent tasks, at least fourteen successes | [19/20 full-suite result](../spikes/evidence/live-agent-2026-10-08.json); [separate ten-step follow-up](../spikes/evidence/ten-step-followup-2026-10-08.json) | The original incomplete response remains a failure in the full-suite score; the in-editor composer is not implemented in Phase 0 |
| OAuth, API-key fallback and credential persistence | [macOS live sign-in/refresh/rebuild evidence](../spikes/auth-login-repair.md), [three-desktop CI](../spikes/evidence/desktop-credentials-2026-10-08.json) and automated API-key protocol tests | Live revocation evidence remains pending. A separate live API key is not required for the director's OAuth workflow |
| Claude 5.5 ACP roundtrip | [First result packet](../../handoffs/0001-editor-foundation/result.md), later native/account/palette packets and integration commits | No copied coding-agent credentials; handoff 0008 resumes visual review after the director restored capture permission |
| Runnable artifacts for six targets | [Build and execution evidence](../spikes/evidence/six-platform-2026-10-08.json), including hosted Android emulator execution | PR-triggered runs do not prove scheduled nightly history or signed distribution |
| Apple and Windows signing certificates | No certificate evidence recorded | Director-provided accounts/certificates are required; unsigned development builds are not substitutes |
| Year 1 director/reviewer/contracts | Director is identified by the plan; reviewer/contracts are not recorded | Director confirmation is required; no staffing or contract commitments were made by an agent |

The ledger at [phase0.json](phase0.json) remains unapproved. The gate command reports
auth verification, scheduled nightly artifacts, signing, staffing and director
approval as outstanding. Phase 1 implementation is now separately authorized.

## Latest requested editor changes

Claude's connected panels, neutral palette and logo spacing are integrated, and
the matching landing-page revision passed its CI. The account dialog opens on a
safe control. macOS menu history now routes through focus and the shared command
bus. Local tests and the development build pass. At source revision `d87436f`,
all four hosted workflows also passed: source checks, Windows/Linux native editor
and renderer probes, three-desktop credential persistence, and six-platform
probes. The linked evidence records exact revisions and run URLs; see the
[integration report](../spikes/connected-editor.md).

The director temporarily stopped computer use and screen capture, then explicitly
permitted them again on 2026-10-09 when needed. Actual CUA checks on the rebuilt macOS app verified project Cmd+Z/redo, text-only
Undo in a rename draft, account-dialog Close focus, divider keyboard resizing and
fullscreen transitions. Claude reviewed the supplied native images in handoff 0008.
The capture indicator still hides the traffic lights; exact light alignment, a
settled fullscreen capture, minimum window size and the newest F2 fix need follow-up.
A browser fixture or compiled app does not prove those native details.

## Open decisions from PLAN.md section 12

| Decision | Current disposition |
|---|---|
| React versus Solid | React is the implemented proposal in ADR 0009 |
| Loro versus Automerge | Loro is the implemented proposal, with both benchmark results retained in ADR 0008 |
| Managed identity versus self-hosted accounts | ADR 0012 separates optional cloud metadata from local provider login; the identity operating choice still requires director review before service implementation |
| RON versus JSON | Canonical typed JSON with derived JSON Schemas is implemented in ADR 0008 |
| Browser editor in 1.0 or 1.1 | PLAN.md's definition of done includes it in 1.0; the WASM probe is not the delivered browser editor |
| Engine license/business model | No license or paid-cloud business decision has been made on the director's behalf |
| Game name and engine-name clearance/reservations | Driftwake remains a placeholder. Incant is the chosen engine name; trademark/domain/package reservations are not represented as complete |
| Codex execution mode | Phase 0 work has used local Codex/CLI, local Claude ACP and hosted CI; no cloud Codex deployment is claimed |
| Approval mode by risk | Reversible local implementation/tests proceed autonomously. Claude ACP bypassPermissions was explicitly authorized. Agents merge completed PRs after review and passing checks under the 2026-10-09 authorization. Phase approval, signing, purchases and legal/staffing actions remain director-owned |

## Order for remaining work

1. Continue Phase 1 implementation and merge completed PRs after review and passing
   checks. Landing-page design changes remain routed through Claude ACP.
2. Complete live revocation verification with a suitable disposable connection;
   do not revoke the director's working account just to satisfy a test.
3. Complete the native checks now that the director has restored computer-use and
   capture permission; record actual results through Claude visual review.
4. The foundation is now on main. Record scheduled workflow results from the default
   branch as they become available; a merge or a manual run does not prove nightly history.
5. Record the human-owned signing, staffing and outstanding strategic decisions.
   Request phase approval only when its evidence is ready; do not self-approve it.

No additional Windows/Linux desktop machines or manual login checks are required.
No additional API key is requested from the director.
