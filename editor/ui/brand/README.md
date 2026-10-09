# Incant brand assets

The Incant logo is the purple wisp mascot alone: no wordmark, no tile, transparent
background. Body `#482AC4`, eyes `#FFFFFF`.

## Provenance

- Source: director-supplied reference artwork, kept unchanged at
  `handoffs/0004-wisp-mascot/reference.png`. It is not shipped in any bundle.
- Only the large purple wisp was traced. The wordmark and example logos were excluded.
- The trace was made locally, without image-generation services, by
  `handoffs/0004-wisp-mascot/tools/trace-mascot.mjs`. It uses sub-pixel marching
  squares on the purple channel and a cubic Bézier fit with a 0.6 px tolerance.
  The two tail tips are kept as sharp corners.
- Fidelity against the reference has a binary IoU of 0.992. Differences are limited
  to anti-aliased edge pixels. The overlay and diff images are in
  `handoffs/0004-wisp-mascot/screenshots/trace/`.

## Files

| File | Role |
| --- | --- |
| `brand/wisp-mascot.svg` | Master trace in reference pixel units. |
| `public/icon.svg` | 512 canvas, mascot 488 px tall. Used by the titlebar and favicon. |
| `public/icon.png` | 512 raster of `public/icon.svg`. |
| `brand/icon-macos.svg` | 1024 canvas. The mascot fills the 824 px macOS body height inside the 100 px margin, with a soft drop shadow. |
| `brand/Incant.icns` | macOS iconset built from `brand/icon-macos.svg`. |

The SVGs are now the source of truth. Edit them directly, or re-trace and re-compose.

## Regenerate

Run these from the repository root. They require Chrome, and the icns step requires macOS `iconutil`.

```
node handoffs/0004-wisp-mascot/tools/trace-mascot.mjs     # optional: re-trace master
node handoffs/0004-wisp-mascot/tools/compose-icons.mjs    # optional: rebuild both SVGs
npm run icon --workspace editor/ui                         # public/icon.png
npm run icon:macos --workspace editor/ui                   # brand/Incant.icns
```

`editor/app/icons/icon.png` is a copy of `public/icon.png` used by the native host.
Copy it again after you regenerate the icons.
