# Result: 0007 landing page neutral palette

## Status

Complete in this worktree. It is committed with `Built-by: claude` for Astra to integrate into
the existing landing PR. Nothing was pushed, published, deployed or merged, and no phase gate is
claimed or approved.

## Director restriction: screen capture stopped

On 2026-10-08, after the design commit `6090702`, the director said: "Stop using computer use
for now or tell cloude to if its using it. Amazon is blocking prime video output due to
screnecapture".

- **Acknowledged.** Effective immediately, this session does not use computer-use tools,
  browser or native automation, screenshots, screen recording or capture helpers.
- **No computer-use tools were used at any point in this packet.** The earlier screenshots came
  from headless Chrome driven by Playwright. Those runs, and the headless violet pixel count,
  all finished before the restriction arrived. They will not be repeated.
- **Further visual verification is deferred at the director's request** until the director
  explicitly lifts the restriction. That covers new renders, visual review and re-running the
  Playwright suite, which takes screenshots.
- **Nothing else changed.** The design and existing evidence stay as committed, and no tests ran
  for this notification. The existing images remain on disk.

## Model

Claude Opus 5.5 (`claude-opus-5-5`), running in Claude Code. No other model and no
image-generation service was used. All artwork remains hand-written SVG and CSS.

## Director feedback acknowledged

The director wrote: "tell claude its a bit too purple. I liked the previous designes color
choices better. Update the app and landing page to match this change. Also logo is too close to
traffic lights and not aligned".

- **Too purple:** applied to the landing page. The editor illustration now uses the earlier
  neutral editor tokens from `references/previous-editor-tokens.css`. The page drops violet
  as a background, hover and tint colour.
- **Update the app:** out of scope here. A separate Claude packet owns the editor itself. The
  shared colour decisions are in `palette-notes.md` for Astra to pass on.
- **Logo spacing near traffic lights:** the native fix belongs to the editor packet. The
  landing-page illustration was not changed for this, because its title bar already leaves
  space between the dots and the mark. A screenshot is at
  `screenshots/after/stage-review-1440.png`.

## What changed

- **Editor chrome:** the title bar, panels, raised controls, hairlines, text, selection pill and
  accent now use the reference token values one to one. The previous values were violet-tinted
  near-blacks such as `#131118` and `#24202c`, plus a lavender accent `#a998ff`. The outline
  column is now a panel surface, so the three connected panels read like the editor's islands.
  The Discard button and the agent input use the 3:1 control border.
- **Scene art:** the dusk sky, ridges, headland, isle, cliff and play-test thumbnails moved from
  purple to cool slate. The warm horizon, moon and lamp glow stay. The selection box and the
  predicted path use the editor accent `#8c95ff`, and the move gizmo uses the editor axis colours.
- **Page:** the full-bleed violet statement band is now ink, with the approved wisp drawn in
  paper. Only the word "permanent." keeps an accent, `#a9b0ff`. The "together." and "sentence."
  emphasis words are plain ink italics. Button hovers go to a soft ink instead of violet.
- **Accent rule:** violet stays only on the wisp logo mark in the header, footer and favicon.
  Interactive accents use one indigo, `#4650c8`. It shares the editor accent's 235° hue and
  appears on the hero emphasis, the active tab, principle numerals, focus rings and selection.
- **Neutrals:** ink, muted text and paper lost their purple and yellow cast. The values are in
  `palette-notes.md`.

The layout, copy, components, interactions and wisp shape are unchanged.

## Changed paths

- `website/src/style.css`: page and editor colour tokens, plus the focus and selection colour.
- `website/src/components/EditorStage.vue`, `StagePanel.vue` and `StageScene.vue`: the editor
  illustration.
- `website/src/components/StatementBand.vue`, `HeroSection.vue`, `PrinciplesSection.vue`,
  `FaqSection.vue` and `SiteFooter.vue`: the page accents.
- `website/index.html`: the theme colour.
- `handoffs/0007-landing-neutral/`: this result, `palette-notes.md` and the screenshots. The
  brief and its references were supplied by Astra and are committed unchanged.

## Commands and results

All commands ran from `website/` on 2026-10-08 against system Chrome.

```
npm ci
npm run build
PAGES_BASE_PATH=/incant/ npm run build
PAGES_BASE_PATH=/incant/ \
  PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
  INCANT_EVIDENCE_DIR=$PWD/../handoffs/0007-landing-neutral/screenshots/after \
  npx playwright test
```

| Check | Result |
|---|---|
| `npm ci` | Pass |
| `npm run build`, root base, strict `vue-tsc` | Pass |
| Build with the `/incant/` base | Pass. Asset URLs are prefixed with `/incant/`. |
| Full Playwright suite on the `/incant/` preview | **22 passed, 0 failed** |
| Same landing spec on the unchanged build, capturing before screenshots | 10 passed |

The 22 tests cover the following:

- Both axe-core suites at 1440 and 390 pass with no violations reported.
- There is no overflow at 1440, 768, 390 and 320, and no third-party requests.
- Ask, Review, Play-test and Undo tabs respond to arrow, Home and End keys.
- The skip link and heading order work.
- The mobile menu works from the keyboard, closes with Escape and has working section links.
- The FAQ disclosures toggle, and reduced motion is honoured.

**Payload:** JS is 35.6 kB gzip, CSS is 7.5 kB gzip, and `dist/` totals 392 kB. Both sizes are
essentially unchanged from revision 2.

**Contrast:** these ratios were computed with the WCAG formula.

| Pair | Ratio |
|---|---|
| Accent `#4650c8` on paper | 5.83 |
| Muted `#5b5e66` on paper | 5.84 |
| Muted on deep paper | 5.25 |
| `#a9b0ff` on panel | 9.06 |
| Editor muted text on panel | 8.28 |

**Violet measurement:** this counts pixels with a hue of 245 to 310° and saturation above 0.25,
measured in headless Chrome. The script lived in a temporary directory outside the repository.

| Render | Before | After |
|---|---|---|
| Desktop 1440, full page | 21.9% | 0.0% |
| Mobile 390, full page | 25.4% | 0.0% |
| Stage, Ask | 49.0% | 0.0% |
| Stage, Review | 42.4% | 0.0% |
| Stage, Play-test | 46.8% | 0.0% |
| Stage, Undo | 49.1% | 0.1% |

The remaining pixels are the brand logo, the agent badge and the indigo selection pill.

## Screenshots

Before and after sets live in `handoffs/0007-landing-neutral/screenshots/`, under `before/`
and `after/`, with the same file names in each:

- `desktop-1440-full.png` and `desktop-1440-fold.png`
- `tablet-768-full.png` and `tablet-768-fold.png`
- `mobile-390-full.png`, `mobile-390-fold.png` and `mobile-390-menu-open.png`
- `narrow-320-full.png` and `narrow-320-fold.png`
- `stage-ask-1440.png`, `stage-review-1440.png`, `stage-play-1440.png` and `stage-undo-1440.png`
- `stage-play-390.png`

The menu-open render also shows the new indigo keyboard focus ring.

## Limitations

- **Browsers:** tested only in Chrome. Firefox, Safari and real devices were not tested, and
  Lighthouse was not run.
- **No visual-regression baseline:** before and after were compared by eye and by the violet
  pixel count above.
- **Reduced motion:** verified by the automated animation check only. No reduced-motion screenshot
  was taken.
- **Editor parity is by token, not pixels:** the illustration copies the reference tokens. It
  has not been compared against a render of the real editor, which is another packet's work.
- **Tablet crop:** this limitation from revision 2 remains. At 768 px the scene's outer edges
  are cropped.

## Open questions

1. Should the logo inside the editor chrome stay periwinkle or use brand violet? This depends on
   what the editor packet chooses for its titlebar mark. See `palette-notes.md`.
2. The reference keeps an indigo-tinted selection pill, `#262b52`. If the editor packet
   neutralises it, the landing page should follow.
