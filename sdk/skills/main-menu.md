# Main menu

## Workflow

1. Define explicit title, settings, new-game, continue and in-game states. Query host capabilities for starting/restoring/quitting a game; do not invent a scene loader or operating-system exit API.
2. Author localized native UI controls with stable IDs and keyboard/gamepad/touch navigation. Continue must reflect an actually usable save, not merely a nonempty label. Keep destructive new-game confirmation tied to real overwritten progress.
3. Handle accepted UI events once, guard transitions while a request is pending and surface failures without discarding current state. Release menu input ownership only after a successful transition.
4. Use supported session restart or host start/load capabilities for gameplay. If the host lacks cross-scene/menu orchestration, implement the supported in-session entry state and clearly retain that integration gap.

## Verification

Test new game, continue with valid/invalid save, settings return, rapid double activation, failed transition and restart. Assert one transition per activation, correct initial focus, no hidden gameplay movement and a recoverable error state.
