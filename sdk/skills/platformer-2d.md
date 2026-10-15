# 2D platformer controller

## Workflow

1. Import a real sprite atlas with declared frame IDs. Author a solid tilemap, the player's Sprite/flipbook, orthographic camera and native 2D collision components using their schemas. Keep collision tiles and visual tiles aligned.
2. Bind horizontal move and jump actions. Apply gravity and horizontal speed in fixed ticks, normalize analog input, and use the supported native 2D physics path. Grounded state must come from actual contacts or an available query, not a hard-coded floor Y.
3. Drive idle/run/jump animation from accepted movement and ground state. Queue supported flipbook changes through context.commands; do not rebuild the document or reload the atlas per tick. Follow the player with a stable orthographic camera and choose an explicit pixel-perfect policy.
4. If adding coyote time or jump buffering, use tick counters/timers with declared limits and reset behavior. Keep authored defaults distinct from mutable runtime state.

## Verification

Test standing, running into solid tiles, jump/landing, walking off an edge, repeated jump input, animation progression and camera follow. Include a nonzero tile origin and a low ceiling. Verify the saved authoring state is unchanged by play. Inspect pixel alignment separately from numerical collision tests.
