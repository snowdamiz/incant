# Explicit local Responses provider

The headless `agent` command can now select a loopback streaming Responses server
with `--local-endpoint http://127.0.0.1:8000/v1` and an explicit `--model`. Without
that flag, the saved OpenAI connection remains the selected route. The endpoint
comes only from trusted host configuration; project data and model output cannot
change it. This implements a transport step toward PLAN.md sections 2.7 and 3.1,
not the complete provider-selection interface or a live-model gate.

Only literal IPv4/IPv6 loopback HTTP(S) URLs are admitted. Hostnames, remote
addresses, URL credentials, queries and fragments are refused. The local HTTP
client disables proxies and redirects. `INCANT_LOCAL_API_KEY` is an optional,
separate local-server key held in zeroizing storage. This route never opens the
saved-account store, renews OAuth credentials or falls back to a hosted service.
Diagnostics do not echo endpoint or server-response contents.

The existing bounded Responses parser handles streaming text, images in input,
function calls, usage and terminal completion. Incomplete streams cannot return
executable calls. The local connection retains the shared approval policy,
command bus, host/project permissions, durable token ledger, provenance and Undo.
Boxed provider dispatch forwards the saved-account activity check, so adding a
second transport does not remove the existing account-switch/sign-out fences.

## Verification

On aarch64 macOS with Rust 1.99 and the `ci` profile, locked/offline checks pass:

- 15 provider controls, including six local HTTP controls for endpoint admission,
  request/tool/image encoding, optional credentials, terminal 401, redirect refusal,
  incomplete streams and an approved, reversible command-bus edit.
- Eight agent integration controls, including denial, cancellation, incomplete
  usage, unknown tools and terminal-success requirements.
- 14 headless command controls covering explicit provider selection, journal
  recovery, cumulative budgets, provider setup failure, cancellation/panic and
  output publication failure.
- Strict all-target Clippy for `incant_agent` and `incant_headless`, formatting
  and source diff checks. Independent source review found no blocking issue.

[The receipt](evidence/local-responses-provider.json) binds source, commands,
artifacts, logs and review. No live local model, saved account, GPU, native window
or screen capture was used. Synthetic transport tests do not establish a model's
Responses compatibility, tool accuracy, image understanding or performance.
The editor's provider selector, other provider protocols and live local-model
smoke/evaluation remain separate work. No phase gate is passed by this change.
