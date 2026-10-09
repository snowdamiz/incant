# Result: handoff 0004, wisp mascot logo

## Status

The work is complete and ready for Astra's review. The phase gate is not approved here.

- **Model.** Claude Opus 5.5, model ID `claude-opus-5-5`, running in Claude Code.
- **Routing caveat.** The session runs on the director's Claude subscription. I cannot independently verify that it reached me through ACP.
- **Scope.** All edits stay inside the allowed paths and this handoff packet. No provider or login UI, contracts, Rust code, or other handoffs were touched.
- **Native integration.** None was done. Astra owns it, and the items to integrate are listed below.

## What changed

The old spark-and-ring icon on a dark tile is gone. Every asset now shows the purple wisp
from the reference alone. There is no wordmark, no tile, and the background is
transparent. The body color is `#482AC4`, the median of solid body pixels in the
reference. The eyes are filled white, as in the reference, so the face still reads on
dark surfaces. Eye cut-outs would turn dark there.

The mascot was traced from the reference pixels, not redrawn. The local tracer applies
sub-pixel marching squares to the purple channel, then fits cubic Béziers with a 0.6 px
tolerance. It kept exactly two sharp corners, the top and bottom tail tips. No external
services or new dependencies were used.

| Fidelity check against the reference | Value |
| --- | --- |
| Binary IoU of body and eyes | 0.992 |
| Pixels differing by more than 50% coverage | 572, all single-pixel edge fringe |

## Changed paths

- `editor/ui/public/icon.svg`: the new 512 icon, with the mascot 488 px tall. It is 1.6 KB.
- `editor/ui/public/icon.png`: regenerated 512 raster with alpha. It shrank from 46 KB to 7 KB.
- `editor/ui/brand/icon-macos.svg`: the mascot fills the 824 px macOS body height inside the 100 px margin. The system-style shadow stays inside that margin.
- `editor/ui/brand/Incant.icns`: regenerated with all ten iconset members.
- `editor/ui/brand/wisp-mascot.svg`: new master trace in reference pixel units.
- `editor/ui/brand/README.md`: new file with provenance and regeneration commands.
- `handoffs/0004-wisp-mascot/`: tools, screenshots and this result. The packet's `brief.md` and `reference.png` are unchanged.
- The titlebar component and its CSS were not changed. The 18×18 `./icon.svg` reference already picks up the new art.

## Commands and results

| Command | Result |
| --- | --- |
| `npm run icon --workspace editor/ui` | Wrote `public/icon.png` |
| `npm run icon:macos --workspace editor/ui` | Wrote `brand/Incant.icns` |
| `npm run test --workspace editor/ui` | 6 files and 160 tests passed |
| `npm run build --workspace editor/ui` | **Failed** with a pre-existing type error, shown below |
| `npx vite build` in `editor/ui` | Succeeded, and was used for the fixture captures |

The build failure reproduces on a clean HEAD at `a1b4f5a`, before any of my changes.
It comes from the in-progress provider contract, which is outside my scope:

```
src/bridge/resolve.ts(45,7): error TS2739: ... missing the following properties ...
"provider.cancel", "provider.disconnect", "provider.switch"
```

These evidence tools run from the worktree root:

```
node handoffs/0004-wisp-mascot/tools/trace-mascot.mjs     # trace master SVG
node handoffs/0004-wisp-mascot/tools/compare-trace.mjs    # fidelity overlay and diff
node handoffs/0004-wisp-mascot/tools/compose-icons.mjs    # bake public and macOS SVGs
node handoffs/0004-wisp-mascot/tools/contact-sheet.mjs    # contact sheet
node handoffs/0004-wisp-mascot/tools/capture-titlebar.mjs <before|after>
```

## Screenshots

All captures are from the **browser fixture** in headless Chrome. None of them is proof of the native window.

- `screenshots/before/`: titlebar identity and logo at 1× and 2×, the full fixture window, and the old 512 icon.
- `screenshots/after/`: the same titlebar captures with the new logo.
- `screenshots/after/contact-sheet.png`: the SVG at 16, 18, 32, 128 and 512 px on the app background, the panel color and white. It also shows the PNG on dark, light and checkerboard, and every icns member.
- `screenshots/after/small-sizes-8x.png`: real 1× rasters at 16, 18 and 32 px, enlarged 8× with nearest-neighbour scaling.
- `screenshots/trace/overlay.png` and `diff.png`: the trace drawn over the reference, and the pixel mismatch.

## Visual findings

- **Titlebar.** The logo box measures 12, 11, 18×18 both before and after, so the layout is unchanged. At 2× the silhouette, both tail curls and both eyes are clear. At 1× the mascot is small but still recognizable, and both eyes resolve as separate white pixels.
- **Small sizes.** At 16 and 18 px the eyes show as two distinct one-to-two-pixel highlights. At 32 px they are clean ellipses. The thin top tail tip fades to partial coverage at 16 px, which is expected from faithful geometry.
- **Dark surfaces.** The faithful purple against the titlebar color `#0b0c0f` has about 2.2:1 contrast. The silhouette reads, and the white eyes carry the face. I kept the exact color rather than lightening it, because the brief asks for faithful color.
- **Light surfaces.** Contrast is strong.
- **Raster quality.** There is no halo, cropping or distortion in the PNG or the icns members. Transparent margins are clean, and the macOS shadow stays inside the canvas.
- **Network.** There are no external requests, fonts or animation, and both captures logged zero external requests.

## For Astra: other brand placements and limitations

- **Native host icon.** `editor/app/icons/icon.png` is a byte-identical copy of the old `public/icon.png`. Tauri uses it by default. It is outside my allowed paths, so it still shows the old icon. Copy the new `editor/ui/public/icon.png` over it.
- **Packaged app.** `Incant.icns` needs to be wired into the app bundle, as in handoff 0002 request R2. I did not test it in a running app or in Finder or the Dock.
- **macOS 26 icon platter.** This host runs macOS 26. That release may draw icons that are not squircles inside a grey system platter. Following the brief, the mascot ships without a tile. Whether to adopt an Icon Composer asset or a tile is a decision for the director. Only a native check can show the actual Dock appearance.
- **Optional small-size variant.** A pixel-hinted 16 px variant with slightly enlarged eyes could sharpen favicon rendering. I did not make one, because the brief asks for faithful geometry and a single SVG.

## Open questions

1. On macOS 26, if the system wraps the tile-free mascot in a platter, should the macOS icon get a purple or white tile like the reference's app-tile example?
2. Should the faithful `#482AC4` stay on dark UI surfaces, or may a lighter brand tint be approved for dark contexts only?
