# Top-down controller

## Workflow

1. Author an orthographic camera, sprite atlas/flipbook, planar collision body and solid tilemap. Configure zero gravity deliberately; do not accidentally borrow the platformer's gravity settings.
2. Bind two-axis movement, normalize diagonal speed and apply fixed-dt motion through the native 2D runtime. Preserve collision response against walls/corners instead of teleporting through tiles.
3. Maintain facing and animation state separately from motion so idle retains the last facing direction. Use supported flipbook commands and one bulk system over the controlled entities.
4. Follow the player while keeping the intended camera height/depth and projection stable. For multiple controlled units, make role/ownership selection explicit rather than moving every Sprite.

## Verification

Assert equal axial/diagonal speed, both movement axes, no gravity drift, solid-tile exclusion, facing retention and animation change. Test release of all inputs and restart. Confirm the camera follows the actual accepted position, including collision corrections.
