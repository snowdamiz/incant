# ADR 0012: Optional accounts service; local provider secrets

Date: 2026-10-08. Updated: 2026-10-10. Status: decided by director instruction
in the PLAN.md revision 5 decision record (decisions 3 and 16). This does not
approve the Phase 0 gate, and no service is deployed.

## Decision

Retain axum and Postgres for optional Incant cloud metadata, licensing and sync. The engine must work offline without an Incant account. OpenAI connection is independent and direct from the client. Windows/Linux use OS credential stores. Following the director's explicit request to eliminate repeated macOS Keychain prompts, macOS uses atomic owner-only local credential files. These files have no additional encryption layer. Tokens never enter projects, logs or the webview.

## Evidence and implementation boundary

The editor and CLI share loopback PKCE, callback and ID-token validation, API-key fallback, refresh serialization and disconnect. Real macOS browser sign-in, refresh, persistence across changed development builds and live agent inference pass. The saved session completed a twenty-task evaluation with 19 passes. The director replaced manual Windows/Linux sign-in with CI builds and automated protocol/storage checks, which passed on all three desktop hosts. API-key fallback uses automated protocol/storage evidence; a separate live key is not required for the verified OAuth workflow. Live revocation verification remains pending. See [login repair evidence](../spikes/auth-login-repair.md).

## Consequences and revisit trigger

No server exists or has received user credentials. Engine accounts use WorkOS AuthKit as the managed identity provider, reached through WorkOS Connect using standard OpenID Connect. The desktop client is a public PKCE application with no embedded client secret. Self-hosted Zitadel can replace the protocol endpoint, with explicit identity and session migration verification. Passwordless email uses Magic Auth codes; the older Magic Link product is deprecated. The accounts service and the other Incant Cloud services run on AWS. Game player identity is separate: the self-hostable player service uses Steam, Apple and Google sign-in directly and never depends on WorkOS. Creating the WorkOS and AWS accounts is a human-owned action due before Phase 2 cloud sync work. Follow current OpenAI public-client eligibility and registration requirements rather than borrowing Codex tokens.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
