# Fresh observation routing correction

Checkpoint6's Linux desktop and general macOS jobs failed the same unchanged
player routing assertion. Adding checkpoint resume had moved observation parsing
after opening the scene to calculate the restored absolute end tick. Fresh runs
therefore reported a missing scene before rejecting an unsupported observation
plan. The local unchanged real-process test reproduces that exact failure.

Fresh runs now validate the bounded plan before scene/device startup, using their
known end tick. Checkpoint loads still validate after restoration with the checked
absolute end tick. Both branches share the same parser. The existing exact
unsupported-format assertion, window routing, observation limits and checkpoint
frame arithmetic remain unchanged; no generic failure fallback or retry is added.

The unchanged process regression passes after correction. All 36 player and
observation tests pass, including fractional checkpoint continuation and malformed
observation controls; both packages pass all-target Clippy with native-window
compiled. These tests do not open a window or execute GPU work. Hosted repair
confirmation belongs to the next frozen full checkpoint; checkpoint6 stays failed.

[Source identities, retained red/green and hosted logs, local results](evidence/checkpoint6-player-routing.json).

[Independent source review](evidence/checkpoint6-player-routing-root-review.json) found no actionable issue.
