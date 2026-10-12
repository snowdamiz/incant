# 0060 — Core Sample level, art and game UI

Required model: Claude Opus 5.5 over ACP. Required thinking: explicitly Max,
verified before prompting. Read CLAUDE.md and PLAN.md Phase 1. You own the
visual design, modeling/art, layout, look development and rendered review.
Astra owns game logic, runtime correctness and integration. Do not use another
model or effort setting. Do not edit personal memory files.

## Outcome

Build a genuinely finished-looking, small third-person Core Sample level using
Incant's actual native renderer, authored game UI and existing asset pipeline.
PLAN.md promises a character on terrain, enemies, pickups, UI and music. The
gameplay foundation in samples/core/gameplay.ts already moves a character,
follows a navmesh with chasing enemies, collects pickups, applies damage and
locks the exit until all pickups are collected. Astra is separately verifying
source-free shipping and will connect changing gameplay state to your HUD/audio.

Create the actual level and original reusable assets, not just a mockup or a
proposal. Exercise it in the real native game window. Choose a coherent art
direction with full creative control: appealing terrain and landmarks, readable
characters/enemies/pickups/exit, good camera composition, restrained polished HUD,
and a comprehensible game flow. The director strongly dislikes generic/vibe-coded
cards, decorative stripes/glows, cramped layouts, repeated explanatory text and
oversized chrome. These concerns apply to the whole composition, not just one
widget. Avoid a programmer-box demo. Keep scope to one small polished playable
level, not a large game or new renderer features.

## Integration boundaries

The accepted numerical test room and controller are in samples/core. The packet
contains evidence/development-gameplay.json for the seven passing development
scenarios. That room's exact numerical fixtures are not a visual constraint.
Do not modify gameplay.ts, verify.py or tsconfig.json: Astra continues those files.
Author a separate samples/core/scene.py (or similarly focused builder), art/
assets, localized UI documents and a presentation README. The builder must create
and modify project content exclusively through public incant_cmd transactions
via the existing RPC/CLI, with stable IDs, provenance and Undo/Redo support.
Generating original source art files before importing them is allowed; never
write a project JSON directly or invent a second mutation path.

Use the controller's actual role properties: player, navigation, exit, enemies,
pickups, optional camera, speed, enemySpeed and jumpSpeed. Actors and camera are
unparented; visual child geometry may be attached. The player and enemies need
Velocity and suitable kinematic capsules. The navigation mesh must be built from
the real terrain/collider sources and must not need off-mesh traversal. Choose
level distances and speeds that permit play. Do not alter runtime world/save/ABI
formats or repair unrelated engine semantics; record a precise reproduction for
Astra if something blocks the actual level. Script movement is currently
world-space WASD/left stick and jump Space/south. Preserve valid body/terrain
geometry, avoid nonuniform capsule scaling and keep collision silhouettes legible.

Use real authored UI and localized strings. Design HUD placeholders with stable
widget IDs for health, collected count, objective, win and loss so Astra can bind
actual state; do not claim dynamic bindings work until exercised. Include English,
pseudo-localized, one RTL and one CJK presentation using the actual shaping and
fallback stack; avoid baking text into textures. Document exact IDs and the small
logic hook needed. Original short music/source audio is welcome if tools permit,
but don't fabricate playback evidence or hold visual progress for audio.

## Paths, commands and limits

Use ./tools/cargo with locked Rust 1.99.0, ci profile and at most two jobs. The
exclusive warm target /Users/andreyyurlov/.codex/incant-native-target is released
by 0059 and assigned to this packet. Acquire a new packet-local lease with owner,
start, expiry and release marker; don't reuse 0059's released runner. Other agents
own incant-animation-target and incant-integration-target; never use those.
No pushes, PRs, merges, credentials, account changes or remote workflows.

Read samples/core/README.md and existing tools/probes examples for command-bus
authoring and native exports. Headless native-play and the ordinary native-window
player are the verification route; do not use legacy document-backed Play. All
artifact sources and exact commands must be retained outside temporary build
directories. Keep assets reasonably bounded for mobile tiers: modest mesh/material
counts, limited texture sizes and a small level. Report actual asset/draw/memory
counts you can measure; Mac ci-profile timings are not reference-phone evidence.

Native interaction only in short bursts after 20 seconds of real user idle.
Check foreground ownership, yield immediately on input and restore prior focus.
Use window-only captures, never a whole-desktop screenshot; keep a fresh capture
lease and release it. Do not leave a play session or capture process running.
Offscreen actual renderer output is useful for iterative work but final acceptance
needs a scoped native-window review, including resize and representative states.

## Acceptance and return

Return implemented builder/assets/UI with concise reproducible commands and a
finished visual result reviewed at native pixels. Verify import through the real
pipeline, project reopen, and no authoring mutation by Play. Check relevant
source/asset validation and focused render tests without rewriting goldens to
conceal defects. Commit with Built-by: claude. Write result.md with model/effort,
source and output hashes, exact screenshots, presentation decisions, actual
adapter, checks, limitations and a small integration contract for Astra's state
bindings. Stop/reap every owned process and release the target/capture leases.
This is one Core Sample presentation slice, not a claim that the full sample,
engine, game UI or device/phase exit gate is complete.
