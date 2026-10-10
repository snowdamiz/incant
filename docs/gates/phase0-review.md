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
| Apple and Windows signing certificates | Removed from Phase 0 by director decision on 2026-10-10 | First needed for Phase 5 store uploads; unsigned development builds remain development builds, not substitutes |
| Year 1 director/reviewer/contracts | Director is identified by the plan; reviewer/contracts are not recorded | Director confirmation is required; no staffing or contract commitments were made by an agent |

The ledger at [phase0.json](phase0.json) remains unapproved. The gate command reports
auth verification, scheduled nightly artifacts, staffing and director
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

## Decisions from PLAN.md section 12

On 2026-10-10 the director instructed agents to resolve every open decision with
engineering judgment. PLAN.md section 12 records all 27 decisions and the
human-owned actions that remain. Recording these decisions does not approve this
gate or complete any listed human-owned action.

| Decision | Disposition |
|---|---|
| React versus Solid | React (decision 1, ADR 0009) |
| Loro versus Automerge | Loro, one document per scene; Automerge fallback with a measured switch rule (decision 2, ADR 0008) |
| Managed identity versus self-hosted accounts | WorkOS AuthKit behind OpenID Connect, Zitadel as replacement; game player identity self-hosted (decision 3, ADR 0012). Account creation is human-owned |
| RON versus JSON | Canonical typed JSON with derived JSON Schemas (decision 4, ADR 0008) |
| Browser editor in 1.0 or 1.1 | 1.0 with a defined scope (decision 5, ADR 0009); not yet delivered |
| Engine license/business model | MIT OR Apache-2.0, royalty-free, repository public at the Phase 8 beta, revenue from Incant Cloud and Driftwake (decision 6) |
| Game name and engine-name clearance/reservations | Driftwake (decision 7). Name clearance, domain registration and package-name reservations were removed as requirements by director decision on 2026-10-10 |
| Codex execution mode | Both CLI and cloud tasks (decision 8); no cloud Codex deployment is claimed |
| Approval mode by risk | Settled by the 2026-10-08 and 2026-10-09 director decisions (decision 9). Claude ACP bypassPermissions was explicitly authorized. Phase approval, signing, purchases and legal/staffing actions remain director-owned |

## Order for remaining work

1. Continue Phase 1 implementation and merge completed PRs after review and passing
   checks. Landing-page design changes remain routed through Claude ACP.
2. Complete live revocation verification with a suitable disposable connection;
   do not revoke the director's working account just to satisfy a test.
3. Complete the native checks now that the director has restored computer-use and
   capture permission; record actual results through Claude visual review.
4. The foundation is now on main. Record scheduled workflow results from the default
   branch as they become available; a merge or a manual run does not prove nightly history.
5. Record the human-owned staffing actions; signing moves to Phase 5 store uploads.
   Request phase approval only when its evidence is ready; do not self-approve it.

No additional Windows/Linux desktop machines or manual login checks are required.
No additional API key is requested from the director.
