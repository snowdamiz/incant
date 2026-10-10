# ADR 0013: Pluggable generation providers

Date: 2026-10-08. Updated: 2026-10-10. Status: decided by director instruction
in the PLAN.md revision 5 decision record (decision 22). This does not approve
the Phase 0 gate.

## Decision

Images and texture sets use the user's existing OpenAI connection. For 3D, audio and animation, each phase ships the provider plugin with the highest output-validity pass rate in the eval harness whose terms allow commercial use of outputs and bring-your-own keys; the selection is recorded here when each phase begins.

Keep generated assets behind provider adapters and the same validation, content hashing, cooking and provenance pipeline used for imported assets. Credentials belong in OS storage; licensing metadata belongs with the asset reference.

## Evidence and implementation boundary

No asset-generation provider or cook pipeline exists in Phase 0. The screenshot tool captures local rendering and is not an asset generator.

## Consequences and revisit trigger

Provider licensing, output validity, retries, provenance and user approval must be tested when Phase 3 begins. Build-time Claude credentials must never become a runtime dependency.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
