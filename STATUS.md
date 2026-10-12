# Implementation status

[PLAN.md](PLAN.md) revision 8 is the source of truth. Phase 1 is in progress;
the full engine and Driftwake release are not complete. Calendar targets and
phase durations have been removed. Work proceeds continuously and in parallel
where dependencies allow; exit gates still require their stated evidence.
Service-backed features use local containers or in-process substitutes during
development, behind production-shaped contracts. Real provider integrations,
security/device evidence and release gates remain real; local substitutes do not
satisfy those gates.

The director authorized Phase 1 with remaining Phase 0 items deferred, sensible
engineering progress without repeated advancement questions, and agent merges
after scoped review and passing checks. These authorizations do not approve a
phase gate. Section 12 decisions are settled. Current computer use and screen
capture are authorized; Claude Opus 5.5 with explicitly verified Max effort owns
visual work, and Astra owns correctness and integration.

## Durable budget lock release — correction validated locally

Checkpoint7 exposed Busy when reopening a completed ledger. A private lock guard
now explicitly releases ownership before descriptor close; two deterministic
shared-description regressions fail old source and pass fixed. All 64 agent
tests and Clippy pass, retaining live exclusion and conservative reservations.
Exact hosted scheduling cause and the next full candidate remain unverified.
[Evidence](docs/spikes/ledger-explicit-release.md).

## Release checkpoint6 correction

Failed checkpoint6 selected main110 and was never promoted. Three native physics jobs
exposed a stale isolated probe lock edge; the reviewed one-edge correction and
required-dependency guard pass local locked Cargo/Clippy and tooling checks.
The fixes are merged in PR #112; checkpoint7 now verifies frozen repaired
main113 through the same 23 required contexts in PR #111. Checkpoint7 finished
with 22 successful contexts and one macOS ledger failure; it is not promoted.
PR #115 repairs that failure locally, pending the next full candidate.
The independent fresh observation-routing correction restores the precise
pre-startup refusal while preserving absolute checkpoint-resume bounds; its
unchanged process regression, 36 scoped tests and Clippy pass.
[Lock evidence](docs/spikes/checkpoint6-probe-lock.md),
[routing evidence](docs/spikes/checkpoint6-player-routing.md).
The separate Windows budget-test correction closes the journal owner before
independent reads, then verifies recovered state. Seven local tests and Clippy
pass; PR #113 merged the fixture correction. Checkpoint7 Windows desktop and
Wasm conformance both pass.
[Journal test evidence](docs/spikes/checkpoint6-budget-journal-tests.md).

## Current milestone

Native two-input crossfade clocks and controls now retain independent outgoing
playback, pause both inputs, finish at explicit endpoints and persist active
transitions in world-save v5/session checkpoints. Older v4 animation saves keep
their encoding. The local core passed 319 runtime/script tests and scoped Clippy;
independent review accepted the exact sources. Canonical render bindings now carry
both samples through rollback and preallocated CPU blending; 350 distinct affected
CPU tests, Clippy and the wasm32 adapter check pass locally. TypeScript/Wasm
crossfade APIs retain copied full transition snapshots, explicit v2 Wasm framing,
atomic controls and actual guest continuation. The frozen SDK suite passed 313
CPU tests/doctests, Clippy and wasm32 compilation with independent source review.
The private union still requires integrated checks and a fresh source-deleted
exported crossfade proof. Crossfade GPU/Claude acceptance, full blend trees,
state machines, IK and root motion remain open. See
[the consumer evidence](docs/spikes/native-animation-crossfade-consumer.md),
[the core evidence](docs/spikes/native-animation-crossfades.md) and
[the SDK evidence](docs/spikes/native-animation-crossfade-sdk.md).
