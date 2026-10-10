# Headless gameplay assertions

`play --assertions FILE` evaluates data-only checks at specified absolute game
ticks. It reads the isolated runtime snapshot, behavior state and normalized input;
all gameplay edits still use the simulation command bus. The file cannot execute
code or grant access to project files, network, accounts or the host shell.

The version-1 format is `incant-play-assertions`. `checks` contains unique names,
absolute `tick` values, JSON Pointer `path` values below `/state`, `/script_state`
or `/input`, and a typed `expect` condition:

- `equals`: structural JSON equality, including integer/float numeric equivalence.
  Large integers are not rounded through f64 for comparison.
- `approx`: a finite numeric value and a finite nonnegative absolute tolerance.
- `range`: finite inclusive minimum/maximum bounds.
- `exists`: presence or absence of the path. A present JSON null differs from a
  missing path; comparing missing data to null does not pass.

Plans are validated in full before output creation or renderer initialization.
There must be 1–256 checks, the file is at most 256 KiB, names are at most 120 UTF-8
bytes, paths at most 1,024 bytes, and ticks must lie within the requested playback
interval. Malformed pointer escapes, duplicate names, unknown versions/fields,
unsupported conditions and invalid numeric bounds fail explicitly. Checks are
sorted by tick, preserving source order within a tick. Initial and restored
checkpoints are evaluated before advancing gameplay; subsequent checks observe
completed physics/script ticks. Input seeking occurs before checkpoint checks.

## Failure and output behavior

A successful simulation always reports `completed: true`. `passed` says whether
all assertions passed; with no assertion file it is true. The `assertions` array
contains each check, its result and an actual-value JSON preview capped at 1,024
bytes (with an explicit truncation flag). Missing paths report a null preview;
present JSON null reports the text `null`. Previews do not replace full comparison.

Failed checks do not abort the remaining simulation, so one run collects every
scheduled result. The CLI prints the structured report and exits nonzero. Script
logs remain available. If frame output was requested, the completed diagnostic
`report.json` and actual frames remain available even on assertion failure. A
requested game save is never published on assertion failure and is omitted from
the report. Script/physics/render/IO failures retain the existing stricter rule:
they cannot claim a completed report or publish a game save.

This satisfies the runner's assertion-result/exit-status path. It does not claim
that the Core Sample exists, that its device performance gate passed, or that
arbitrary assertion scripts, agent play tools or native test controls are ready.

## Verification

Five new separate-process CLI tests cover initial/intermediate/final ticks,
sorted results, unchanged authoring files, missing/null distinction, pointer
escaping, numeric types and large integers, bounded multibyte diagnostics,
invalid/oversized plans before output creation, failed-save suppression, and
absolute checks against saved/replayed input. Existing no-assertion play/save
behavior is covered by the workspace tests.

The strict TypeScript character probe now checks its starting position, a real
jump and airborne tick-75 save, focus recovery, wall contact and landing. All
checks pass for uninterrupted, repeated and saved/resumed runs, with exact final
state and log suffix. Adding an intentionally incorrect landing check produces a
nonzero exit, the expected failure report and no save file. This probe runs in
hosted source and Windows/Linux desktop CI.

The existing ignored GPU test is extended to compare the retained failure frame
against its previously captured initial frame and verify the failed report/no-save
behavior. It is compiled locally, but not executed during the director's screen
capture pause. Hosted source and both desktop jobs passed the actual GPU check. No new local computer use or
captures are part of this increment.

The final local tree passes 195 Rust tests, 315 UI tests, five tool tests,
workspace Clippy, generated contracts and strict TypeScript. The native release
build/package also passes without opening the app. All three final hosted checks
passed at `7379f9c`; PR #28 merged as `734d008` with the identical reviewed tree.
The main app was rebuilt without opening. These are synthetic engine/CLI checks, not the full Core Sample gate.

Machine-readable evidence: [play-assertions-2026-10-09.json](evidence/play-assertions-2026-10-09.json).
