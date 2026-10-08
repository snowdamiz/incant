# Phase 0: review the integrated native viewport

## Goal and ownership

Review the actual macOS native Tauri/wgpu composition with the Claude-owned UI.
You own rendered-pixel assessment, layout, screenshots, icon and visual fixes.
Astra owns Rust, the command bus and transport correctness. Do not claim Windows
or other platform evidence from this Mac. Do not begin Phase 1.

## Run

Read CLAUDE.md and editor/ui/NATIVE_VIEWPORT.md, plus the first handoff result.
Use `npm ci` and `npm run build --workspace editor/ui`. The native bridge is in
editor/bridge. Start the app with bundled assets, not a fixture browser page:

```
tools/cargo run -p incant_headless -- init artifacts/native.incant.json --name "Native Surface Spike"
tools/cargo run -p incant_editor --features custom-protocol -- artifacts/native.incant.json
```

If the fixture already exists, reuse it. The initialization command deliberately
refuses overwrites. `custom-protocol` embeds local frontend assets so no dev server
or external network is required. Leave source/project edits on the command bus.

## Verify

- Capture the actual Incant native window at approximately 1280×800 and 1920×1080.
- Confirm real rendered geometry shows through the viewport while chrome is opaque.
- Check hierarchy/inspector selection, console and undo history remain usable.
- Check bounds when panels/window resize; record DPI and platform.
- Review actual GPU screenshot output from `incant_headless screenshot` separately.
- Screenshots must contain only this app, never the user's other windows or accounts.
- If native capture or OS permissions are unavailable, report the exact blocker;
  do not fabricate a native screenshot or substitute a browser fixture.

The native renderer is deliberately diagnostic grayscale geometry, not a material
or art pipeline. Report correctness problems to Astra; fix visual/CSS issues only
in editor/ui. Do not modify credentials, publish, merge, or approve phase gates.

## Return

`handoffs/0002-native-viewport/result.md` with exact model, commands/results,
actual screenshot paths, platform/size/DPI, findings and limitations. Clearly
separate browser transparency evidence from native composition evidence. Commit
visual changes and the result with `Built-by: claude` for Astra to integrate.
