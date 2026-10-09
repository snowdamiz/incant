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
python3 tools/editor-dev.py
```

If the fixture already exists, reuse it. The initialization command deliberately
refuses overwrites. `custom-protocol` embeds local frontend assets so no dev server
or external network is required. Leave source/project edits on the command bus.
On macOS, open the resulting `artifacts/Incant.app`. A proper app bundle is needed
for reliable native app discovery/capture; a bare terminal binary was not listed
by the automation surface. The bundle opens a disposable default project unless
a project path is supplied as an application argument.

## Verify

- Capture the actual Incant native window at approximately 1280×800 and 1920×1080.
- Confirm real rendered geometry shows through the viewport while chrome is opaque.
- Check hierarchy/inspector selection, console and undo history remain usable.
- Check bounds when panels/window resize; record DPI and platform.
- Review the redesigned custom titlebar and its macOS native traffic-light inset.
  Verify titlebar dragging, double-click zoom and focus/fullscreen state. The host
  implements `window.*` requests and publishes `WindowChrome` through the bridge.
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

## Integration notes for this review

The director says the UI must look very good, clean, modern and easy to use, with a
custom titlebar. Keep that higher visual standard in this native review. The first
redesign is integrated; do not revert to the rejected initial layout. The director
explicitly authorized open permissions for Claude's work, and this session is
launched with `--permission-mode bypassPermissions`. Scoped builds/tests are
allowed; prior interrupted session denials do not prohibit this packet's commands.

Astra has implemented your requested window.* contract, 78 CSS-pixel macOS inset,
14/14 traffic-light position, canvas #0b0c0f, and GPU masking for cornerRadii. The
surface stays below the webview so modal UI can cover it. All shared types are now
in editor/bridge/contract.ts; the UI module re-exports them. Both UI and bridge tests
run with `npm test --workspace editor/ui` (158 passed on integration). Actual
engine tests have been run by Astra (29 passed); do not call those unrun based on
the first handoff's earlier interruption.

Review the native app, not only fixtures. Fix visual UI issues in this worktree and
report native Rust correctness/integration requests to Astra. Native double-click
currently toggles zoom; respecting the macOS alternate double-click preference is
still an integration detail to assess. The original titlebar icon has been replaced
with your public/icon.png in the Tauri host; include a macOS app icon if required.

One functional UI detail observed through accessibility: the skip link reads
“Skip to hierarchy” while its href remains #viewport-title; its click handler does
focus the correct region. Make the href agree with the visible label.

Only capture Incant windows from this packet. Do not close or alter any other
running Incant instance the director may be inspecting. If another Incant process
exists, distinguish it from your new worktree build by executable path before
capturing or manipulating it. Record any capture/OS-permission limitation exactly.

## New director feedback — continued visual refinement

The director says: “tell claude to keep improving UI and fixing little issues while
you continue working on logic. For example traffic lights position as one example”.
Their attached native screenshot is at
`handoffs/0002-native-viewport/reference-director-2026-10-08.png`. Read that image
now and explicitly acknowledge this updated instruction before continuing.

Keep iterating on visual quality and usability in the real native app. Address the
traffic-light position/alignment, and inspect other small spacing, alignment,
clipping, focus, truncation and control-state issues across the entire UI. Do not
stop at documenting issues that you can fix within this visual task. Astra is
continuing logic, native integration and platform work in parallel. If a polish
fix needs Rust/native support, write exact requested values/behavior to
`handoffs/0002-native-viewport/native-requests.md` as soon as you identify it;
Astra will read and implement it while you continue UI work. You may keep revising
that file. Keep an actual before/after evidence trail and report remaining issues
candidly. No phase gate approval is implied.

The main-checkout app in the director's screenshot was launched before the
latest committed 14/14 traffic-light inset change; compare the current native
worktree build as well and choose the final placement from actual pixels. The
existing director instance must remain untouched. Resume the native review already
underway in this session, using your existing test instance and captured evidence.
