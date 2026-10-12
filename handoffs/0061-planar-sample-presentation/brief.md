## Priority revision — actual HUD and current runtime, 2026-10-12

Resume with Opus 5.5 and explicitly verified Max. This revision supersedes older
statements below that HUD bindings and native-play UI are unavailable. Your private
branch now includes main169 (4fc128f9), actual native HUD APIs, localized runtime
text, and headless native-play UI viewport support. Astra independently reran your
first art/builder commit: public transactions, Undo/Redo/reopen and all seven native
scenarios passed. This is correctness acceptance, not native-window acceptance.

Astra has supplied source-reviewed optional presentation bindings in gameplay.ts.
Strict TypeScript passed; the separate numerical runtime/export proof is running.
Do not edit that file, verify.py, export_verify.py, tsconfig.json or presentation.py.
Build on your original Pearl Cove art and level with a restrained, polished actual
engine HUD, readable live progress and clear terminal states. Own the artwork,
scene.py, localization copy, widget layout and rendered critique. No static labels
masquerading as live state and no baked text textures. Use ordinary imported fonts,
UI documents and atomic public authoring commands.

Optional props: flipbooks {player?, boat?}; hud {table, status, prompt}; results
{won, lost, active, inactive}. Use distinct canonical ULIDs. Inspect the supplied
source for exact validation. The controller pauses bound state flipbooks at first
bind; player frames are 0 playing, 2 won, 1 lost; boat is 1 when all pickups are
collected and 0 otherwise. HUD keys are hud.collecting (collected,total), hud.ready,
hud.won (total), hud.lost. Supply localized StringTable entries. Runtime locale,
pseudo-localization and fallback refresh are supported. Terminal label nodes use
the supplied active/inactive parent bindings to expose the matching result. The
prompt is deliberately blank: whole-session restart has not been implemented, so
do not advertise a nonfunctional restart control. Report restart as open.

Build a fresh matching headless from this updated branch before runtime evidence.
The exclusive incant-core-presentation-target is still assigned to you and idle.
Do not reuse the old released lease or permit guard. Acquire fresh lease/markers
under ~/.codex/incant-evidence/0061-hud, retain source hashes and actual commands,
and use at most two cargo jobs. Preserve prior evidence as historical. Exercise
keyboard/gamepad win, loss and locked exit with the authored HUD and review actual
engine offscreen renders, localized text and result states. Native window review
still waits for ~/.codex/incant-evidence/0060/released and at least 20 seconds real
user idle, fresh capture lease and window-only capture; yield on any user input.
Do not wait idly if this is unavailable: complete offscreen/art/correctness work
and explicitly return that remaining native review. No phone/performance claim.
Update result.md/PRESENTATION.md with exact new scope, real evidence and remaining
gates. Commit your own changes Built-by: claude. Never merge shared branches.

# 0061 — Playable 2D sample presentation

Use Claude Opus 5.5 over ACP with thinking explicitly Max, verified before the
prompt. Read CLAUDE.md and PLAN.md revision 8. You own original game art, visual
design, tile layout, sprite presentation, UI and rendered review. Astra owns
runtime/gameplay correctness and integration. Do not substitute model or effort.
Do not read credentials, edit personal memories, publish, merge or alter accounts.

## Outcome

Create a small, polished, immediately understandable 2D sample with real authored
tiles and sprites using Incant's native engine. The new `samples/planar/gameplay.ts`
is a working top-down XY controller: move, collect every pickup, reach an unlocked
exit and avoid hazard sensors. Tile collisions use the real 2D solver. Choose a
coherent original visual direction with full creative freedom. Make a finished
small playable scene, not a programmer grid, generic UI, mockup or concept image.
The numerical room is only a correctness fixture; you can redesign its layout.
Keep navigation readable and the play loop short enough to verify reliably.

Use actual SpriteRenderer, stable atlas frame IDs, Tilemap and 2D physics through
the public importer and command transactions. Generating original source textures
or descriptors before importing them is allowed. All project edits go through
incant_cmd via ordinary RPC/CLI with stable ULIDs, provenance, atomic Undo/Redo
and journal reopen. Never directly synthesize or edit project JSON. Do not change
engine, ABI, save, solver or compiler implementation; give Astra a minimal repro
for blockers. Do not switch to legacy document-backed Play.

## Logic contract and boundaries

Read `samples/planar/README.md`, `gameplay.ts` and `verify.py`. Seven actual native
keyboard/gamepad/win/loss/wall/locked-exit/diagonal/repeat scenarios pass. Source
review is accepted; Astra is separately running the ordinary source-free export.
Do not edit `gameplay.ts`, `verify.py`, `export_verify.py`, `tsconfig.json` or CI.
Create a separate `scene.py`, art sources/generators/assets, and a presentation
guide under `samples/planar`. Use focused files and modest source/output sizes.

The controller props are player, exit, 1–16 pickups, optional 0–16 hazards,
optional camera and speed (default 3 m/s; finite (0,12]). All roles are distinct
canonical IDs. The player has a dynamic zero-gravity RigidBody2D, Collider2D and
Velocity. Fixed sensor Collider2D objects own pickups, hazards and exit; solid
Tilemap layers own wall collision. Physics entities are root, unit-scale XY
objects. Pickups must have no children because the controller despawns them.
One source controller handles gameplay; no per-entity scripts. Positive input Y
means up; choose keyboard and left-stick bindings accordingly. Optional camera
follows XY preserving its initial offset; source-reviewed but not yet exercised
in the numerical verifier. Prefer a single readable camera for this bounded map
unless a moving camera clearly improves the scene. Do not add props unsupported
by the source. No restart action exists yet: document the small hook you need.

You may design restrained authored game UI if helpful. The current controller
has no HUD bindings: give Astra stable widget/string IDs and exact intended
state transitions for integration. Do not pretend static labels are live state.
Headless native-play currently cannot host UI; ordinary exported player supports
it with a declared camera and --ui-viewport. Native headless UI routing is being
implemented separately. Keep gameplay verification possible without UI. Never
replace engine text with baked text textures. The director dislikes cramped
layouts, generic cards, decorative borders/glows and repeated explanatory copy.

## Builds, evidence and desktop coordination

The exclusive warm target `/Users/andreyyurlov/.codex/incant-core-presentation-target`
is now released by Astra's matching-headless build and assigned to this packet.
It contains the main160 engine headless build. Astra's ongoing export uses its own
private build directory. Acquire a fresh packet-local lease with owner, start,
12-hour expiry and release marker. Use `./tools/cargo`, locked Rust 1.99.0,
CI profile and at most two jobs. Every build/run wrapper must check that lease.
Retain commands, exit statuses, source/output hashes and every owned handle.
Do not reuse targets or lease files belonging to 0060 or other agents.

0060 is independently designing the 3D Core Sample. Work on your own art,
builder and offscreen actual renderer review in parallel. To avoid fighting over
the desktop, do not open a native window or capture the screen until
`/Users/andreyyurlov/.codex/incant-evidence/0060/released` exists; that file is
read-only to you. Once it is released, require at least 20 seconds of real user
idle, a fresh capture lease, foreground ownership and window-only capture. Yield
immediately on input, restore focus and reap every window/capture process. Do not
capture the whole desktop. If 0060 is still active when offscreen work is ready,
return a precise pending native-review item rather than interrupt the user.

Use actual native renderer outputs, not another renderer pretending to be Incant.
Measure the bounded atlas/mesh/tile counts that you can observe. Local Mac CI
timings are not reference-phone performance. Keep artwork friendly to mobile
budgets and provide reproducible source generation or checked original sources.

## Return

Verify imports, ordinary command authoring, Undo/Redo/reopen, playability and no
project mutation from Play. Review actual rendered pixels and correct obvious
layout/readability issues. Return `handoffs/0061-planar-sample-presentation/result.md`
with the verified model/effort, implementation contract, exact commands, output
hashes, screenshots and explicit remaining gates. Commit with `Built-by: claude`.
Reap every owned process and release target/capture leases. No feature, device or
phase gate is approved by this handoff.
