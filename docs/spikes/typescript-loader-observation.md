# TypeScript dependency loading observation

Release checkpoint46, candidate `78a7c4c5e77292c3bcc1aa04d95ffae5275bd328`,
failed the Windows desktop job's spatial starter test. Its only completed
compiler milestone was `driver_entry` at 311 microseconds. The parent's reported
elapsed time was 10,955,531 microseconds, including cleanup after the unchanged
eight-second deadline. The prior successful main270 diagnostic and the earlier
checkpoint32 failure remain recorded in [the original observation](typescript-cook-phases.md).
Neither success nor failure identifies TypeScript, SWC, or host contention as
the cause. The failed test has not reached source checking or gameplay.

The driver now imports the pinned TypeScript and SWC packages separately before
loading the compiler module. Its later static imports reuse those module
instances. Two new fixed milestones identify the completed dependency loads.
Observation version 2 has ten ordered names; it keeps the same 4 KiB bound,
integer validation, partial-write handling and exclusion of source text, paths
and arbitrary child content. The deadline, child cleanup, type checking, artifact
validation and fallback behavior are unchanged. This is diagnostic work, not a
timeout repair or a claimed performance improvement.

The existing manual Windows workflow now selects both fallback and template
integration targets. Success requires both three-test harnesses and five complete
successful cooks: one authored fallback attachment and four scripted starters.
The empty starter uses no authored script. Expected deleted-source refusal is
retained. There is no retry, deadline override or release-check substitution.

On this Mac, all 23 Node compiler controls pass, including artifact-byte parity
with observation enabled/disabled. Four Rust parser/deadline controls and six
fallback/template tests pass; the latter emit all ten phases for five cooks.
Strict Clippy with the explicit `typescript` feature, formatting, actionlint,
conventions and six release-policy controls pass. Initial local parser/lint
commands omitted the optional TypeScript feature; the parser selected zero
tests and does not qualify it. Their logs remain retained, followed by corrected
explicit-feature commands. No source change or test tolerance was needed.

The [portable evidence packet](evidence/typescript-loader-observation/manifest.json)
retains the failed hosted log, source patch, local logs, command receipts and
that qualification correction. Its verifier checks identities, actual counts,
the five observations and the preserved eight-second deadline. The Windows
one-shot run remains pending; even a passing run will be only one observation.
Checkpoint46 remains failed until reviewed main work passes the complete frozen
release candidate suite.
