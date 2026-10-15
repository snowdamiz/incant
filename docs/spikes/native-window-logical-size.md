# Native player initial window units

The Arabic shipping replay exposed a crowded 320×240 logical viewport on this Mac's scale-two display. The native player requested 640×480 physical pixels and then correctly divided by display scale for layout. Five actual partial images and the eligibility log retain that observation; the separate complete numeric replay is not an appearance pass.

The initial window request now uses 640×480 logical units. Existing 16×16 minimum and 1920×1080 maximum physical sizes, actual-surface admission, pointer conversion, focus rules and frame pacing remain unchanged. On this display the new request corresponds to 1280×960 physical pixels, within the cap. The OS may clamp the window on another display; rendering and layout continue to use the actual returned extent and scale.

Source review found no additional runtime behavior change. Rust 1.99 arm64 locked/offline qualification passes 97 ordinary native-window/profile player tests, strict all-target Clippy and workspace formatting. The copied test executable is `c38e8650f79cf008b9df2ddd28cd9bc22fd5d586f8d9f7ac2f1e946ac0608e88`. Commands and raw logs are retained under `~/.codex/incant-evidence/native-window-logical-size-001/`; source commit `a6c6d9ee` contains the two-line change. Actual fresh shipping-window verification remains pending.

This raises raster area fourfold on a scale-two display. No frame-time comparison with the old default is claimed: performance measurements must identify and match actual physical extents. The physical cap remains in force at higher display scales. This change does not resolve Arabic health-pip direction, narrow pseudo-locale layout, reconstruction quality or any phase gate.
