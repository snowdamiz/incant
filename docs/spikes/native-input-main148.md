# Native input corrections on main148

The integrated changes repair macOS managed text-client destruction and keep
text-field pointer release from submitting the field. Explicit activation still
submits, and buttons retain matching-release activation. All five implementation
and workflow inputs are byte-identical to the independently accepted producer
commits. The actual-main merge resolved only STATUS; current manifests, lock,
animation, UI and audio checkpoint behavior are preserved.

The frozen union passes 127 CPU tests in 20 result groups, zero failed or ignored,
all-target Clippy for UI/input/native-window player, wasm32 UI, runtime dependency
guards, tool tests, formatting and conventions. The macOS lifetime example was
compiled as a test harness; its main was not run by these integration checks.
The original source-bound hidden AppKit proof retains both the old abort and
768 passing contexts/256 stale refusals after the repair.

No native-window/GPU/input interaction was performed for this integration.
Claude0058 owns the separate visible typing/IME follow-up. These results do not
approve complete IME, appearance, device or phase gates. The failed checkpoint11
remains frozen while its separate Linux numerical correction is prepared.

[Exact frozen sources, commands and retained logs](evidence/native-input-main148.json).

[Independent final integration review](evidence/native-input-main148/root-final-review.json) accepted the source identities and completed CPU checks.
