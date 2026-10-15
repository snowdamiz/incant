# Third-person controller

## Workflow

1. Author a player Transform, Velocity, kinematic RigidBody and capsule Collider using current schemas. Import or reuse a real visual asset as a child, keeping visual scale distinct from the collision body. Bind player/camera ULIDs in one controller Script's props.
2. Author move, look and jump input actions with keyboard and gamepad bindings. Normalize diagonal movement, apply fixed-dt gravity and jump only from a grounded transition. Use context.physics.computeCharacterMotion for the intended displacement; apply its complete collision/sliding result to bulk velocity, rather than discarding contact corrections.
3. Keep yaw, pitch, vertical velocity and grounded state in versioned context.state. Clamp pitch, use camera local -Z consistently, and put the orbiting camera behind the player with an authored eye offset. Add obstruction avoidance only with an available native query and clearly define its clearance.
4. Avoid per-entity controller callbacks for crowds. Use one system and stable role IDs, validate required components, and keep camera ownership separate from player movement.

## Verification

Record walking, diagonal movement, jumping/landing, wall contact, slopes/edges and look changes. Assert movement distance, bounded lateral drift, wall exclusion and exact intended camera/player relation. Test reset and missing roles. Do not assume floating-point capsule contacts keep an unused axis exactly zero. Native camera/input feel and obstruction appearance still need actual review.
