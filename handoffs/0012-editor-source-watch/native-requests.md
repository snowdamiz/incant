# Native capture requests: Problems row wrapping

Status: fulfilled. No further native evidence is requested.

Astra integrated the wrapping fix d497e2c as 3cb5c8b, rebuilt the release app and
supplied the captures below. They are kept private in the ignored
`artifacts/native-source-watch-review` and are not committed. Claude reviewed them in
`result.md`. Browser fixture captures in `screenshots/after` remain fixture-only and
are not native evidence.

| Request | Capture | Outcome |
| --- | --- | --- |
| 1. Error at 1000×650 | w13-wrap-error-minimum.jpg | Supplied and passed. The message wraps to two lines and ends with the line and column. |
| 2. Error at 1440×900 | w14-wrap-error-wide.jpg | Supplied and passed. One line, path at the right. |
| 3. Optional recovered state at 1000×650 | w15-wrap-recovered-minimum.jpg | Supplied and passed. "No problems", Attached, history unchanged. |
| 4. Optional entity focus at 1000×650 | Not captured | Not performed. The fixture project has no entity diagnostic. Focus evidence stays fixture-only. |

Astra also supplied w13-wrap-before-error.jpg, a 1440×900 baseline with no error before
the malformed write. It was reviewed as context.

If a later handoff adds a project with an entity diagnostic, a native capture of the
wrapped entity row with its keyboard focus ring at 1000×650 would close request 4.
