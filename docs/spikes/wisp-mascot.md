# Wisp mascot integration

Date: 2026-10-08. Scope: application branding only; no phase gate approval.

The director-supplied purple wisp replaces the previous spark-and-ring logo. The
wordmark and presentation background are excluded. Claude Opus 5.5 performed the
trace, icon composition and rendered review through
`python3 tools/handoff/main.py run 0004-wisp-mascot --permission-mode acceptEdits`.
The handoff ran to completion and was integrated with its `Built-by: claude`
provenance. See [the result](../../handoffs/0004-wisp-mascot/result.md) and
[brand regeneration instructions](../../editor/ui/brand/README.md).

## Consumers

- Titlebar and browser favicon: `editor/ui/public/icon.svg`.
- Web raster and native host: `editor/ui/public/icon.png` and
  `editor/app/icons/icon.png`, now synchronized by the icon rendering script.
- macOS: `editor/ui/brand/Incant.icns`, referenced explicitly by Tauri and copied
  into the local app bundle with `CFBundleIconFile` set in Info.plist.

The production SVG contains only local vector artwork. The original reference and
review tools remain in the handoff directory, outside the shipped UI assets.

## Verification

- Claude reviewed browser fixture captures at 1×/2× and icon sizes 16, 18, 32,
  128 and 512 px on light and dark backgrounds. The source trace has a measured
  binary overlap of 0.992. The titlebar's 18×18 box and surrounding layout are
  unchanged. Browser captures are not native Dock evidence.
- The integrated strict UI production build passes; all 180 UI/bridge tests pass.
  The older isolated handoff's provider-contract type error is absent in the
  integrated checkout.
- Rust formatting and workspace clippy pass. All 41 Rust tests pass on the final
  rerun. A preceding run hit a connection-reset failure in the concurrent
  authentication callback test; no branding changes were made to that test.
- Icon regeneration produces a 512×512 PNG with alpha, byte-identical in the web
  and native paths. The built UI's SVG matches its source.
- The release editor build with `custom-protocol` succeeds. Packaging with
  `python3 tools/editor-dev.py --release --no-build` succeeds. The packaged
  executable matches the release binary, and the declared Resources icon matches
  the source ICNS byte for byte.
- Four Python tool tests, convention generation, bridge generation, schema and
  SDK checks, the offline twenty-case corpus check, and collaboration spike pass.
  The corpus check does not count as live inference or phase gate evidence.

The source/packaged ICNS SHA-256 is
`7caf36224bb365064ad4975d37dedc1a3a9925e3beea916c0c3aa7d17ac5d185`.

## Limits

The running application must be relaunched to load the rebuilt embedded UI and
icon. Existing app instances were not closed. The updated native Dock rendering
and Windows runtime were not visually reviewed in this task. The supplied purple
and freestanding silhouette are preserved; no alternate tile or tint was added.
Concurrent authentication work remains separate from the branding change.
