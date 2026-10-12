# Windows desktop checkpoint budget

Checkpoint13's Windows desktop job was cancelled by GitHub's one-hour job limit.
Its successful pre-export stages consumed 53m27s: workspace tests/compile 28m20s,
native-window Clippy 4m03s and tests/compile 9m10s, followed by numerical GPU checks
and tool builds. The first fresh authored export received only 6m40s before
cancellation. The separate shipping runtime/symbol checks and later scenarios
were never reached; this is not a test-failure diagnosis for those steps.

Only this matrix job's Windows allowance changes to 120 minutes. Linux stays at
60 minutes, the authored-export step stays at 45 minutes, and every command,
required context, trigger and cache policy remains unchanged. The main fast check
is unaffected. Both fresh shipping targets remain private and uncached. Splitting
jobs would require a separate aggregate/artifact boundary; avoiding native-window
feature-graph rebuilds is a separate optimization requiring coverage comparison.
The increased allowance does not predict or establish hosted completion.

Actionlint, existing CI/release policy tests, conventions and diff checks pass.
[Exact raw log, job annotation and step timings](evidence/windows-desktop-checkpoint-budget.json)
are retained. Checkpoint13 remains failed; its successor must pass all 23 exact-head
required checks before release promotion.

Independent source and execution-evidence review accepts this bounded workflow
repair. Exact-head fast CI and the successor full checkpoint remain required.
