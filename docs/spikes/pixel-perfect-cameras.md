# Pixel-perfect camera foundation

A third authored Camera projection supplies fixed logical resolution and pixels
per metre. Scene geometry, lighting and depth render into bounded logical-size
attachments, then the display pass replicates whole pixels into the physical
viewport with centered letterboxing. Ordinary perspective/orthographic paths
retain their existing target and composition. Output smaller than the logical
resolution fails explicitly, before allocating scene attachments.

Camera-plane motion snaps to the nearest world grid cell, including inherited
roll; forward depth is unchanged. Snapping affects the renderer only: authored
project data and simulated transforms remain continuous. Individual objects,
textures and rotation are not snapped. Logical dimensions each range 16..4096,
with at most 4,194,304 pixels, and density is finite 1..4096 pixels per metre.
This bounds logical color/depth attachments to 48 MiB. Physical output retains
the renderer's existing limits. This is a camera feature, not a sprite/tilemap or
2D-physics implementation.

Local foundation checks pass 320 Rust tests, Clippy and 47 real GPU checks on
Apple M5 Pro. The numerical checks verify exact integer pixel replication,
odd-size letterboxing, subpixel camera stability, one-cell movement, and queued
logical targets surviving output resizing and projection switches. They also
cover the existing perspective/orthographic, lighting, shadow, retained-asset,
editor recovery and headless frame paths.

The strict-TypeScript public probe authors a legacy camera through shared
commands, changes projection and moves its simulated transform by 1/256 metre
per tick. A 25-tick save plus 35-tick restore exactly matches uninterrupted and
repeated 60-tick runs. Project and journal bytes stay unchanged. Both pixel-perfect
and ordinary orthographic modes pass. Generated schemas/SDK, bridge, conventions
and five tool tests pass. This is synthetic behavior evidence, not a device gate.

Claude handoff 0031 owns the Inspector and original real-engine visual review.
The initial UI run passes 385 tests and fails two Camera-schema parity assertions
because the presentation fixture has not yet adopted the third projection.
Native packaging, portable compilation, appearance review and hosted CI remain
open. No phase-gate pass is claimed.
