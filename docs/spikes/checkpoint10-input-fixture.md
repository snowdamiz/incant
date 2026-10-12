# Replay version refusal fixture

Checkpoint10 retained frozen main137 and completed all eight workflows: 20 of
23 required checks passed. Linux/Windows desktop and macOS general checks all
failed the same legacy headless input test. The original UI input implementation
intentionally accepts recording versions 1 and 2; the older table still expected
version 2 to fail. No release promotion or rerun was made.

The unchanged test reproduces locally with exit 101. Its unknown-version sentinel
is now 3. Existing future-device, tick-rate, start/range, oversized-file and
output-absence assertions remain; refusal cases also require their specific
diagnostic. A positive control runs the same 60-tick physical-input clip under
versions 1 and 2 and compares state, input and logs. All three actual headless
input cases and scoped Clippy pass. Production parsing is unchanged.

Actual main140 integration changes only unrelated window source and documents
relative to the tested main139. The fixture bytes remain independently reviewed.
[Retained failure, checks and review](evidence/checkpoint10-input-fixture.json).
Hosted confirmation remains pending the next frozen full checkpoint.
