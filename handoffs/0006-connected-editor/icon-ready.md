# Windows ICO ready

- Commit: `a304a11` ("Add Windows ICO for the approved wisp app icon"), branch `handoff/0006-connected-editor`.
- Path: `editor/app/icons/icon.ico` (123,204 bytes). That commit contains only this file.
- Source: `editor/app/icons/icon.png`, the approved 512 px wisp raster. Artwork unchanged.
- Entries: 32-bit BMP (DIB with alpha and AND mask) at 16, 20, 24, 32, 40, 48, 64, 128 px; PNG at 256 px.
- Checks: macOS `sips` decodes the file as `com.microsoft.ico` with alpha. Single-entry 16 and 32 px
  extracts were decoded back and visually confirmed upright, transparent and unchanged.
- Not done here (Astra): adding `icons/icon.ico` to `bundle.icon` in `editor/app/tauri.conf.json`
  and re-running Windows CI. No Windows host was available to Claude, so `rc.exe` was not run locally.
