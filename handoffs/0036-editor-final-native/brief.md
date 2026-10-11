# Final isolated native editor acceptance

Required Claude Opus 5.5, thinking Max; the ACP runner must explicitly verify both.
This is final native acceptance of the existing plain editor, not a redesign. The
source is reviewed28e5491, presentation65e1717;524 UI tests, typecheck/build,40 tools
and exact custom-protocol native build pass. Read0035/result.md and its final
reconciliation. Four new observable tests fix import/reimport completion focus.
Do not run Cargo, change source/style or publish. Report any defect to Astra.

The director explicitly reauthorized scoped native computer use. This supersedes
the old0035 native ban ONLY for the separate review app below. Never interact with,
focus, close, inspect account content from, or capture the user's older0d14f4f app.
No full-screen screenshot. Never read credentials or the Incant account directory.

## Prepared production binary and isolation

Astra copied the exact rebuilt binary without modifying it into:
`/private/tmp/incant-native-review-28e5491/Incant Review 28e5491.app`
Bundle ID `dev.incant.review.28e5491`, executable
`Contents/MacOS/incant_review_28e5491`, SHA256
`51599f65f3e96458d706a81c81b71bc392e93812923f11549e4ae0bd3024b79e`.
Its own project is
`/private/tmp/incant-native-review-28e5491/fixture/review.incant.json`.
Fixture authoring used incant_headless/command.execute/project.save; provenance
and inputs are in that directory. Project edits must continue through native UI
commands. No direct JSON changes. The project contains actual current Camera
perspective/orthographic/pixel_perfect variants, a compound Dumbbell and Atlas
Sprite. Existing atlas/audio are imported; new.atlas.json,new.wav,new.ogg and
ordinary.png are prepared for actual native imports; refused.json must be refused.

Launch ONLY through `/usr/bin/sandbox-exec -f
/private/tmp/incant-native-review-28e5491/review.sb` followed by the exact executable
and project arguments. Do not use open (it would bypass the sandbox), do not remove
or weaken the profile. It denies network and all read/write access to the shared
`~/Library/Application Support/Incant` store. Synthetic read/write/network probes
all failed EPERM; isolation-proof.json records this. Do not test the real store.
The app will truthfully report account storage unavailable; do not click Connect,
add accounts, sign out or touch external settings. Connected/switching behavior
was accepted through account-free browser fixtures in0035; native acceptance here
covers the actual compact unavailable/error dialog and focus, without real auth.

You may launch/inspect ONLY this prepared temporary bundle/project and write your
own capture scripts/evidence under this handoff worktree or its ignored artifacts.
This narrowly overrides the runner's generic no-external-edit rule for test app
launch and UI-owned project changes. Record its PID and unique executable path.
Target/assert that PID/bundle before every native input; do not use generic Incant
process/window names (window title is still Incant). Use process-scoped AppleScript
accessibility or appropriate native automation; confirm the review process is
frontmost before input. Capture only its exact window ID, never screen-wide.
The user's other app may remain running throughout. No global keystrokes unless
the exact review PID is asserted frontmost immediately before dispatch.

## Required native review

Review normal1440x900 and minimum1000x650/narrow Inspector as feasible, recording
actual size. Preserve five requirements: remove useless collider explanatory text
and permanent entity/ID rows; compact professional account dialog; useful searchable
shortcut table with actual Mac notation; profile/avatar-only titlebar control;
plain controls without decorative edges/glows/backdrops/nested ornament.

Observe actual native account unavailable/error and Escape/close focus; shortcut
search/platform labels; all three real Camera variants; compound-part list and
metadata removal; sprite/atlas details, atlas Color-only and ordinary-image editable
interpretation; real WAV/OGG/atlas import path validation and completion keyboard
focus. Exercise relevant keyboard/menu transitions. Import through the native UI
only; if a route isn't accessible, state the exact limitation instead of fabricating
coverage. No game/runtime/performance/phase acceptance claim. Browser findings3–10
in0035 remain scoped follow-ups unless this source causes a new regression.

Return `handoffs/0036-editor-final-native/result.md` with model/effort, exact binary
and source identity, sandbox/app PID, screenshot hashes, observed acceptance and
honest limits. Commit a small useful account-free screenshot set/evidence with
Built-by: claude. Stop only your review app after finishing (verifyPID/path first),
never the user's app. Do not push/merge. No additional source changes without Astra.
