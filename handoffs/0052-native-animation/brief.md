# Native skeletal playback acceptance

Use Claude Opus 5.5 through ACP, with thinking explicitly set to Max and both
settings verified before prompting. Read AGENTS.md and editor/ui/DESIGN.md.
This packet verifies the new native animation path in the actual editor. It is
not a broad redesign or a provider evaluation.

## Scope and exact inputs

Read build.json before doing anything involving the app. Verify the immutable
editor/headless binary hashes and the fixture manifest it names. Use those
copied binaries only; do not rebuild or replace them. The manifest records the
source and checks. If an input is absent or differs, stop that dependent action
and report the mismatch. Do not infer a successful build from this brief.

The fixture is /tmp/incant-animation-native-review-v2/project.json. It is a separate
retained native-review project, authored through the public importer and command
bus. It is not the export proof's deleted source fixture. Its source script
provides repeating move/rotate playback with a stationary camera and entity.
It has one joint and a weighted triangle: rigid joint motion relative to the
unchanged entity is expected; bending is not required from this simple fixture.
The superseded v1 fixture used the wrong journal filename; do not use it. Inspect the
fixture's public project/script data to understand its intended timeline.

CPU and numerical GPU tests already verify native clocks, committed TypeScript
snapshots, atomic controls, source-free startup, retained skin deformation,
single-sided reflection and shadows. Your job is rendered-pixel and native
interaction acceptance; do not represent those earlier tests as your own work.

## Required checks

1. Open the actual WKWebView editor on the supplied project. Confirm that the
   imported rig renders in authoring preview. Select the intended scene/camera
   and start native Play. Verify actual visible deformation or articulation over
   time, including the move and rotate phases. A changing text counter or moving
   whole-object transform alone does not establish skeletal playback.
2. Pause and verify the displayed skeletal pose remains fixed across redraws;
   Step advances it; resume animates again. Stop returns to authored state, and
   starting a fresh run reproduces the initial timeline. Keep the ordinary
   focus/keyboard Play controls usable. Record any missing or clipped content,
   wrong culling, flicker, disappearing triangles or stale pose on restart.
3. Verify viewport resize at approximately 1440x900 and 1000x650 retains the
   animation and usable Play/inspector controls. Check the Animation component
   presentation for an obvious native layout regression; this is not permission
   to expand into a full redesign. Use existing styling conventions for any
   necessary localized visual fix.
4. Compare project, journal and behavior source hashes before/after pure Play
   lifecycle checks. Simulation must not mutate authoring. Keep any deliberate
   UI authoring edits in a separately copied fixture using the normal command
   bus; never edit project JSON or journals directly.

Use a small, purposeful capture set (at most eight original captures plus useful
crops), with exact window dimensions, state/timeline evidence and pixel review.
The fixture is mathematical acceptance data, not final game art or a UI mockup.
Do not invent expected lighting, create decorative assets or change renderer
algorithms to make the test look better. Report backend defects to Astra.

## Native ownership and privacy

Build.json must say native/GPU ownership is granted before any app launch,
capture or GPU helper. This is an exclusive test interval; no unrelated probes.
Reuse/adapt the owned-window helpers from 0050 in this handoff's worktree,
pinning the new binary hash. Check foreground ownership before sending events,
pause if the user changes apps, and restore the prior app/cursor after short
bursts. Capture only the owned Incant window, never the desktop or another app.

No provider turn, account switch, sign-in, sign-out or account dialog is needed.
Do not read credentials or keychain contents. Keep personal identifiers out of
captures and reports. Preserve the user's current login and unrelated projects.

If you change UI source, run meaningful interaction checks, strict TypeScript,
the UI suite (maximum four workers) and production build. Report that such a fix
still needs a newly granted native binary; do not claim the immutable executable
contains source changes made after its build.

Return result.md with the verified model/effort, exact tested build/fixture
identities, actual checks, reviewed captures, limitations and remaining defects.
Quit and reap all owned app/helper processes, list their identities, and
explicitly release native/GPU ownership. Preserve unsuccessful attempts. Commit
the scoped result with Built-by: claude. Do not publish or merge. No reference-
phone, shipping performance, cross-platform or phase-gate claim.
