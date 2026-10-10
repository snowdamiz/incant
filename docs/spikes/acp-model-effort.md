# Explicit Claude model and thinking effort

Director decision, 2026-10-10: all Claude visual work uses Opus 5.5 with Max
thinking. The handoff runner now selects the model, reads the model-specific
configuration returned by ACP, explicitly sets Max and checks the returned
current value before sending the packet. The same procedure runs on resumed
sessions; a previously selected model or inherited effort is not sufficient.
Missing model/Max options or an unconfirmed setting stop before the prompt.

A read-only audit of the completed 0031 session reported model `opus` (display
name Opus 5.5) and effort `default`. The runner previously pinned the model but
not effort. The audit does not establish the historical effective default effort,
and prior reviews must not be retroactively described as Max runs.

The pinned local ACP adapter 0.88.0 exposes `thought_level` with config ID `effort`.
Its Opus options in this account are Default, Low, Medium, High, Xhigh and Max.
The runner uses the exact `max` value. No alternative is selected or inferred.
Claude Code is 2.1.295. No account settings or credentials were changed/read.

Live ACP accepted and returned Max for the new full-editor design session 0033,
the resumed corrected-camera review 0031 and the new sprite engine review 0032.
Each transport log starts with `Claude session verified: Opus 5.5; thinking: max`.
These are configuration checks, not claims that the visual work has finished.
The director's complete UI redesign request is preserved in packet 0033.

Seven credential-free tool tests pass, including a resumed-session test that
uses freshly returned model-specific effort options, and failure cases for
missing Max, a different selected model and unconfirmed Max. Generated AGENTS.md,
CLAUDE.md and PLAN.md carry the standing instruction. Runtime engine providers
and user accounts are unaffected.
