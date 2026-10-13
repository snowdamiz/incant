# Core Sample locked-exit proof correction

The original 0060 `locked` run did not reach the lift alive. It lost at tick 608
and froze 3.510885494 m from the exit, whose victory radius is 1 m. Its assertion
checked only zero pickups and no victory, so it passed without exercising the
inventory gate. The original source, result and raw report remain historical
records; this follow-up corrects only the numerical probe and input recording.

The new recording keeps the accepted win input through tick 1273, after the first
three pickups, then crosses the upper mesa to the exit while avoiding pickup 4.
The exact retained CI player executes normal movement, collision and gameplay.
At every committed tick 1559–1588, the native observation API reports both player
and exit positions, the first three pickups absent and the fourth still present.
The oracle checks actual values as well as each native result, complete logs,
positive health and no victory or loss. The stopped player remains
0.050727–0.050750 m from the exit with health 2 and 3/4 pickups for all 30 ticks.

All three actual CPU scenarios pass. The win and loss reports and PCM bytes
match the historical f5 runs exactly. Seven focused oracle regressions pass,
including dead-before-arrival, corrupted `exists` actuals, wrong owner, missing
observations, dropped logs, victory and a one-tick excursion beyond the radius.
The initial correction assumed request order, but native checks are sorted by
tick and name; that failed helper result is preserved. Final proof003 uses only
explicit PATH/TMPDIR variables, with HOME neither changed nor inherited.

The reused package is bound to player `565f1592`, project `443f3243` and controller
`58d7007d` through its producer receipt and every cooked data hash. Controller,
original win recording, authoring builder, level and UI/art remain unchanged.
The builder's authored tree is recursively hashed before and after play. No
compiler, native window, GPU, image inspection or device work was performed.

The unchanged `verify.py` seven-scenario suite describes the earlier numerical
collider room. It is not seven scenarios of this authored 45-entity mesa level.
This follow-up uses the actual f5 authored level with three recorded scenarios.

The historical 16 kHz sink accommodates the retained player's one-million-frame
cap. Accepted streaming commit `49e18d9a` separately proves this same f5 project
at 48 kHz for 2156 ticks (1,724,800 frames, victory at tick 2066), with source,
map and build inputs deleted. It remains a separate integration dependency.
No old binary is relabeled as fixed.

[Source and lossless evidence](evidence/core-locked-exit.json) bind freeze002 and
final proof003. [Independent root review](evidence/core-locked-exit-review.json)
is accepted; publication is pending. Original 0060 visual acceptance stays with
Claude. Exact offscreen-harness source identity still needs qualification:
`sources.json` claims `d0863272`, while committed source hashes to `9f453fd0`.
Live HUD/audio/restart integration and native/device/performance gates are not
completed by this probe.
