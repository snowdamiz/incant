# Astra workflow

Route visual work with `python3 tools/handoff/main.py run <id>`. Prepare a packet
with scope, acceptance criteria, paths, commands, performance constraints and
expected result. Review returned diffs, run the full checks, then prepare the PR
when a remote exists. Do not merge your own PR or approve a phase gate.

Generate both instruction files using `python3 tools/generate_conventions.py`.
Keep technical evidence in docs/spikes and current outstanding work in STATUS.md.
