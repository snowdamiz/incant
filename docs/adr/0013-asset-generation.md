# ADR 0013: Pluggable generation providers

Date: 2026-10-08. Status: proposed for director review.

## Decision

Keep generated assets behind provider adapters and the same validation, content hashing, cooking and provenance pipeline used for imported assets. Credentials belong in OS storage; licensing metadata belongs with the asset reference.

## Evidence and implementation boundary

No asset-generation provider or cook pipeline exists in Phase 0. The screenshot tool captures local rendering and is not an asset generator.

## Consequences and revisit trigger

Provider licensing, output validity, retries, provenance and user approval must be tested when Phase 3 begins. Build-time Claude credentials must never become a runtime dependency.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
