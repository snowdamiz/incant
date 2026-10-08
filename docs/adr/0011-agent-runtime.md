# ADR 0011: Local provider loop and shared typed tools

Date: 2026-10-08. Status: proposed for director review.

## Decision

Run the agent locally with provider-agnostic traits. OpenAI Responses is the first direct provider. Generate patch arguments from the shared Command schema. Query, patch and real screenshot tools support the spike; all project content is untrusted data.

## Evidence and implementation boundary

Mock provider tests verify denial, cancellation, budgets, atomic edits, provenance and streamed completion handling. The live twenty-case harness checks exact scope, all three tools and full undo. Live credentials remain unconnected.

## Consequences and revisit trigger

No mock score can pass the live gate. Token caps are conservative; dollar pricing is deliberately unavailable until an authoritative model price is known. Each patch is undoable; grouping a multi-patch conversation into a single revert remains later work. The complete Appendix A tool set is not implemented.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
