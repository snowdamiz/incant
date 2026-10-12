# Draw connected native game UI through the canonical renderer

Claude Opus 5.5 via ACP with explicitly verified Max thinking owns all rendered
appearance, shaders, look-dev, captures and visual acceptance. Read AGENTS.md,
CLAUDE.md and the connected UI evidence in this checkout. Root has independently
reviewed the CPU UI contracts. Their fresh public export proof remains separately
tracked; this handoff does not approve that or any phase gate.

## Implement

Finish the screen-space GPU consumer for the actual native UI tree. Existing
`incant_ui::UiPaint` carries accepted layout, canonical draw order, logical-pixel
origins, clipping, shaped glyph runs and retained atlas handles. Reuse those exact
inputs; do not introduce another shaper, font lookup, UI document, DOM overlay,
state model or GUI-only mutation route. World text keeps its separate world-space
rendering path. Game UI must composite correctly over the existing game output.

Implement readable labels, buttons and text inputs, with functional visual states
for disabled, focused, hovered/pressed where the runtime exposes them, and caret,
selection and preedit where exact runtime data supports them. Identify a missing
CPU contract explicitly and request a narrow addition; never infer selection or
IME ownership from raw text. Root owns native platform IME/input receipt wiring.
You own GPU buffers/pipelines, draw order, clips/scissors, scaling, colour handling,
and visual states. Necessary renderer-boundary wiring belongs to this scope, but
leave world/input/save/script semantics unchanged. UI save support remains refused.

The director wants a restrained, professional product: clear hierarchy, deliberate
spacing, useful state feedback, no gratuitous glow or decorative borders. You have
creative control over appearance. Do not spend this packet restyling editor panels.
The result is a working game UI consumer, not a screenshot-only mockup.

## Correctness and acceptance

Use `NativeRenderHost` and the ordinary GPU consumer. Advertise SUPPORTS_UI only
when the consumer really draws it. Admission failures must occur before consuming
an accepted frame; preserve old retained GPU resources while a candidate refuses.
Respect the bounded 4MiB UI atlas, glyph/line counts and existing stale-owner checks.
No per-frame font discovery, project IO, shaping of unchanged content, synchronous
readbacks or unbounded GPU allocations. Record resource reuse numerically; do not
claim performance from a debug/ci build or desktop-only timing.

Add meaningful numerical GPU tests for logical/physical scale, nested clips,
draw ordering over a world scene, transparent coverage and retained atlas lifetime.
Verify layout and drawing use the same shaped glyph positions: the recent bidi
fix intentionally allows Word overflow and an indivisible glyph wider than its
container. RTL-base long-token glyphs can begin at negative X. Clip actual glyph
bounds rather than assuming every glyph starts inside its content box.

Author a small mixed fixture through public imports and incant_cmd using packaged
fonts, labels, buttons and text fields. Include a normal desktop size and a narrow
size; Latin, Arabic/RTL and CJK text; wrapped text; focus/activation/value/preedit
states; and ordinary scene rendering behind the UI. Exercise accepted CPU snapshots
and your actual GPU consumer. Use actual native-window review when the production
host path supports it; otherwise report the exact missing platform connection and
retain offscreen pixel/numerical evidence without claiming native input/IME passed.
No provider/account action is needed.

## Source and tooling

Use the exact clean union committed for this packet, not older executables. Record
source/tree, binary/profile, fixture/font and capture identities. The checkout
contains reviewed connected UI, canonical native text drawing and the bidi repair.
The final source-free CPU export proof is independent evidence; do not overwrite it.
Use `./tools/cargo`, `--locked`, bounded `-j2`. Build and test relevant crates and
existing renderer numerical regressions, then inspect pixels yourself. Root reads
source and numerical evidence but does not approve pixels.

## Exclusive resources and cleanup

0055 is terminal and its players/renderer have been reaped. This packet receives
the exclusive native/GPU lease and `/tmp/incant-agent-native-target`. No other lane
may run GPU/native-window review or use that target until you release it. Never
use/delete another lane's target. Create new fixture/output directories and a new
lease marker; do not reopen released 0054/0055 tools or mutate their evidence.
Use brief review bursts, restore the prior foreground app/pointer, and yield to
human input immediately. Log the actual child/player PID; wrappers should exec.

Commit focused source/tests and evidence with `Built-by: claude`. Return
`handoffs/0056-native-game-ui-draw/result.md` with Opus5.5/Max verification, exact
checks, your own visual judgment, image links, and remaining limitations. Stop and
reap every owned player/helper/build before returning, and record release of the
native/GPU lease and target. No phone, mobile keyboard, complete Game UI or phase
gate claim unless independently evidenced for its full scope.
