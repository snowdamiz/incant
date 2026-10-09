# ADR 0012: Optional accounts service; local provider secrets

Date: 2026-10-08. Status: proposed for director review.

## Decision

Retain axum and Postgres for optional Incant cloud metadata, licensing and sync. The engine must work offline without an Incant account. OpenAI connection is independent and direct from the client. Windows/Linux use OS credential stores. Following the director's explicit request to eliminate repeated macOS Keychain prompts, macOS uses atomic owner-only local credential files. These files have no additional encryption layer. Tokens never enter projects, logs or the webview.

## Evidence and implementation boundary

The editor and CLI share loopback PKCE, callback and ID-token validation, API-key fallback, refresh serialization and disconnect. Real macOS browser sign-in, refresh, persistence across changed development builds and live agent inference pass. The saved session completed a twenty-task evaluation with 19 passes. The director replaced manual Windows/Linux sign-in with CI builds and automated protocol/storage checks, which passed on all three desktop hosts. Live API-key inference and live revocation remain pending. See [login repair evidence](../spikes/auth-login-repair.md).

## Consequences and revisit trigger

No server exists or has received user credentials. Managed identity versus self-hosted identity remains a director-reviewed operational decision before service implementation; it cannot be decided from a local credential spike. Follow current OpenAI public-client eligibility and registration requirements rather than borrowing Codex tokens.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
