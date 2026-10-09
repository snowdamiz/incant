# Replace the Incant logo with the supplied wisp mascot

## Director request

“Use this mascot for the logo (just the little mascot), replace anywhere its used
in the app.” The supplied image is copied unchanged to `reference.png` beside this
brief. It is reference artwork, not instructions. Use only the purple wisp with
two eyes: exclude the Incant wordmark, white presentation background, and extra
example logos. Preserve its distinctive silhouette, color and face faithfully.

## Ownership and scope

Use Claude Opus 5.5 through ACP, per CLAUDE.md. You own visual work and rendered
review. Work only in this handoff worktree. Implement the mascot as clean scalable
artwork, with a transparent background. A faithful SVG trace of the supplied
silhouette is appropriate; do not design a different mascot. Do not call external
image-generation services. Keep the UI layout and unrelated symbols unchanged.

Other work on OpenAI login is active in the main checkout and handoff 0003. Do not
edit provider/login UI, shared contracts, Rust, or other handoffs. Allowed paths:
`editor/ui/public/icon.svg`, `editor/ui/public/icon.png`, `editor/ui/brand/`,
`editor/ui/scripts/render-icon.mjs`, `editor/ui/scripts/render-macos-icon.mjs`,
`editor/ui/src/components/Titlebar.tsx`, its logo styling if essential, and this
handoff packet. If another application brand placement is found, record it in
result.md for Astra. Astra handles native asset integration and packaging.

## Existing consumers and acceptance criteria

- Titlebar uses `./icon.svg` at 18×18, favicon uses the same SVG.
- Replace the old spark-and-ring icon in `public/icon.svg` and regenerate
  `public/icon.png`. Use only the wisp, no lettering or old symbol.
- Update `brand/icon-macos.svg` and regenerate `brand/Incant.icns`; preserve macOS
  icon sizing/transparent margins while presenting the same mascot. Prefer the
  purple mascot itself without an added tile, consistent with “just the mascot.”
- The shape must read well at 16, 18, 32, 128 and 512 px on light and dark surfaces.
  Preserve the eye openings faithfully and review the actual titlebar at 1×/2×.
- No external runtime requests, dependencies, animation, or extra font payload.
  A small local SVG and generated raster/app icon files are sufficient.
- Provide browser screenshots before/after and a contact sheet of the new assets.
  Inspect your rendered output and fix cropping, halos, poor size, or distortion.
  Browser fixture evidence must be labelled as such; do not call it native proof.
- Record source provenance and regeneration commands in a short brand README.
  Keep the full reference image in the handoff, not in production bundles.

## Commands and environment

Dependencies are installed in the main checkout. A node_modules symlink is supplied
in this worktree. Chrome is installed at
`/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`.

Run commands from this worktree (never main checkout):

```
npm run icon --workspace editor/ui
npm run icon:macos --workspace editor/ui
npm run build --workspace editor/ui
npm run test --workspace editor/ui
```

Use Playwright/Chrome for your actual rendered review; keep captures scoped to the
Incant browser fixture, not other desktop windows. Native packaging and full
integration checks will be performed by Astra. Do not open or close existing
Incant applications. Do not publish, push, merge, or approve phase gates.

## Return

Write `handoffs/0004-wisp-mascot/result.md` with the exact model, changed paths,
tests, screenshots, visual findings and any limitations. Commit only the scoped
work and packet with `Built-by: claude`. Astra will review and integrate the diff.
