# Native player window acceptance — review only

Required model: **Claude Opus 5.5**. Required thinking: **Max**, verified through
ACP before this prompt. Preserve this unique packet and all earlier handoffs.

## Resume priority: foreground ownership

Editor review0036 has finished native interaction and its review process exited.
You now own the foreground for this packet. Operate/capture only the game
process you launch, identified by its exact PID/window ID. Do not activate,
close or manipulate any Incant editor window or the user's older Incant process
(previously PID64681). Use Finder only if a controlled focus-loss case needs a
neutral foreground app. Never terminate by a broad application/process-name
match. Record the game PID, close only your own game process after captures,
and state explicitly when native interaction is finished so the benchmark can
resume. Continue against the same immutable binary and no production edits.

## Ownership and goal

Review the visible behavior of a frozen native standalone game window on this
Mac. You own rendered-pixel inspection and screenshots. Astra owns source
correctness and has completed numerical lifecycle/source-free process tests.
Do not edit production Rust, shaders, styling, build settings or other handoffs;
do not rebuild, publish, merge or access credentials. This is a small fixture,
not a game-art/design deliverable or an invitation to redesign the engine.

Work only in this isolated worktree. Put your result, review helpers and capture
evidence under `handoffs/0037-native-player-window/`. The executable and cooked
assets supplied under `artifacts/player/` are immutable. Their original source
and Cargo build storage have been deleted. Verify the executable SHA-256 is
`6b6a53a1ee86fabc9b2c98c7dd4601534bd72521c828bd55c779dbafbe5b3560`
before running; do not substitute another binary.

## Commands and fixture

From this worktree, launch (keep stdout/stderr for bounded final diagnostics):

```sh
artifacts/player/incant_game_player --window artifacts/player/static.scene artifacts/player/audio artifacts/player/clips.bin artifacts/player/actions.json artifacts/player/models artifacts/player/textures 00000000000000000000000000000004
```

`stream.scene` is the equivalent streamed-audio case. Optional final
`--lifecycle-probe` runs 40 accepted ticks with an actual resize, minimize/restore
and explicit surface recreation; it exits automatically and prints JSON.
For the manual acceptance cases use the unbounded command, then close its own
window. The title is **Incant native game**. Do not interact with unrelated apps
or windows beyond controlled focus switching needed for this review.

The fixture has one small model beginning at x=-3, a camera at z=8, kinematic
motion and cooked audio. Hold **D** to move right at 3 world units/second; release
to stop. Holding it indefinitely moves the model off camera, which is expected;
restart for another case. **F** intentionally throws an accepted script error:
the process must close and report failure rather than retaining a live session.
The action table also contains C, but this embedded script does not implement
it; do not use C as a camera-error test.

## Acceptance cases

1. Startup presents the fixture in the actual native window. Capture the window
   and record any blank/stale frame, clipping or unexpected content. Judge only
   this fixture, not the completeness of a future game UI or art direction.
2. Hold/release D briefly. Motion and stopping should be coherent. Losing focus
   freezes simulation/audio; restoring focus resumes with held controls reset
   until a new OS event. Do not infer a performance gate from smoothness.
3. Resize within the supported physical 16–1920 × 16–1080 range, minimize,
   restore and switch focus. Check that resumed content is current and usable,
   and that repeated ordinary operations do not leave stale/blank frames.
   Extremely tiny windows naturally obscure the small fixture; separate that
   from lifecycle corruption.
4. Close normally and separately use F. Confirm clean window teardown and
   intentional-error process failure. Record exact stdout/stderr and exit
   status when available. Do not claim GPU or device audio can be rolled back.
5. Run the supplied numerical lifecycle probe once if useful to relate the
   observed window states to its report. It explicitly recreates a real surface;
   it does not inject an OS-generated surface-loss event.

Use local native automation/capture tools available to you. If a tool or
permission is unavailable, report exactly what was blocked; do not fabricate
screenshots, approve unseen frames or change model/effort. Optional speaker
observations must be clearly distinguished from the already-tested PCM data.

## Evidence and result

Read `docs/spikes/native-window-player.md` for lifecycle/limits and
`docs/spikes/evidence/native-window-player-2026-10-10.json` for exact numerical
evidence. Preserve all bounds: no added per-tick scene cloning, JSON, loading,
geometry rebuilding or new rendering path. This review must not add runtime
code or alter the source-free fixture.

Return `handoffs/0037-native-player-window/result.md` with verified model/Max,
binary hash, host details, each tested case, capture paths and SHA-256 hashes,
specific accepted scope, actionable defects, and limitations. Commit only
review files with `Built-by: claude`. No phase/performance gate, mobile, installer,
signing, full game release or pixel-placement feature completion claim.
