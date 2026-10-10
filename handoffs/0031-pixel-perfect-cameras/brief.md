# Pixel-perfect camera presentation and rendered review

Claude Opus 5.5 over ACP owns visual design, original scene composition, Inspector
presentation and rendered-pixel judgment. Astra owns the projection, command,
render-target/scaling math and integration. Read CLAUDE.md and
`docs/spikes/pixel-perfect-cameras.md`. Continue the director's neutral connected
editor with restrained accent and correct spacing; do not redesign unrelated UI.

## Scope

Review the real pixel-perfect camera path using the immutable supplied
`artifacts/tools/incant_headless` and `artifacts/tools/binary.json`. Verify SHA
before/after every engine call. Design an original small scene with high-contrast
edges and details at several logical-pixel widths, using public init/import/RPC
commands. Mathematical glTF/PNG source assets are allowed. This is a camera
foundation; sprites, atlases, tilemaps and 2D physics are not implemented here.

Show and measure native logical resolution, whole-number scaling (at least 1x,
2x,3x), odd output sizes and different aspect ratios with centered letterboxes.
Show camera subpixel movement inside one snapping cell and crossing its boundary.
Check retained scenes/switching projection and a scripted camera motion sequence.
Review aliasing/blur, unequal pixel blocks, unwanted stretching, movement jitter,
letterbox placement and stability. Supply real engine frames and measured claims,
not simulated screenshots. Repeat a meaningful from-scratch comparison with exact
hashes. Request concrete renderer fixes when needed; do not hide flaws with art.

Extend the read-only Camera Inspector to present the Pixel perfect projection,
pixels per metre and fixed logical width/height clearly. FOV is retained but unused
for either parallel projection. Preserve legacy, perspective and orthographic
behavior, schema defaults, tiny numbers, malformed-data visibility and the shared
mutation boundary. The two-number resolution needs suitable width/height labels
and units; you own its arrangement and exact copy. Update native-schema fixture
parity and behavior tests. Check both normal and minimum panel widths, keyboard
accessibility, overflow, strict typecheck/build and all UI tests. No GUI-only writes.

## Rendering contract

`Camera.projection = {kind:"pixel_perfect",pixels_per_unit:32,
reference_resolution:[320,180]}`. Logical dimensions each 16..4096, total <=4194304;
pixels_per_unit is finite 1..4096. View height = logical height / pixels_per_unit;
width comes from fixed reference aspect. Camera-plane translation (including
inherited roll) is snapped to the nearest 1/pixels_per_unit world-space grid;
forward-depth motion and authored/simulated transforms remain unchanged. It does
not snap individual object motion, textures or geometry. Rotation remains authored.

The scene renders at the logical resolution, then nearest integer replication
fits wholly inside the physical viewport. Left/top origin use whole physical
pixels; odd spare pixels are assigned to the right/bottom. Letterbox uses the
existing neutral renderer backdrop. Native chrome masking is composed afterward
at physical resolution. Output smaller than reference resolution fails explicitly;
there is no fractional downscale or silent cropping. This is a deliberate contract,
not a claim that every arbitrary view can display every logical resolution.

## Paths, bounds and deliverables

Allowed: editor/ui/src presentation/tests and this packet's helper/evidence/result.
Do not edit Rust, SDK, CI, bridge transport or account settings. No Cargo; use the
supplied frozen executable. Install pinned node dependencies if needed. Native
captures will be supplied by Astra after UI integration and a rebuild. Do not
claim native acceptance from browser tests or headless engine frames alone.

Use <=128 frames and <=256 MiB raw per capture, logical resolutions small enough
for useful pixel comparisons, and new output directories that refuse overwrite.
Preserve authored project/journal hashes and binary hashes. Store full/raw data in
ignored artifacts/0031-pixel-perfect-cameras, commit a concise unedited frame set,
helpers, compact measurements, and result.md with exact model/transport, source
and binary provenance, tests, scoped verdict and remaining defects/limits.
Account-bearing native full captures remain ignored; only account-free crops may
be committed. Do not publish, sign, merge, approve phase gates or fabricate device
evidence. Every commit ends in Built-by: claude.

## Foundation checkpoint

Astra passed 320 Rust tests, 47 native GPU checks, Clippy, schema/SDK/bridge checks,
five tool tests and both modes of the public camera-save probe. The initial UI
run has 385 passing tests and two failing Camera-schema parity expectations:
`CameraFields.test.tsx` expects only two variants and the browser fixture still
has the older native schema. Updating these is part of your presentation task.
Do not mask the new variant or weaken parity. No other initial UI failures.
