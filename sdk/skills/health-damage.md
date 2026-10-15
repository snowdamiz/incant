# Health and damage

## Workflow

1. Define maximum health, damage sources, faction rules and respawn behavior as explicit authored configuration. Use bounded versioned runtime state for current health, cooldowns and alive/dead transitions rather than inventing an unsupported Health component.
2. Apply each accepted hit once with a stable event/source identity. Validate finite nonnegative damage, clamp health, and gate invulnerability by fixed ticks. Separate contact detection from damage application so repeated overlap does not create accidental per-frame damage.
3. Queue supported death/despawn/reset effects through runtime commands. Update localized health/damage/death UI and audio without replacing authored sources. Haptics require an advertised target; unavailable hardware must remain an explicit no-feedback case.
4. Define checkpoint reset of health, timers, animations, input and pending damage. Keep one authoritative death transition.

## Verification

Test zero/maximum damage, simultaneous hits, friendly fire policy, sustained overlap, invulnerability expiry, death once and repeated restart. Verify health bounds, no post-death movement/damage, localized HUD and exact reset of related runtime state.
