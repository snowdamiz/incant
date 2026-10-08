# ADR 0012: Optional accounts service; local provider secrets

Date: 2026-10-08. Status: proposed for director review.

## Decision

Retain axum and Postgres for optional Incant cloud metadata, licensing and sync. The engine must work offline without an Incant account. OpenAI connection is independent, direct from the client, with only nonsecret account metadata outside OS secure storage.

## Evidence and implementation boundary

The CLI implements loopback PKCE, callback and ID-token validation, API-key fallback, refresh serialization and disconnect. macOS live authorization did not complete; three-OS authentication and keychain evidence remain pending.

## Consequences and revisit trigger

No server exists or has received user credentials. Managed identity versus self-hosted identity remains a director-reviewed operational decision before service implementation; it cannot be decided from a local credential spike. Follow current OpenAI public-client eligibility and registration requirements rather than borrowing Codex tokens.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
