# Windows conformance checkpoint split

Checkpoint 16's Windows conformance job exceeded its 90-minute limit. Both the
baseline and MSVC `/O2` suites passed 484 tests in 87 groups, with no failures or
ignored tests. The optimized test build took 26m38s and Clippy took 24m23s, after
the baseline step had already consumed 26m14s. Cancellation occurred while the
optimized development child ran; neither fresh shipping step had started.

Four independent Windows jobs now run the unchanged baseline, optimized,
standalone shipping, and composed shipping commands. Each retains the existing
90-minute limit and download-only cache. No target or shipping artifacts cross
jobs. The existing required `conformance (windows-latest)` context runs even when
a child fails and requires actual success from all four named jobs. Missing,
malformed, skipped, cancelled, neutral, and failed results refuse the gate.
macOS/Linux commands and all 23 required context names remain unchanged.

Six gate regressions, six release-policy tests, actionlint, the complete Python
and Node tooling suites, conventions and diff checks pass. The exact source has
independent review. No hosted success or measured speedup is claimed yet, and
the static development child graph remains unchanged. The next frozen checkpoint
must run every required context; checkpoint 16 remains failed and release unchanged.

[Manifest, original failure and lossless check logs](evidence/windows-conformance-split.json).
