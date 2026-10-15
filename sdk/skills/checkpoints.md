# Checkpoints

## Workflow

1. Author checkpoint trigger volumes and stable checkpoint IDs in the scene. Define activation order, repeat policy, respawn transform and reset scope. Choose whether this is only a gameplay respawn marker or a full native session save; they are different contracts.
2. Observe actual native trigger/interaction events and commit activation once. Keep the chosen checkpoint and progress in versioned context.state, with localized confirmation and optional supported audio feedback.
3. For simple respawn, reset all explicitly owned state, velocities, transient inputs and timers consistently through supported runtime operations. For exact restore, require the host's versioned whole-session checkpoint capability; do not serialize a JSON world or claim a camera teleport restores physics/audio.
4. Handle checkpoint removal, invalid IDs and incompatible saves explicitly. Keep authored document/source history separate from disposable gameplay state.

## Verification

Test repeated entry, competing checkpoints, death immediately after activation, unavailable/removed marker, restart and load. Assert respawn location, health/timers/velocity policy and preserved progression. For exact restore, compare the complete supported continuation against its baseline; partial transform equality is insufficient.
