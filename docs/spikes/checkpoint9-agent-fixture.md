# Checkpoint9 agent failure fixture

Frozen main132 failed the Linux and macOS credential job at
`unknown_failure_redacts_provider_content_and_reserves_unknown_usage`. Its fixed
20,000-token allowance is smaller than the current generated tool input, so
production correctly refuses admission before the intended provider error. The
unchanged local test reproduces the exact failure.

Only this test changes. Its allowance includes the current complete tool catalog
and fixed prompt/output headroom, and its output cap equals that allowance. The
actual request must be admitted and capped below that maximum. The original
redacted error, no remaining budget and unchanged revision assertions remain;
exact full charge and a nonzero request are explicit. The independent test that
refuses a tiny budget before any provider call is unchanged.

All eight integration tests, the full agent suite and all-target Clippy pass.
[Evidence](evidence/checkpoint9-agent-fixture.json) preserves hosted and local
failures plus passing logs. This changes no production budgeting, error handling
or provider behavior. Actual cross-platform repair validation awaits a later
frozen release checkpoint; checkpoint9 is not promoted.
