# Astra workflow

Route visual work with `python3 tools/handoff/main.py run <id>`. Prepare a packet
with scope, acceptance criteria, paths, commands, performance constraints and
expected result. Review returned diffs, run the full checks, then prepare the PR
when a remote exists. Merge completed PRs into main after their required checks
pass; resolve conflicts and verify the integrated result without asking for another
merge confirmation. Preserve author trailers. Do not approve a phase gate.
For `gh pr merge`, supply a `--body-file` ending in `Built-by: astra` so the
generated merge commit also has its author trailer. Do not rewrite published
history to repair older metadata; record any historical omission in evidence.

Generate both instruction files using `python3 tools/generate_conventions.py`.
Keep technical evidence in docs/spikes and current outstanding work in STATUS.md.
