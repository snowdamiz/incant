# Result: 0005 landing page (revision 2)

## Status

Revision 2 is complete in this worktree and ready for Astra to integrate. It responds to the director's rejection of the first design. Not pushed, published or merged. No phase gate is claimed or approved.

The rationale and the critique of the first version are in `revision-2.md`. The revision 1 commit is `7557c10`, and its screenshots remain unchanged in `screenshots/`.

## Model

Claude Opus 5.5 (`claude-opus-5-5`), running in Claude Code with the director's Claude account. No other model and no image-generation service was used. All artwork is hand-written SVG/CSS.

## Director feedback applied

- **"Vibe coded, generic":** the page has a new visual direction, structure, illustration and copy. The rationale is in `revision-2.md`.
- **"Imply finished release":** the copy keeps a confident present-tense voice. It contains no Phase 0, roadmap, beta or internal-tracking language.
- **No fabrication:** there are no customers, quotes, awards, pricing, versions, downloads, signup forms or demos.
- **Genuine destinations:** the calls to action go to the repository and to PLAN.md, presented as the "product guide". Screen-reader text notes that GitHub may require access.

## Changed paths

**Added components** in `website/src/components/`:
- `EditorStage.vue`
- `StageScene.vue`
- `StagePanel.vue`
- `PrinciplesSection.vue`
- `StatementBand.vue`
- `SiteFooter.vue`

**Rewritten:**
- `website/src/style.css`
- `website/src/content.ts`
- `website/src/App.vue`
- `website/src/components/HeroSection.vue`
- `website/src/components/SiteHeader.vue`
- `website/src/components/FaqSection.vue`

**Removed** from `website/src/components/`:
- `ClosingSection.vue`
- `EditorIllustration.vue`
- `FeatureCard.vue`
- `FeatureSection.vue`
- `GameScene.vue`
- `TrustSection.vue`
- `WorkflowSection.vue`
- `WorkflowVisual.vue`

**Updated:**
- `website/index.html`: title, theme colour and light colour scheme.
- `website/public/favicon.svg`: now uses the brand violet.
- `website/tests/landing.spec.ts`: stage-tab test, plus screenshots now go to `screenshots/revision-2/`.

**Dependency:** `website/package.json` and `website/package-lock.json` gain `@fontsource-variable/fraunces` at exactly 5.3.0 (OFL-1.1). The scripts are unchanged. Only the latin "soft" normal and italic files are bundled.

**Handoff files:** `handoffs/0005-landing-page/revision-2.md`, `result.md` and `screenshots/revision-2/*.png`.

**Not touched:** `playwright.config.ts`, `vite.config.ts`, CI and workflow files. Two working-tree changes did not come from this session and are left uncommitted: `website/.gitignore` and the updated `brief.md`.

## Commands and results

These were run from `website/` on 2026-10-08.

| Command | Result |
|---|---|
| `npm run build` | Pass. Strict `vue-tsc` is clean. |
| `PAGES_BASE_PATH=/incant/ npm run build` | Pass. Asset and favicon URLs are prefixed with `/incant/`. |
| `npx playwright test` | **10 passed** in system Chrome against the `/incant/` preview on port 4175. |

The port 4175 server was started by the test runner. One manual preview, PID 90759, was stopped by that exact PID. The user's preview on port 4176, PID 75746, was not touched.

The tests cover:
- No overflow and no external requests at 1440, 768, 390 and 320.
- axe-core at 1440 and 390 with zero serious or critical violations.
- The skip link, heading order and outbound link targets.
- The mobile menu from the keyboard, including Escape and focus return.
- Stage tabs driven by arrow, Home and End keys, which change the panel and its accessible description.
- FAQ disclosure and reduced motion.

Deployed payload:

| Asset | Size |
|---|---|
| JS | 95.9 kB raw, **35.5 kB gzip** (budget 100 kB) |
| CSS | 34.7 kB raw, 7.4 kB gzip |
| Fonts | Fraunces soft 62 kB and italic 78 kB, Inter 48 kB, JetBrains Mono 40 kB |
| Total `dist/` | **368 kB** (target below 2 MB) |

## Screenshots

These are in `handoffs/0005-landing-page/screenshots/revision-2/`:

- `desktop-1440-full.png` and `desktop-1440-fold.png`
- `tablet-768-full.png` and `tablet-768-fold.png`
- `mobile-390-full.png`, `mobile-390-fold.png` and `mobile-390-menu-open.png`
- `narrow-320-full.png` and `narrow-320-fold.png`
- `stage-ask-1440.png`, `stage-review-1440.png`, `stage-play-1440.png` and `stage-undo-1440.png`
- `stage-play-390.png`

## Review findings fixed during this revision

1. **Scene too sparse:** the moon dominated the middle, the wanderer was tiny and the lighthouse beam cut across the isle.
   - The wanderer is now 1.2 times larger and the moon has moved to the upper right.
   - The beam has been replaced by a lamp glow.
   - The landing spot is clear of the anchor post.
2. **Play-test frames:** the onion-skin frames floated off the path, and the rope started where no character was. The frames now sit on the actual swing curve, starting from the cliff.
3. **Lamp glow:** the glow rendered as a dark blob. It is now a warm radial gradient.
4. **Mobile tabs:** the two-row tab grid made the active bar of the second row read as an underline of the first. The tabs are now a single row at every width, with numerals hidden below 520px.
5. **Test locator:** the original locator also matched small presentational SVG icons inside the labelled illustration. It now targets the labelled container.

## Limitations

- **Tablet crop:** at 768px the viewport is taller than the scene's 16:10 ratio, so its far left and right edges are cropped. The character, path and isle stay in frame.
- **Not tested:** Firefox, Safari and real devices. Lighthouse was not run.
- **Mascot sync:** the mascot redraw matches the reference. If the parallel mascot handoff changes the shape, update `WispMark.vue` and `public/favicon.svg`.
- **Private destinations:** both calls to action lead to a private repository, so visitors without access will see a GitHub sign-in page or a 404.
- **Illustrative content:** the editor UI and the scene format excerpt are illustrative, not the shipped product, and both are labelled as conceptual.
- **Tests to adapt:** Astra's independent behavioural tests may need updates for the new labels. Nav items are now How it works, Principles and FAQ. The stage uses the ids `stage-tab-*` and `stage-panel`, replacing the old `tab-*` and `panel-*`.
