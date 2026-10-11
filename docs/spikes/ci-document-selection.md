# Conservative documentation-only general checks

The general `checks` job previously occupied a macOS runner for a full engine
build when a plan/conventions-only edit changed no build or test input. A small
Linux classification job now selects the existing full macOS path or a Linux
documentation path under the same `checks` identity. This does not filter main
source validation or change any other workflow/check matrix.

The short path accepts only modifications/additions of regular, non-executable
files in this explicit reviewed set:

- PLAN.md, STATUS.md, AGENTS.md and CLAUDE.md;
- docs/conventions/shared.md, astra.md and claude.md;
- docs/adr/0012-accounts.md and docs/adr/0014-ci.md.

These files were audited against main64 `77d86b8`. The convention generator and
Claude handoff runner read the instruction sources. No compiler/test/build reader
of the two ADRs was found; README links the ADR directory. Other Markdown,
READMEs, spikes, fixtures, new ADRs and unknown paths require the full path.
The short path verifies regular UTF-8/nonempty/NUL-free documents, exact-diff
whitespace, generated conventions and the classifier's real-Git/failure tests.
A literal-reader regression guard makes new common include/read references to
these files require policy review. It is not general dynamic program analysis;
future indirect readers still require an explicit review of this allowlist.

PR classification uses the base/head merge base. Main pushes require the before
commit to be an ancestor of after. Raw NUL-delimited Git records preserve both
file modes; deletes, renames, symlinks, executable mode changes, empty diffs,
unknown paths, invalid revisions, missing/shallow history and parse/Git failures
select full validation. A failed classifier job also runs the full checks job;
normal cancellation of superseded runs remains respected. Dispatch always runs
full checks. Only fixed scope strings and validated hexadecimal commit IDs enter
workflow outputs.

All 45 prior full-path command strings remain identical and in the same order.
No engine, GPU, shipping, schema, SDK, bridge, UI or credential assertion is
removed for source changes. The classification workflow and helper themselves
are unknown paths and therefore require full validation when edited.

Local verification: 82 tool tests, including real temporary repositories,
malformed/incomplete event and Git failures, unsafe file modes, convention drift,
workflow fallback/identity checks and recorded historical diffs. Original PLAN8
commit `638bf8f` selects docs; UI/runtime source changes select full. Actionlint,
generated conventions and whitespace checks pass. Hosted duration and actual
queue reduction remain unmeasured; full-history checkout is an explicit Linux
classification cost. No profile optimization or broad graph-based skip follows
from this selector. The credential dependency classifier remains report-only.
