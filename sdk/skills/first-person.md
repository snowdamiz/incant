# First-person controller

## Workflow

1. Start from a kinematic capsule, Velocity, Transform and explicit camera role. Place the camera at the authored eye height; keep the player's own visual geometry from obscuring that view through supported authoring choices.
2. Map movement and look actions. Use fixed simulation dt for stick/keyboard angular velocity; distinguish relative mouse deltas from rates so they are not multiplied incorrectly. Request native pointer capture only through a supported host input capability and keep Escape/focus-loss release behavior. Never treat synthetic motion as real captured-mouse evidence.
3. Keep yaw/pitch and vertical motion in versioned runtime state. Clamp pitch, normalize movement, rotate movement by yaw, use native character motion for contacts, and update the camera quaternion with the engine's local -Z convention.
4. Keep look ownership, UI focus and gameplay input mutually consistent. When a menu or text input owns the event, do not also rotate the camera.

## Verification

Test grounded jump, wall/ceiling contact, movement at different yaw angles, pitch limits and unchanged eye offset. Exercise focus loss, menu entry and restart. Verify actual relative-pointer capture with real native input when available; otherwise state that specific check is pending.
