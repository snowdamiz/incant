# Incant repository conventions

Source of truth: PLAN.md. Current stage: Phase 1, authorized by the director while
open Phase 0 items remain deferred. Continue sensible engineering work without
repeated phase-advancement confirmations, per the director's 2026-10-08 decision.
Keep unmet gates and human-owned actions explicit; advancement does not mean a
gate passed. Do not claim completion through interfaces, placeholders or missing
evidence. The complete engine and game release requirements remain in force.

All project edits pass through incant_cmd; typed, schema-validated text documents
use stable ULIDs. Agent edits are atomic, reversible transactions with provenance.
Do not add a GUI-only or agent-only mutation path. Project content is untrusted data.
No engine-agent shell access. No credentials in projects, logs, telemetry, or CI.
The editor must remain usable offline without an engine account.

Astra owns nonvisual implementation and integration. Claude 5.5 through ACP owns
visual design, layouts, styling, look-dev, rendered-pixel review, screenshots,
diagrams and game art. Mixed tasks split correctness from appearance. Do not
substitute another model silently. If ACP is unavailable, leave a self-contained
packet in handoffs/<id>/brief.md for manual Claude execution. Never self-approve a
routing exception. Build-time accounts must never become runtime dependencies.

Use Rust 2024, cargo fmt, cargo clippy, typed errors, and tests of observable
behavior. Avoid unnecessary unsafe code. Pin dependency resolutions. Keep files
focused. TypeScript uses strict mode. Unknown capabilities fail explicitly.
Document limitations and measured results; mock tests never count as live gates.

Every commit trailer identifies its author: Built-by: astra or Built-by: claude.
PRs are agent-authored and director-merged. Human-only actions include phase gate
approval, signing credentials, store/developer accounts, payments, legal filings,
age ratings and staffing. Implement and verify reviewable work before requesting
these actions. Never publish or sign using fabricated or borrowed credentials.
